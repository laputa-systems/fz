import json
import os
import sys
import tempfile
import errno
import subprocess
from pathlib import Path

from acceptance import PtyFz


def run(args, pending=None):
    session = None
    try:
        with tempfile.NamedTemporaryFile() as input_file:
            if pending is not None:
                input_file.write(pending)
                input_file.flush()
                os.environ["FZ_PENDING_INPUT"] = input_file.name
            else:
                os.environ.pop("FZ_PENDING_INPUT", None)
            session = PtyFz(b"alpha\nbeta\n", ["--worker", *args],
                            binary=Path(sys.argv[1]), foreground_owner="supervisor")
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


assert run([], b"b\r") == (0, b"beta\n", b"")
assert run([], b"\x03") == (1, b"", b"")
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
        finally:
            os.environ.pop("LD_PRELOAD", None)
            os.environ.pop("FZ_READ_ERROR", None)
print("foreground worker selection, cancellation, pending input and help passed")
