import json
import os
import sys
import tempfile
import errno
import subprocess
import signal
import termios
import time
from pathlib import Path

from acceptance import PtyFz


def run(args, pending=None, *, rounds=1, action=None,
        candidates=b"alpha\nbeta\n", expected_lines=None):
    session = None
    try:
        with tempfile.NamedTemporaryFile() as input_file:
            if pending is not None:
                input_file.write(pending)
                input_file.flush()
                os.environ["FZ_PENDING_INPUT"] = input_file.name
            else:
                os.environ.pop("FZ_PENDING_INPUT", None)
            os.environ["FZ_WORKER_ROUNDS"] = str(rounds)
            session = PtyFz(candidates, ["--worker", *args],
                            binary=Path(sys.argv[1]), foreground_owner="supervisor")
            if action is not None:
                if candidates is None:
                    deadline = time.monotonic() + 2
                    while termios.tcgetattr(session.master)[3] & termios.ICANON:
                        assert time.monotonic() < deadline, "worker never entered raw mode"
                        time.sleep(.005)
                else:
                    session.wait_for_lines(expected_lines or [">", "alpha", "beta"])
                action(session)
            code, stdout, stderr = session.finish(timeout=8)
        marker = b"FZ_FOREGROUND_STATE:"
        state_lines = [line for line in stderr.splitlines() if line.startswith(marker)]
        assert len(state_lines) == 1, stderr
        assert json.loads(state_lines[0][len(marker):]) == {
            "foregroundRestored": True, "terminalRestored": True,
        }, stderr
        diagnostics = b"\n".join(line for line in stderr.splitlines() if not line.startswith(marker))
        return code, stdout, diagnostics
    finally:
        if session is not None:
            session.close()


selected = run([], b"b\r")
assert selected == (0, b"beta\n", b""), selected
second_session = run([], b"b\r", rounds=2)
assert second_session == (0, b"beta\nb\n", b""), second_session
assert run([], b"\x03") == (1, b"", b"")
assert run([], b"b", action=lambda session: session.send(b"\r"),
           expected_lines=["> b", "beta"]) == (0, b"beta\n", b"")
assert run([], action=lambda session: session.proc.send_signal(signal.SIGTERM)) == (143, b"", b"")
for candidates in (b"alpha\nbeta\n", None):
    with tempfile.NamedTemporaryFile() as control:
        os.environ["FZ_WORKER_TERM_CONTROL"] = control.name
        try:
            def terminate(session):
                control.write(b"terminate")
                control.flush()
            result = run([], candidates=candidates, action=terminate)
            assert result == (143, b"", b""), result
        finally:
            os.environ.pop("FZ_WORKER_TERM_CONTROL", None)
code, help_stdout, help_text = run(["--help"])
assert code == 0
assert help_stdout == b""
assert b"default 1 in ish; small searches use 1" in help_text, help_text
if sys.platform == "linux":
    root = Path(__file__).resolve().parents[1]
    scratch = root / ".work/tmp"
    scratch.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=scratch) as directory:
        library = str(Path(directory) / "read-error.so")
        subprocess.run(["cc", "-shared", "-fPIC", str(root / "tests/read_error.c"),
                        "-o", library, "-ldl"], check=True)
        os.environ["LD_PRELOAD"] = library
        try:
            os.environ["FZ_READ_ERROR"] = str(errno.EINTR)
            assert run([], b"b\r") == (0, b"beta\n", b"")
            os.environ["FZ_READ_ERROR"] = str(errno.EIO)
            code, stdout, diagnostics = run([], b"b\r")
            assert (code, stdout) == (1, b"")
            assert b"os error 5" in diagnostics, diagnostics
            os.environ.pop("FZ_READ_ERROR")
            os.environ["FZ_TTY_READ_ERROR"] = str(errno.EINTR)
            assert run([], action=lambda session: session.send(b"b\r")) == (0, b"beta\n", b"")
            os.environ["FZ_TTY_READ_ERROR"] = str(errno.EIO)
            code, stdout, diagnostics = run([], action=lambda session: session.send(b"b"))
            assert (code, stdout) == (1, b"")
            assert b"os error 5" in diagnostics, diagnostics
            os.environ.pop("FZ_TTY_READ_ERROR")
            os.environ["FZ_SIGNALFD_ERROR"] = str(errno.EMFILE)
            code, stdout, diagnostics = run([], b"b\r")
            assert (code, stdout) == (1, b"")
            assert b"os error 24" in diagnostics, diagnostics
        finally:
            os.environ.pop("LD_PRELOAD", None)
            os.environ.pop("FZ_READ_ERROR", None)
            os.environ.pop("FZ_TTY_READ_ERROR", None)
            os.environ.pop("FZ_SIGNALFD_ERROR", None)
print("foreground worker selection, cancellation, pending input and help passed")
