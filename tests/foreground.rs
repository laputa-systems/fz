use std::os::unix::ffi::OsStringExt;
use std::process::Command;

fn caller_signal_state() -> Vec<(i32, i32, usize)> {
    // SAFETY: zeroed signal records are initialized ABI storage, and every
    // queried signal is valid. The worker remains on this thread throughout.
    unsafe {
        let mut mask: libc::sigset_t = std::mem::zeroed();
        assert_eq!(libc::pthread_sigmask(libc::SIG_SETMASK, std::ptr::null(), &mut mask), 0);
        [libc::SIGHUP, libc::SIGTERM, libc::SIGQUIT, libc::SIGWINCH, libc::SIGTTOU, libc::SIGUSR1]
            .into_iter().map(|signal| {
                let mut action: libc::sigaction = std::mem::zeroed();
                assert_eq!(libc::sigaction(signal, std::ptr::null(), &mut action), 0);
                let blocked = libc::sigismember(&mask, signal);
                assert!(blocked >= 0);
                (signal, blocked, action.sa_sigaction)
            }).collect()
    }
}

fn prepare_caller_signals() {
    // SAFETY: these initialized records change only this disposable worker's
    // state. The caller's blocked signals and ignored HUP must survive fz.
    unsafe {
        let mut mask: libc::sigset_t = std::mem::zeroed();
        assert_eq!(libc::sigemptyset(&mut mask), 0);
        assert_eq!(libc::sigaddset(&mut mask, libc::SIGTERM), 0);
        assert_eq!(libc::sigaddset(&mut mask, libc::SIGUSR1), 0);
        assert_eq!(libc::pthread_sigmask(libc::SIG_BLOCK, &mask, std::ptr::null_mut()), 0);
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = libc::SIG_IGN;
        assert_eq!(libc::sigemptyset(&mut action.sa_mask), 0);
        assert_eq!(libc::sigaction(libc::SIGHUP, &action, std::ptr::null_mut()), 0);
    }
}

fn main() {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() == Some(std::ffi::OsStr::new("--worker")) {
        let arguments: Vec<_> = args.map(|arg| arg.into_vec()).collect();
        let rounds = std::env::var("FZ_WORKER_ROUNDS").unwrap_or_else(|_| "1".into()).parse::<usize>().unwrap();
        assert!((1..=2).contains(&rounds));
        prepare_caller_signals();
        let original = caller_signal_state();
        let signal_thread = std::env::var_os("FZ_WORKER_TERM_CONTROL").map(|path| {
            assert_eq!(rounds, 1);
            std::thread::spawn(move || {
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
                while std::fs::read(&path).unwrap() != b"terminate" {
                    assert!(std::time::Instant::now() < deadline, "signal control never arrived");
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                // SAFETY: this disposable helper thread unblocks and raises a valid
                // signal on itself, so the selector's waiting thread is not interrupted.
                unsafe {
                    let mut set: libc::sigset_t = std::mem::zeroed();
                    assert_eq!(libc::sigemptyset(&mut set), 0);
                    assert_eq!(libc::sigaddset(&mut set, libc::SIGTERM), 0);
                    assert_eq!(libc::pthread_sigmask(libc::SIG_UNBLOCK, &set, std::ptr::null_mut()), 0);
                    assert_eq!(libc::raise(libc::SIGTERM), 0);
                }
            })
        });
        let mut status = 0;
        for _ in 0..rounds {
            let pending = std::env::var_os("FZ_PENDING_INPUT")
                .map(|path| std::fs::File::open(path).expect("open pending worker input"));
            status = fz::cli::run_in_foreground(arguments.clone().into_iter(), pending);
            assert_eq!(caller_signal_state(), original, "worker leaked signal state");
            if status != 0 { break; }
        }
        if let Some(thread) = signal_thread { thread.join().unwrap(); }
        std::process::exit(status);
    }
    let status = Command::new("python3")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/foreground_probe.py"))
        .arg(std::env::current_exe().expect("foreground test executable"))
        .status()
        .expect("python3 must be available for the worker boundary test");
    assert!(status.success(), "foreground worker probe failed: {status}");
}
