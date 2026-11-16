import os
import json
from pathlib import Path
import selectors
import signal
import subprocess
import sys


class Session(subprocess.Popen):
    def __init__(self, args, slave, ignore_hangup=False, *, foreground_owner="selector"):
        if foreground_owner not in ("selector", "supervisor"):
            raise ValueError("invalid foreground owner")
        read_fd, self.watch_fd = os.pipe()
        try:
            super().__init__(
                [sys.executable, str(Path(__file__).resolve()), str(read_fd),
                 str(slave), str(int(ignore_hangup)),
                 str(int(foreground_owner == "selector")), *args],
                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                pass_fds=(read_fd, slave), start_new_session=True,
            )
        except BaseException:
            os.close(self.watch_fd)
            raise
        finally:
            os.close(read_fd)

    def kill(self):
        if self.watch_fd is not None:
            os.close(self.watch_fd)
            self.watch_fd = None

    def wait(self, timeout=None):
        status = super().wait(timeout)
        self.kill()
        return status


def guard(watch_fd, slave, ignore_hangup, selector_foreground, args):
    import fcntl
    import termios

    fcntl.ioctl(slave, termios.TIOCSCTTY, 0)
    original_termios = termios.tcgetattr(slave)
    child = None
    pending = []

    def forward(signum, frame):
        if child is None:
            pending.append(signum)
        elif child.poll() is None:
            try:
                os.killpg(child.pid, signum)
            except ProcessLookupError:
                pass

    for signum in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGQUIT, signal.SIGWINCH):
        signal.signal(signum, signal.SIG_IGN if ignore_hangup and signum == signal.SIGHUP else forward)

    def foreground():
        os.setpgid(0, 0)
        signal.signal(signal.SIGTTOU, signal.SIG_IGN)
        if selector_foreground:
            os.tcsetpgrp(slave, os.getpid())
        signal.signal(signal.SIGTTOU, signal.SIG_DFL)

    try:
        child = subprocess.Popen(args, pass_fds=(slave,), preexec_fn=foreground)
        for signum in pending:
            forward(signum, None)
        with selectors.DefaultSelector() as selector:
            selector.register(watch_fd, selectors.EVENT_READ)
            while child.poll() is None:
                if selector.select(0.05) and not os.read(watch_fd, 1):
                    break
    finally:
        if child is not None:
            if child.returncode is None:
                try:
                    os.killpg(child.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
            status = child.wait()
            if not selector_foreground:
                # Darwin sets PENDIN on restoring ICANON. FIONREAD processes
                # pending input without consuming it before comparing attributes.
                # https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/tty.c
                fcntl.ioctl(slave, termios.FIONREAD, b"\0" * 4)
                state = {"foregroundRestored": os.tcgetpgrp(slave) == os.getpgrp(),
                         "terminalRestored": termios.tcgetattr(slave) == original_termios}
                print("FZ_FOREGROUND_STATE:" + json.dumps(state), file=sys.stderr)
        os.close(slave)
        os.close(watch_fd)
    if status < 0:
        signum = -status
        signal.signal(signum, signal.SIG_DFL)
        os.kill(os.getpid(), signum)
    return status


if __name__ == "__main__":
    try:
        sys.exit(guard(int(sys.argv[1]), int(sys.argv[2]), bool(int(sys.argv[3])),
                       bool(int(sys.argv[4])), sys.argv[5:]))
    except Exception:
        print("FZ_HARNESS_ERROR: PTY supervisor failed", file=sys.stderr)
        raise
