// Terminal behavior derived from fzy, copyright (c) 2014 John Hawthorn.
// SPDX-License-Identifier: MIT. See LICENSE.

//! The small POSIX boundary used by the interactive selector.
//!
//! The editor deliberately stores the query as bytes because fzy matches bytes.
//! Cursor movement and deletion only step at UTF-8 code point boundaries, so a
//! terminal user never lands in the middle of a normally encoded character.

#[cfg(target_os = "linux")]
use rustix::{
    event::{self, PollFd, PollFlags, Timespec},
    io::{self as rustix_io, Errno},
};
#[cfg(target_os = "macos")]
use rustix::io::Errno;
use rustix::termios::{self, InputModes, LocalModes, OptionalActions, Termios};
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Read, Write};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::marker::PhantomData;
use std::mem;
#[cfg(target_os = "linux")]
use std::mem::MaybeUninit;
#[cfg(target_os = "linux")]
use std::os::fd::{FromRawFd, OwnedFd};
#[cfg(target_os = "macos")]
use std::os::fd::{FromRawFd, OwnedFd};
#[cfg(target_os = "macos")]
use std::os::unix::thread::JoinHandleExt;
#[cfg(target_os = "macos")]
use std::rc::Rc;
#[cfg(target_os = "macos")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "macos")]
use std::sync::Arc;
#[cfg(target_os = "macos")]
use std::thread::JoinHandle;
use std::path::Path;
#[cfg(target_os = "linux")]
use std::rc::Rc;
use std::time::{Duration, Instant};

#[cfg(not(all(
    any(target_os = "linux", target_os = "macos"),
    any(target_arch = "x86_64", target_arch = "aarch64")
)))]
compile_error!("fz's terminal boundary supports Linux and macOS on x86_64 or aarch64");

#[cfg(target_os = "macos")]
struct ResizeEvents {
    wake_read: OwnedFd,
    wake_write: OwnedFd,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    previous: libc::sigaction,
    original_mask: libc::sigset_t,
    installed: bool,
    // pthread_sigmask is thread-local. Restoring it on another thread would
    // leak the blocked SIGWINCH into whichever thread dropped the terminal.
    _thread_bound: PhantomData<Rc<()>>,
}

#[cfg(target_os = "macos")]
extern "C" fn discard_winch(_: libc::c_int) {}

#[cfg(target_os = "linux")]
struct ResizeEvents {
    signal_fd: OwnedFd,
    // `pthread_sigmask` is thread-local. The thread that installs this
    // descriptor must also restore this exact complete mask.
    original_mask: Option<libc::sigset_t>,
    // `sigset_t` itself is Send, but moving it would restore one thread's
    // mask on another thread. Keep the whole Terminal on its opening thread.
    _thread_bound: PhantomData<Rc<()>>,
}

#[cfg(target_os = "linux")]
impl ResizeEvents {
    fn install(_: &File) -> io::Result<Self> {
        let winch_mask = winch_mask()?;
        // Some libc ABIs write only the kernel-sized prefix of sigset_t. Its
        // all-zero representation is a valid empty mask, so pre-zero unused
        // ABI storage before Rust later treats the full value as initialized.
        let mut original_mask = MaybeUninit::<libc::sigset_t>::zeroed();
        // SAFETY: `winch_mask` is initialized and `original_mask` is writable.
        // pthread_sigmask returns its error number directly rather than errno.
        let mask_result = unsafe {
            libc::pthread_sigmask(libc::SIG_BLOCK, &winch_mask, original_mask.as_mut_ptr())
        };
        if mask_result != 0 {
            return Err(io::Error::from_raw_os_error(mask_result));
        }
        // SAFETY: pthread_sigmask initialized the kernel-sized prefix; the
        // zeroed remainder was already initialized as valid mask storage.
        let original_mask = unsafe { original_mask.assume_init() };

        // SIGWINCH remains blocked for this terminal's lifetime. A resize
        // arriving while this descriptor is created or while poll starts is
        // pending, so it makes this descriptor readable instead of racing a
        // handler's check-before-poll window.
        // SAFETY: -1 requests a new descriptor, `winch_mask` is initialized,
        // and these are valid Linux signalfd flags.
        let raw_fd = unsafe {
            libc::signalfd(
                -1,
                &winch_mask,
                libc::SFD_CLOEXEC | libc::SFD_NONBLOCK,
            )
        };
        if raw_fd == -1 {
            let signal_error = io::Error::last_os_error();
            return match restore_signal_mask(&original_mask) {
                Ok(()) => Err(signal_error),
                Err(restore_error) => Err(restore_error),
            };
        }

        // SAFETY: signalfd returned a newly owned nonnegative descriptor.
        let signal_fd = unsafe { OwnedFd::from_raw_fd(raw_fd) };
        Ok(Self {
            signal_fd,
            original_mask: Some(original_mask),
            _thread_bound: PhantomData,
        })
    }

    fn wait_for_input(&self, input: &File, timeout: Option<Duration>) -> io::Result<Wait> {
        let timeout = timeout.map(|timeout| {
            Timespec::try_from(timeout).expect("the fixed escape timeout fits in a timespec")
        });
        let mut descriptors = [
            PollFd::new(input, PollFlags::IN),
            PollFd::new(&self.signal_fd, PollFlags::IN),
        ];
        match event::poll(&mut descriptors, timeout.as_ref()) {
            Ok(0) => Ok(Wait::TimedOut),
            Ok(_) if descriptors[1].revents().contains(PollFlags::IN) => {
                self.drain_signals()?;
                // Input may be ready too. Redraw first; it stays readable for
                // the next iteration, which preserves the resize transition.
                Ok(Wait::Interrupted)
            }
            Ok(_) if descriptors[1].revents().is_empty() => Ok(Wait::Ready),
            Ok(_) => Err(io::Error::other("signalfd stopped reporting SIGWINCH")),
            // Other signals can still interrupt poll. Redraw and retry.
            Err(Errno::INTR) => Ok(Wait::Interrupted),
            Err(error) => Err(error.into()),
        }
    }

    fn drain_signals(&self) -> io::Result<()> {
        // signalfd requires a buffer at least as large as this ABI-owned
        // record. Its payload is intentionally unused: this descriptor only
        // accepts SIGWINCH, and every queued record means redraw.
        let mut record = [0_u8; mem::size_of::<libc::signalfd_siginfo>()];
        loop {
            match rustix_io::read(&self.signal_fd, &mut record) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "signalfd reached EOF",
                    ));
                }
                Ok(bytes) if bytes == record.len() => {}
                Ok(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "signalfd returned a partial signal record",
                    ));
                }
                Err(Errno::INTR) => continue,
                Err(Errno::AGAIN) => return Ok(()),
                Err(error) => return Err(error.into()),
            }
        }
    }

    fn restore(&mut self) -> io::Result<()> {
        let Some(original_mask) = self.original_mask.take() else {
            return Ok(());
        };
        if let Err(error) = restore_signal_mask(&original_mask) {
            // Retain it so Terminal::drop can retry after an explicit restore
            // failure. The descriptor stays open until this field drops.
            self.original_mask = Some(original_mask);
            return Err(error);
        }
        Ok(())
    }

    fn input_pending(&self, input: &File) -> io::Result<bool> {
        let timeout = Timespec::try_from(Duration::ZERO).expect("a zero timeout fits in a timespec");
        let mut descriptors = [PollFd::new(input, PollFlags::IN)];
        match event::poll(&mut descriptors, Some(&timeout)) {
            Ok(0) => Ok(false),
            Ok(_) => Ok(descriptors[0].revents().contains(PollFlags::IN)),
            Err(Errno::INTR) => Ok(false),
            Err(error) => Err(error.into()),
        }
    }
}

#[cfg(target_os = "linux")]
impl Drop for ResizeEvents {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

#[cfg(target_os = "linux")]
fn winch_mask() -> io::Result<libc::sigset_t> {
    // See install: a zero mask is valid and initializes ABI storage libc may
    // not touch outside the kernel-sized signal-set prefix.
    let mut mask = MaybeUninit::<libc::sigset_t>::zeroed();
    // SAFETY: the libc functions initialize the supplied writable sigset_t,
    // and SIGWINCH is a valid Linux signal number.
    if unsafe { libc::sigemptyset(mask.as_mut_ptr()) } == -1
        || unsafe { libc::sigaddset(mask.as_mut_ptr(), libc::SIGWINCH) } == -1
    {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: zeroed storage was already initialized, and the libc calls set
    // the mask's required kernel-visible bits.
    Ok(unsafe { mask.assume_init() })
}

#[cfg(target_os = "linux")]
fn restore_signal_mask(mask: &libc::sigset_t) -> io::Result<()> {
    // SAFETY: `mask` was captured by pthread_sigmask for this same thread and
    // restores its complete caller-owned signal mask.
    let result = unsafe { libc::pthread_sigmask(libc::SIG_SETMASK, mask, std::ptr::null_mut()) };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(result))
    }
}

#[cfg(target_os = "macos")]
fn configure_fd(fd: libc::c_int) -> io::Result<()> {
    // SAFETY: `fd` is an open descriptor owned by the caller.
    let fd_flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    let status = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if fd_flags == -1
        || status == -1
        || unsafe { libc::fcntl(fd, libc::F_SETFD, fd_flags | libc::FD_CLOEXEC) } == -1
        || unsafe { libc::fcntl(fd, libc::F_SETFL, status | libc::O_NONBLOCK) } == -1
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn winch_mask() -> io::Result<libc::sigset_t> {
    let mut mask: libc::sigset_t = unsafe { mem::zeroed() };
    if unsafe { libc::sigemptyset(&mut mask) } == -1
        || unsafe { libc::sigaddset(&mut mask, libc::SIGWINCH) } == -1
    {
        return Err(io::Error::last_os_error());
    }
    Ok(mask)
}

#[cfg(target_os = "macos")]
fn serve_winch(write_fd: libc::c_int, stop: Arc<AtomicBool>) {
    let Ok(mask) = winch_mask() else {
        unsafe { libc::close(write_fd) };
        return;
    };
    while !stop.load(Ordering::Acquire) {
        let mut signal = 0;
        // SAFETY: `mask` contains SIGWINCH, which is blocked in this thread.
        // sigwait dequeues it without running a handler.
        let result = unsafe { libc::sigwait(&mask, &mut signal) };
        if result != 0 || stop.load(Ordering::Acquire) {
            break;
        }
        let token = [1u8];
        unsafe {
            libc::write(write_fd, token.as_ptr().cast(), 1);
        }
    }
    unsafe { libc::close(write_fd) };
}

#[cfg(target_os = "macos")]
impl ResizeEvents {
    fn install(_: &File) -> io::Result<Self> {
        // A SIGWINCH handler that writes while the main thread is inside a tty
        // operation can wedge the process so the bare-Escape wait never
        // returns. SIGWINCH stays blocked. Its default disposition is ignore,
        // which would discard it before sigwait, so a no-op handler marks it
        // catchable and the blocked mask keeps that handler from running.
        let mut pipes = [0; 2];
        // SAFETY: `pipes` is writable storage for two descriptors. On failure
        // neither entry is owned.
        if unsafe { libc::pipe(pipes.as_mut_ptr()) } == -1 {
            return Err(io::Error::last_os_error());
        }
        if let Err(error) = configure_fd(pipes[0]).and_then(|()| configure_fd(pipes[1])) {
            unsafe {
                libc::close(pipes[0]);
                libc::close(pipes[1]);
            }
            return Err(error);
        }
        // SAFETY: `pipes[1]` is the open write end. The duplicate is owned by
        // the sigwait thread, which closes it when that thread exits.
        let thread_write = unsafe { libc::dup(pipes[1]) };
        if thread_write == -1 {
            let error = io::Error::last_os_error();
            unsafe {
                libc::close(pipes[0]);
                libc::close(pipes[1]);
            }
            return Err(error);
        }
        if let Err(error) = configure_fd(thread_write) {
            unsafe {
                libc::close(pipes[0]);
                libc::close(pipes[1]);
                libc::close(thread_write);
            }
            return Err(error);
        }
        // SAFETY: pipe returned two owned descriptors, and both are still open.
        let wake_read = unsafe { OwnedFd::from_raw_fd(pipes[0]) };
        let wake_write = unsafe { OwnedFd::from_raw_fd(pipes[1]) };

        let mask = match winch_mask() {
            Ok(mask) => mask,
            Err(error) => {
                unsafe { libc::close(thread_write) };
                return Err(error);
            }
        };
        let mut original_mask: libc::sigset_t = unsafe { mem::zeroed() };
        // SAFETY: `mask` is initialized. pthread_sigmask writes the previous
        // mask and returns its error number instead of setting errno.
        let mask_result = unsafe {
            libc::pthread_sigmask(libc::SIG_BLOCK, &mask, &mut original_mask)
        };
        if mask_result != 0 {
            unsafe { libc::close(thread_write) };
            return Err(io::Error::from_raw_os_error(mask_result));
        }

        // SAFETY: both sigaction values are zeroed. `discard_winch` is an
        // `extern "C"` function and is not invoked while SIGWINCH stays blocked.
        let mut action: libc::sigaction = unsafe { mem::zeroed() };
        action.sa_sigaction = discard_winch as *const () as libc::sighandler_t;
        let mut previous: libc::sigaction = unsafe { mem::zeroed() };
        if unsafe { libc::sigemptyset(&mut action.sa_mask) } == -1
            || unsafe { libc::sigaction(libc::SIGWINCH, &action, &mut previous) } == -1
        {
            let error = io::Error::last_os_error();
            unsafe {
                libc::pthread_sigmask(libc::SIG_SETMASK, &original_mask, std::ptr::null_mut());
                libc::close(thread_write);
            }
            return Err(error);
        }

        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let thread = match std::thread::Builder::new()
            .name("fz-sigwinch".to_string())
            .spawn(move || serve_winch(thread_write, thread_stop))
        {
            Ok(thread) => thread,
            Err(error) => {
                unsafe {
                    libc::sigaction(libc::SIGWINCH, &previous, std::ptr::null_mut());
                    libc::pthread_sigmask(libc::SIG_SETMASK, &original_mask, std::ptr::null_mut());
                    libc::close(thread_write);
                }
                return Err(error);
            }
        };
        Ok(Self {
            wake_read,
            wake_write,
            stop,
            thread: Some(thread),
            previous,
            original_mask,
            installed: true,
            _thread_bound: PhantomData,
        })
    }

    fn wait_for_input(&self, input: &File, timeout: Option<Duration>) -> io::Result<Wait> {
        // macOS poll reports POLLNVAL for /dev/tty, the default terminal
        // device, so wait with select, which handles it.
        use rustix::event::{FdSetElement, Timespec, fd_set_insert, fd_set_num_elements, select, FdSetIter};
        use std::os::fd::{AsFd, AsRawFd};
        let wake = self.wake_read.as_fd().as_raw_fd();
        let tty = input.as_fd().as_raw_fd();
        let nfds = wake.max(tty) + 1;
        let timeout = timeout.map(|timeout| {
            Timespec::try_from(timeout).expect("the fixed escape timeout fits in a timespec")
        });
        let mut readfds = vec![FdSetElement::default(); fd_set_num_elements(2, nfds)];
        fd_set_insert(&mut readfds, wake);
        fd_set_insert(&mut readfds, tty);
        // SAFETY: both descriptors are owned by this terminal and stay open
        // for the call; the set fits every inserted descriptor.
        match unsafe { select(nfds, Some(&mut readfds), None, None, timeout.as_ref()) } {
            Ok(0) => Ok(Wait::TimedOut),
            Ok(_) => {
                let mut wake_ready = false;
                let mut tty_ready = false;
                for fd in FdSetIter::new(&readfds) {
                    if fd == wake {
                        wake_ready = true;
                    } else if fd == tty {
                        tty_ready = true;
                    }
                }
                if wake_ready {
                    self.drain_wake()?;
                    // Input may be ready too. Redraw first; it stays readable
                    // for the next iteration, which preserves the resize transition.
                    Ok(Wait::Interrupted)
                } else if tty_ready {
                    Ok(Wait::Ready)
                } else {
                    Ok(Wait::TimedOut)
                }
            }
            Err(Errno::INTR) => Ok(Wait::Interrupted),
            Err(error) => Err(error.into()),
        }
    }

    fn drain_wake(&self) -> io::Result<()> {
        let mut buf = [0u8; 64];
        loop {
            match rustix::io::read(&self.wake_read, &mut buf) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "resize wake pipe closed",
                    ));
                }
                Ok(_) => {}
                Err(Errno::INTR) => continue,
                Err(Errno::AGAIN) => return Ok(()),
                Err(error) => return Err(error.into()),
            }
        }
    }

    fn input_pending(&self, input: &File) -> io::Result<bool> {
        use rustix::event::{FdSetElement, Timespec, fd_set_insert, fd_set_num_elements, select};
        use std::os::fd::{AsFd, AsRawFd};
        let tty = input.as_fd().as_raw_fd();
        let nfds = tty + 1;
        let timeout = Timespec::try_from(Duration::ZERO).expect("a zero timeout fits in a timespec");
        let mut readfds = vec![FdSetElement::default(); fd_set_num_elements(1, nfds)];
        fd_set_insert(&mut readfds, tty);
        // SAFETY: `tty` stays open for the call, and the set covers that descriptor.
        match unsafe { select(nfds, Some(&mut readfds), None, None, Some(&timeout)) } {
            Ok(0) => Ok(false),
            Ok(_) => Ok(true),
            Err(Errno::INTR) => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    fn restore(&mut self) -> io::Result<()> {
        if !self.installed {
            return Ok(());
        }
        // The write end stays open until the sigwait thread has exited.
        let _ = &self.wake_write;
        self.stop.store(true, Ordering::Release);
        let mut join_failed = false;
        if let Some(thread) = self.thread.take() {
            // SAFETY: the thread is blocked in sigwait, or about to enter it,
            // with SIGWINCH blocked, so this directed signal unblocks sigwait.
            let kill_result = unsafe { libc::pthread_kill(thread.as_pthread_t(), libc::SIGWINCH) };
            join_failed = thread.join().is_err();
            if kill_result != 0 && kill_result != libc::ESRCH {
                let _ = unsafe {
                    libc::sigaction(libc::SIGWINCH, &self.previous, std::ptr::null_mut())
                };
                let _ = unsafe {
                    libc::pthread_sigmask(
                        libc::SIG_SETMASK,
                        &self.original_mask,
                        std::ptr::null_mut(),
                    )
                };
                self.installed = false;
                return Err(io::Error::from_raw_os_error(kill_result));
            }
        }
        // SAFETY: `previous` was captured from the successful install, and a
        // null old-action pointer only installs that action. The saved mask
        // belongs to this thread.
        if unsafe { libc::sigaction(libc::SIGWINCH, &self.previous, std::ptr::null_mut()) } == -1 {
            return Err(io::Error::last_os_error());
        }
        let mask_result = unsafe {
            libc::pthread_sigmask(libc::SIG_SETMASK, &self.original_mask, std::ptr::null_mut())
        };
        if mask_result != 0 {
            return Err(io::Error::from_raw_os_error(mask_result));
        }
        self.installed = false;
        if join_failed {
            return Err(io::Error::other("resize thread panicked"));
        }
        Ok(())
    }
}

#[cfg(target_os = "macos")]
impl Drop for ResizeEvents {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

/// A raw-mode terminal. Dropping it always puts its saved termios state back.
pub struct Terminal {
    resize_events: ResizeEvents,
    input: File,
    output: BufWriter<File>,
    original: Termios,
    restore_needed: bool,
    foreground: u8,
}

impl Terminal {
    /// Opens `path` for input and output, then switches input to fzy's raw mode.
    pub fn open(path: &Path) -> io::Result<Self> {
        let input = OpenOptions::new().read(true).open(path)?;
        let output = OpenOptions::new().write(true).open(path)?;
        // select on a macOS tty can report readability when a following read
        // would block. A blocking read there never returns, so the descriptor
        // stays nonblocking and a false ready result goes back to the wait.
        #[cfg(target_os = "macos")]
        {
            use std::os::fd::AsRawFd;
            let fd = input.as_raw_fd();
            // SAFETY: `fd` is the descriptor just opened for this terminal.
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
            if flags == -1
                || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1
            {
                return Err(io::Error::last_os_error());
            }
        }

        let original = termios::tcgetattr(&input)?;
        let mut raw = original.clone();
        raw.input_modes.remove(InputModes::ICRNL);
        raw.local_modes
            .remove(LocalModes::ICANON | LocalModes::ECHO | LocalModes::ISIG);
        termios::tcsetattr(&input, OptionalActions::Now, &raw)?;

        let resize_events = match ResizeEvents::install(&input) {
            Ok(resize_events) => resize_events,
            Err(error) => {
                let _ = termios::tcsetattr(&input, OptionalActions::Now, &original);
                return Err(error);
            }
        };

        let mut terminal = Self {
            resize_events,
            input,
            output: BufWriter::with_capacity(16 * 1024, output),
            original,
            restore_needed: true,
            foreground: 9,
        };
        terminal.set_normal()?;
        terminal.flush()?;
        Ok(terminal)
    }

    /// Runs the interactive selector and returns a chosen candidate or query.
    ///
    /// `None` represents an explicit cancellation. Returned bytes do not have a
    /// trailing newline; the caller owns stdout and adds the record delimiter.
    pub fn run(
        &mut self,
        candidates: fz::Candidates,
        options: &crate::Options,
    ) -> io::Result<Option<Vec<u8>>> {
        if !self.restore_needed {
            return Err(io::Error::other("terminal has already been restored"));
        }

        let (_, height) = self.dimensions();
        let show_info = options.show_info || options.multi;
        let reserved_rows = 1usize + usize::from(show_info);
        let lines = options
            .lines
            .min(height.saturating_sub(reserved_rows));
        if options.multi && lines == 0 && !candidates.is_empty() {
            return Err(io::Error::other("terminal too small for multi-select results"));
        }
        let mut state = Editor::new(options.query.clone(), lines, options.multi);
        let mut choices = fz::Choices::from_candidates(candidates);
        choices.set_workers(options.workers);
        // On Linux, `Terminal::open` already blocked SIGWINCH. Any search
        // workers spawned here inherit that mask, keeping resize delivery on
        // this thread's signalfd for the whole interactive session.
        choices.search(&state.query);

        self.draw(&mut choices, options, &state)?;
        let mut input_ready = false;
        // The 25ms bare-Escape window is wall time from the Escape byte. A
        // resize wake must not start that window over, or a stream of
        // SIGWINCH keeps the selector up until the process is killed.
        let mut escape_deadline: Option<Instant> = None;
        loop {
            // Only a bare Escape is ambiguous. Other partial key sequences
            // wait for their next byte, exactly like fzy's keybinding parser.
            if state.ambiguous_key_pending {
                escape_deadline.get_or_insert_with(|| Instant::now() + Duration::from_millis(25));
            } else {
                escape_deadline = None;
            }
            let timeout = escape_deadline.map(|deadline| deadline.saturating_duration_since(Instant::now()));
            let mut ready = if input_ready {
                Wait::Ready
            } else if timeout == Some(Duration::ZERO) {
                Wait::TimedOut
            } else {
                self.wait_for_input(timeout)?
            };
            input_ready = false;
            if matches!(ready, Wait::Interrupted) {
                self.draw(&mut choices, options, &state)?;
                // A resize wake leaves queued input readable. Consuming it on
                // the next iteration never happens while wakes keep arriving,
                // so a SIGWINCH stream would swallow the Escape that starts
                // the deadline.
                if self.resize_events.input_pending(&self.input)? {
                    ready = Wait::Ready;
                } else if escape_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                    ready = Wait::TimedOut;
                } else {
                    continue;
                }
            }
            let byte = match ready {
                Wait::Ready => match self.read_byte() {
                    Ok(Some(byte)) => Some(byte),
                    Ok(None) => return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "terminal input closed")),
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => continue,
                    Err(error) => return Err(error),
                },
                Wait::TimedOut | Wait::Interrupted => None,
            };
            match self.handle_input(&mut state, &mut choices, byte, byte.is_none()) {
                InputResult::Continue { search_changed } => {
                    state.search_dirty |= search_changed;
                }
                InputResult::Accept => return self.finish(&mut state, &mut choices, show_info),
                InputResult::Cancel => return self.cancel(&state, show_info),
            }
            // Drain already queued typing before ranking and drawing. fzy also
            // searches once per input batch; actions that consume the selection
            // synchronize immediately in apply_action instead.
            if matches!(self.wait_for_input(Some(Duration::ZERO))?, Wait::Ready) {
                input_ready = true;
                continue;
            }
            state.update_choices(&mut choices);
            self.draw(&mut choices, options, &state)?;
        }
    }

    fn finish(
        &mut self,
        state: &mut Editor,
        choices: &mut fz::Choices,
        show_info: bool,
    ) -> io::Result<Option<Vec<u8>>> {
        let selection = if state.multi && !state.marked.is_empty() {
            let mut output = Vec::new();
            for (index, &source) in state.marked.iter().enumerate() {
                if index > 0 { output.push(b'\n'); }
                output.extend_from_slice(choices.source(source).expect("marked source exists"));
            }
            Some(output)
        } else if state.multi && choices.available() == 0 {
            // Multi-mode never echoes an unmatched query as a selection.
            None
        } else {
            Some(choices.selected().map(ToOwned::to_owned)
                .unwrap_or_else(|| mem::take(&mut state.query)))
        };
        self.clear(state.lines, show_info)?;
        self.restore()?;
        Ok(selection)
    }

    fn cancel(&mut self, state: &Editor, show_info: bool) -> io::Result<Option<Vec<u8>>> {
        self.clear(state.lines, show_info)?;
        self.restore()?;
        Ok(None)
    }

    fn handle_input(
        &mut self,
        state: &mut Editor,
        choices: &mut fz::Choices,
        byte: Option<u8>,
        resolve_ambiguous_key: bool,
    ) -> InputResult {
        state.ambiguous_key_pending = false;
        if let Some(byte) = byte {
            state.input.push(byte);
        }

        // Space only marks a candidate after Down has moved focus into the list.
        // In the query, it remains an ordinary search character.
        if state.multi && state.focus_list && state.input == b" " {
            state.input.clear();
            state.update_choices(choices);
            if let Some(source) = choices.source_index(choices.selection()) {
                if let Some(index) = state.marked.iter().position(|&id| id == source) {
                    state.marked.remove(index);
                } else {
                    state.marked.push(source);
                }
            }
            return InputResult::Continue { search_changed: false };
        }

        let mut matching_action = None;
        let mut in_middle = false;
        for &(key, action) in KEY_BINDINGS {
            if state.input == key {
                matching_action = Some(action);
            } else if key.starts_with(&state.input) {
                in_middle = true;
            }
        }

        if let Some(action) = matching_action
            && (!in_middle || resolve_ambiguous_key) {
                state.input.clear();
                return self.apply_action(action, state, choices);
            }

        if matching_action.is_some() && in_middle {
            state.ambiguous_key_pending = true;
            return InputResult::Continue {
                search_changed: false,
            };
        }

        if in_middle {
            return InputResult::Continue {
                search_changed: false,
            };
        }

        // Typing while navigating results returns to query editing; previously
        // marked input identities survive re-filtering.
        if state.multi && state.focus_list {
            state.focus_list = false;
        }
        let mut search_changed = false;
        for index in 0..state.input.len() {
            let byte = state.input[index];
            if is_printable(byte) {
                search_changed |= state.insert(byte);
            }
        }
        state.input.clear();
        InputResult::Continue { search_changed }
    }

    fn apply_action(
        &mut self,
        action: Action,
        state: &mut Editor,
        choices: &mut fz::Choices,
    ) -> InputResult {
        if matches!(action, Action::Accept | Action::Complete | Action::Previous | Action::Next | Action::PageUp | Action::PageDown) {
            state.update_choices(choices);
        }
        let changed = match action {
            Action::Cancel => return InputResult::Cancel,
            Action::Accept => return InputResult::Accept,
            Action::DeleteCharacter => { state.focus_list = false; state.delete_character() }
            Action::DeleteWord => { state.focus_list = false; state.delete_word() }
            Action::DeleteToBeginning => { state.focus_list = false; state.delete_to_beginning() }
            Action::Complete => {
                state.focus_list = false;
                choices.selected().is_some_and(|candidate| state.replace(candidate))
            }
            Action::Previous => {
                if state.multi {
                    if state.focus_list {
                        if choices.selection() == 0 { state.focus_list = false; }
                        else { choices.prev(); }
                    }
                } else { choices.prev(); }
                false
            }
            Action::Next => {
                if state.multi && !state.focus_list {
                    state.focus_list = choices.available() > 0;
                } else { choices.next(); }
                false
            }
            Action::Left => { state.focus_list = false; state.left(); false }
            Action::Right => { state.focus_list = false; state.right(); false }
            Action::Beginning => { state.focus_list = false; state.cursor = 0; false }
            Action::End => { state.focus_list = false; state.cursor = state.query.len(); false }
            Action::PageUp => {
                if !state.multi || state.focus_list {
                    for _ in 0..state.lines.min(choices.selection()) { choices.prev(); }
                    if state.multi && choices.selection() == 0 { state.focus_list = false; }
                }
                false
            }
            Action::PageDown => {
                if state.multi && !state.focus_list {
                    state.focus_list = choices.available() > 0;
                } else {
                    let remaining = choices.available().saturating_sub(choices.selection() + 1);
                    for _ in 0..state.lines.min(remaining) { choices.next(); }
                }
                false
            }
            Action::Ignore => false,
        };
        InputResult::Continue { search_changed: changed }
    }

    fn draw(
        &mut self,
        choices: &mut fz::Choices,
        options: &crate::Options,
        state: &Editor,
    ) -> io::Result<()> {
        let available = choices.available();
        let selection = choices.selection();
        let mut start = 0usize;
        if state.lines > 0 && selection.saturating_add(1) >= state.lines {
            start = selection.saturating_add(2).saturating_sub(state.lines);
            if start.saturating_add(state.lines) >= available && available > 0 {
                start = available.saturating_sub(state.lines);
            }
        }

        self.set_column(0)?;
        self.write_bytes(&options.prompt)?;
        self.write_bytes(&state.query)?;
        self.clear_line()?;

        if options.show_info || options.multi {
            self.write_bytes(b"\n[")?;
            self.write_usize(available)?;
            self.write_bytes(b"/")?;
            self.write_usize(choices.len())?;
            self.write_bytes(b"]")?;
            if options.multi {
                self.write_bytes(b" [")?;
                self.write_usize(state.marked.len())?;
                self.write_bytes(b" selected]")?;
            }
            self.clear_line()?;
        }

        let width = self.dimensions().0;
        let budget = state.lines;
        let prefix_width = usize::from(options.multi) * 4 + usize::from(options.show_scores) * 8;
        let mut end = start.saturating_add(budget).min(available);
        let mut row_counts: Vec<usize> = (start..end)
            .map(|index| choices.get(index).map_or(1, |candidate| wrapped_rows(candidate, prefix_width, width)))
            .collect();
        let mut total_rows: usize = row_counts.iter().sum();
        while total_rows > budget && start < selection && end > start {
            total_rows -= row_counts.remove(0);
            start += 1;
        }
        while total_rows > budget && end > selection.saturating_add(1) && !row_counts.is_empty() {
            total_rows -= row_counts.pop().unwrap_or(0);
            end -= 1;
        }
        let mut rows_used = 0usize;
        for index in start..end {
            self.write_bytes(b"\n")?;
            self.clear_line()?;
            let score = options.show_scores.then(|| choices.getscore(index)).flatten();
            let marked = options.multi && choices.source_index(index)
                .is_some_and(|source| state.marked.contains(&source));
            if let Some(candidate) = choices.get(index) {
                let remaining = budget.saturating_sub(rows_used).max(1);
                rows_used += self.draw_candidate(candidate, state, CandidateRow {
                    score, width, max_rows: remaining,
                    selected: index == selection && (!options.multi || state.focus_list),
                    multi: options.multi, marked,
                })?;
            }
        }
        for _ in rows_used..budget {
            self.write_bytes(b"\n")?;
            self.clear_line()?;
        }

        let result_rows = state.lines + usize::from(options.show_info || options.multi);
        if result_rows > 0 {
            self.move_up(result_rows)?;
        }
        self.set_column(0)?;
        self.write_bytes(&options.prompt)?;
        self.write_bytes(&state.query[..state.cursor])?;
        self.flush()
    }

    fn draw_candidate(
        &mut self,
        candidate: &[u8],
        state: &Editor,
        row: CandidateRow,
    ) -> io::Result<usize> {
        let CandidateRow { score, width, max_rows, selected, multi, marked } = row;
        if selected { self.set_invert()?; }
        if multi {
            self.write_bytes(if marked { b"[x] " } else { b"[ ] " })?;
        }
        if let Some(score) = score {
            if score == f64::NEG_INFINITY {
                self.write_bytes(b"(     ) ")?;
            } else {
                write!(self.output, "({score:5.2}) ")?;
            }
        }

        let prefix_width = usize::from(multi) * 4 + usize::from(score.is_some()) * 8;
        if width <= prefix_width {
            self.set_nowrap()?;
            let positions = fz::match_positions(&state.query, candidate).unwrap_or_default();
            let mut position = 0usize;
            for (index, &byte) in candidate.iter().enumerate() {
                if positions.get(position) == Some(&index) {
                    self.set_foreground(3)?;
                    position += 1;
                } else {
                    self.set_foreground(9)?;
                }
                if byte == b'\n' {
                    self.write_bytes(b" ")?;
                } else {
                    self.output.write_all(&candidate[index..=index])?;
                }
            }
            self.set_wrap()?;
            self.set_normal()?;
            return Ok(1);
        }

        let positions = fz::match_positions(&state.query, candidate).unwrap_or_default();
        let mut position = 0usize;
        let mut column = prefix_width;
        let mut rows = 1usize;
        let mut index = 0usize;
        while index < candidate.len() {
            let (scalar, length) = decode_char(&candidate[index..]);
            let char_width = if candidate[index] == b'\n' { 1 } else { scalar_width(scalar) };
            if char_width > 0 && column + char_width > width {
                if rows >= max_rows { break; }
                self.write_bytes(b"\n")?;
                self.set_column(0)?;
                self.clear_line()?;
                rows += 1;
                column = 0;
            }
            if positions.get(position) == Some(&index) {
                self.set_foreground(3)?;
                position += 1;
            } else {
                self.set_foreground(9)?;
            }
            if candidate[index] == b'\n' {
                self.write_bytes(b" ")?;
            } else {
                self.output.write_all(&candidate[index..index + length])?;
            }
            column += char_width;
            index += length;
        }

        self.set_normal()?;
        Ok(rows)
    }

    fn clear(&mut self, lines: usize, show_info: bool) -> io::Result<()> {
        self.set_column(0)?;
        for _ in 0..lines + usize::from(show_info) {
            self.newline()?;
        }
        self.clear_line()?;
        if lines > 0 {
            self.move_up(lines + usize::from(show_info))?;
        }
        self.flush()
    }

    fn dimensions(&self) -> (usize, usize) {
        let Ok(size) = termios::tcgetwinsize(self.output.get_ref()) else {
            return (80, 25);
        };
        if size.ws_col == 0 || size.ws_row == 0 {
            return (80, 25);
        }
        (usize::from(size.ws_col), usize::from(size.ws_row))
    }

    fn wait_for_input(&self, timeout: Option<Duration>) -> io::Result<Wait> {
        self.resize_events.wait_for_input(&self.input, timeout)
    }

    fn read_byte(&mut self) -> io::Result<Option<u8>> {
        let mut byte = [0u8; 1];
        loop {
            match self.input.read(&mut byte) {
                Ok(0) => return Ok(None),
                Ok(_) => return Ok(Some(byte[0])),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
        }
    }

    fn restore(&mut self) -> io::Result<()> {
        let termios_result = if self.restore_needed {
            loop {
                match termios::tcsetattr(&self.input, OptionalActions::Now, &self.original) {
                    Ok(()) => {
                        self.restore_needed = false;
                        break Ok(());
                    }
                    Err(error) => {
                        let error = io::Error::from(error);
                        if error.kind() == io::ErrorKind::Interrupted {
                            continue;
                        }
                        break Err(error);
                    }
                }
            }
        } else {
            Ok(())
        };
        let resize_result = self.resize_events.restore();
        termios_result.and(resize_result)
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.output.write_all(bytes)
    }

    fn write_usize(&mut self, value: usize) -> io::Result<()> {
        write!(self.output, "{value}")
    }

    fn flush(&mut self) -> io::Result<()> {
        self.output.flush()
    }

    fn set_foreground(&mut self, color: u8) -> io::Result<()> {
        if self.foreground != color {
            write!(self.output, "\x1b[{}m", 30 + color)?;
            self.foreground = color;
        }
        Ok(())
    }

    fn set_invert(&mut self) -> io::Result<()> {
        self.write_bytes(b"\x1b[7m")
    }

    fn set_normal(&mut self) -> io::Result<()> {
        self.write_bytes(b"\x1b[0m")?;
        self.foreground = 9;
        Ok(())
    }

    fn set_nowrap(&mut self) -> io::Result<()> {
        self.write_bytes(b"\x1b[?7l")
    }

    fn set_wrap(&mut self) -> io::Result<()> {
        self.write_bytes(b"\x1b[?7h")
    }

    fn clear_line(&mut self) -> io::Result<()> {
        self.write_bytes(b"\x1b[K")
    }

    fn newline(&mut self) -> io::Result<()> {
        self.write_bytes(b"\x1b[K\n")
    }

    fn set_column(&mut self, column: usize) -> io::Result<()> {
        write!(self.output, "\x1b[{}G", column + 1)
    }

    fn move_up(&mut self, rows: usize) -> io::Result<()> {
        write!(self.output, "\x1b[{}A", rows)
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

#[derive(Clone, Copy)]
enum Action {
    Cancel,
    Accept,
    DeleteCharacter,
    DeleteWord,
    DeleteToBeginning,
    Complete,
    Previous,
    Next,
    Left,
    Right,
    Beginning,
    End,
    PageUp,
    PageDown,
    Ignore,
}

const KEY_BINDINGS: &[(&[u8], Action)] = &[
    (b"\x1b", Action::Cancel),
    (b"\x7f", Action::DeleteCharacter),
    (b"\x08", Action::DeleteCharacter),
    (b"\x17", Action::DeleteWord),
    (b"\x15", Action::DeleteToBeginning),
    (b"\x09", Action::Complete),
    (b"\x03", Action::Cancel),
    (b"\x04", Action::Cancel),
    (b"\x07", Action::Cancel),
    (b"\x0d", Action::Accept),
    (b"\x10", Action::Previous),
    (b"\x0e", Action::Next),
    (b"\x0b", Action::Previous),
    (b"\x0a", Action::Next),
    (b"\x01", Action::Beginning),
    (b"\x05", Action::End),
    (b"\x1bOD", Action::Left),
    (b"\x1b[D", Action::Left),
    (b"\x1bOC", Action::Right),
    (b"\x1b[C", Action::Right),
    (b"\x1b[1~", Action::Beginning),
    (b"\x1b[H", Action::Beginning),
    (b"\x1b[4~", Action::End),
    (b"\x1b[F", Action::End),
    (b"\x1b[A", Action::Previous),
    (b"\x1bOA", Action::Previous),
    (b"\x1b[B", Action::Next),
    (b"\x1bOB", Action::Next),
    (b"\x1b[5~", Action::PageUp),
    (b"\x1b[6~", Action::PageDown),
    (b"\x1b[200~", Action::Ignore),
    (b"\x1b[201~", Action::Ignore),
];

enum Wait {
    Ready,
    TimedOut,
    Interrupted,
}

enum InputResult {
    Continue { search_changed: bool },
    Accept,
    Cancel,
}

struct CandidateRow {
    score: Option<f64>,
    width: usize,
    max_rows: usize,
    selected: bool,
    multi: bool,
    marked: bool,
}

struct Editor {
    query: Vec<u8>,
    cursor: usize,
    multi: bool,
    focus_list: bool,
    marked: Vec<usize>,
    input: Vec<u8>,
    ambiguous_key_pending: bool,
    search_dirty: bool,
    lines: usize,
}

impl Editor {
    fn new(mut query: Vec<u8>, lines: usize, multi: bool) -> Self {
        query.truncate(4096);
        let cursor = query.len();
        Self {
            query,
            cursor,
            multi,
            focus_list: false,
            marked: Vec::new(),
            input: Vec::new(),
            ambiguous_key_pending: false,
            search_dirty: false,
            lines,
        }
    }

    fn update_choices(&mut self, choices: &mut fz::Choices) {
        if self.search_dirty {
            choices.search(&self.query);
            self.search_dirty = false;
        }
    }

    fn insert(&mut self, byte: u8) -> bool {
        if self.query.len() == 4096 {
            return false;
        }
        self.query.insert(self.cursor, byte);
        self.cursor += 1;
        true
    }

    fn replace(&mut self, candidate: &[u8]) -> bool {
        let end = candidate.len().min(4096);
        if self.query == candidate[..end] {
            self.cursor = end;
            return false;
        }
        self.query.clear();
        self.query.extend_from_slice(&candidate[..end]);
        self.cursor = end;
        true
    }

    fn delete_character(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        let start = previous_boundary(&self.query, self.cursor);
        self.query.drain(start..self.cursor);
        self.cursor = start;
        true
    }

    fn delete_word(&mut self) -> bool {
        let original = self.cursor;
        let mut cursor = original;
        while cursor > 0 && is_ascii_space(self.query[cursor - 1]) {
            cursor -= 1;
        }
        while cursor > 0 && !is_ascii_space(self.query[cursor - 1]) {
            cursor -= 1;
        }
        if cursor == original {
            return false;
        }
        self.query.drain(cursor..original);
        self.cursor = cursor;
        true
    }

    fn delete_to_beginning(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.query.drain(..self.cursor);
        self.cursor = 0;
        true
    }

    fn left(&mut self) {
        self.cursor = previous_boundary(&self.query, self.cursor);
    }

    fn right(&mut self) {
        self.cursor = next_boundary(&self.query, self.cursor);
    }
}

fn wrapped_rows(candidate: &[u8], prefix_width: usize, width: usize) -> usize {
    if width <= prefix_width {
        return 1;
    }
    let mut column = prefix_width;
    let mut rows = 1usize;
    let mut index = 0usize;
    while index < candidate.len() {
        let (scalar, length) = decode_char(&candidate[index..]);
        let char_width = if candidate[index] == b'\n' { 1 } else { scalar_width(scalar) };
        if char_width > 0 && column + char_width > width {
            rows += 1;
            column = char_width;
        } else {
            column += char_width;
        }
        index += length;
    }
    rows
}

fn decode_char(bytes: &[u8]) -> (u32, usize) {
    let first = bytes[0];
    if first < 0x80 {
        return (u32::from(first), 1);
    }
    let (mut scalar, length) = match first {
        0xC2..=0xDF => (u32::from(first & 0x1F), 2),
        0xE0..=0xEF => (u32::from(first & 0x0F), 3),
        0xF0..=0xF4 => (u32::from(first & 0x07), 4),
        _ => return (u32::from(first), 1),
    };
    if bytes.len() < length {
        return (u32::from(first), 1);
    }
    for &byte in bytes.iter().take(length).skip(1) {
        if byte & 0b1100_0000 != 0b1000_0000 {
            return (u32::from(first), 1);
        }
        scalar = (scalar << 6) | u32::from(byte & 0x3F);
    }
    let minimum = match length { 2 => 0x80, 3 => 0x800, _ => 0x1_0000 };
    if scalar < minimum || scalar > 0x10_FFFF || (0xD800..0xE000).contains(&scalar) {
        return (u32::from(first), 1);
    }
    (scalar, length)
}

fn scalar_width(scalar: u32) -> usize {
    if scalar < 0x20 || (0x7F..0xA0).contains(&scalar) {
        return 1;
    }
    if matches!(scalar,
        0x0300..=0x036F | 0x0483..=0x0489 | 0x0591..=0x05BD | 0x05BF
        | 0x05C1..=0x05C2 | 0x05C4..=0x05C7 | 0x0610..=0x061A | 0x064B..=0x065F
        | 0x0670 | 0x06D6..=0x06DC | 0x06DF..=0x06E4 | 0x06E7..=0x06E8 | 0x06EA..=0x06ED
        | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x20D0..=0x20FF | 0xFE00..=0xFE0F
        | 0xFE20..=0xFE2F)
    {
        return 0;
    }
    if matches!(scalar,
        0x1100..=0x115F | 0x2329..=0x232A | 0x2E80..=0x303E | 0x3041..=0x33FF
        | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xA000..=0xA4CF | 0xAC00..=0xD7A3
        | 0xF900..=0xFAFF | 0xFE10..=0xFE19 | 0xFE30..=0xFE6F | 0xFF00..=0xFF60
        | 0xFFE0..=0xFFE6 | 0x1F300..=0x1F64F | 0x1F900..=0x1F9FF | 0x20000..=0x3FFFD)
    {
        return 2;
    }
    1
}

fn is_printable(byte: u8) -> bool {
    byte >= 0x80 || (0x20..=0x7e).contains(&byte)
}

fn is_ascii_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

fn is_continuation(byte: u8) -> bool {
    byte & 0b1100_0000 == 0b1000_0000
}

fn previous_boundary(bytes: &[u8], cursor: usize) -> usize {
    if cursor == 0 {
        return 0;
    }
    let mut position = cursor - 1;
    while position > 0 && is_continuation(bytes[position]) {
        position -= 1;
    }
    position
}

fn next_boundary(bytes: &[u8], cursor: usize) -> usize {
    if cursor >= bytes.len() {
        return bytes.len();
    }
    let mut position = cursor + 1;
    while position < bytes.len() && is_continuation(bytes[position]) {
        position += 1;
    }
    position
}

#[cfg(all(test, target_os = "linux"))]
mod linux_tests {
    use super::ResizeEvents;
    use std::fs::File;
    use std::io;
    use std::mem::MaybeUninit;

    struct RestoreMask(libc::sigset_t);

    impl Drop for RestoreMask {
        fn drop(&mut self) {
            // The saved mask belongs to this test thread. Test cleanup must
            // not leak its temporary signal state into the harness.
            let _ = unsafe {
                libc::pthread_sigmask(libc::SIG_SETMASK, &self.0, std::ptr::null_mut())
            };
        }
    }

    fn current_mask() -> io::Result<libc::sigset_t> {
        // libc may leave unused sigset_t storage untouched; a zero mask is a
        // valid initial representation for that storage.
        let mut mask = MaybeUninit::<libc::sigset_t>::zeroed();
        let result = unsafe {
            libc::pthread_sigmask(libc::SIG_SETMASK, std::ptr::null(), mask.as_mut_ptr())
        };
        if result == 0 {
            // SAFETY: pthread_sigmask filled its kernel-visible prefix and
            // the zeroed remainder was already initialized.
            Ok(unsafe { mask.assume_init() })
        } else {
            Err(io::Error::from_raw_os_error(result))
        }
    }

    fn signal_set(signal: libc::c_int) -> io::Result<libc::sigset_t> {
        let mut set = MaybeUninit::<libc::sigset_t>::zeroed();
        if unsafe { libc::sigemptyset(set.as_mut_ptr()) } == -1
            || unsafe { libc::sigaddset(set.as_mut_ptr(), signal) } == -1
        {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: zeroed storage is a valid empty mask, and libc updated its
        // required signal-set prefix.
        Ok(unsafe { set.assume_init() })
    }

    fn change_signal_mask(how: libc::c_int, signal: libc::c_int) -> io::Result<()> {
        let set = signal_set(signal)?;
        let result = unsafe { libc::pthread_sigmask(how, &set, std::ptr::null_mut()) };
        if result == 0 {
            Ok(())
        } else {
            Err(io::Error::from_raw_os_error(result))
        }
    }

    fn is_blocked(mask: &libc::sigset_t, signal: libc::c_int) -> bool {
        match unsafe { libc::sigismember(mask, signal) } {
            0 => false,
            1 => true,
            _ => panic!("sigismember({signal}) failed: {}", io::Error::last_os_error()),
        }
    }

    #[test]
    fn resize_events_restore_blocked_and_unblocked_caller_masks() -> io::Result<()> {
        let original = current_mask()?;
        let _restore_original = RestoreMask(original);

        // Preserve both legal caller states. A parent may already block
        // SIGWINCH, while an unrelated blocked signal proves restoration uses
        // the complete saved mask rather than only unblocking SIGWINCH.
        change_signal_mask(libc::SIG_BLOCK, libc::SIGUSR1)?;
        for caller_blocks_winch in [false, true] {
            change_signal_mask(
                if caller_blocks_winch {
                    libc::SIG_BLOCK
                } else {
                    libc::SIG_UNBLOCK
                },
                libc::SIGWINCH,
            )?;
            let expected_after_restore = current_mask()?;

            let input = File::open("/dev/null")?;
            let mut events = ResizeEvents::install(&input)?;
            let installed_mask = current_mask()?;
            assert!(is_blocked(&installed_mask, libc::SIGWINCH));
            assert!(is_blocked(&installed_mask, libc::SIGUSR1));

            events.restore()?;
            let restored_mask = current_mask()?;
            assert_eq!(
                is_blocked(&restored_mask, libc::SIGWINCH),
                caller_blocks_winch
            );
            assert_eq!(
                is_blocked(&restored_mask, libc::SIGUSR1),
                is_blocked(&expected_after_restore, libc::SIGUSR1)
            );
        }
        Ok(())
    }
}
