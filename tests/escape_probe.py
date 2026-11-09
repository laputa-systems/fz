#!/usr/bin/env python3
"""One bare Escape on a fresh PTY. A blocked call prints its name and exits."""

import faulthandler
import fcntl
import os
import pty
import struct
import subprocess
import sys
import termios
import threading
import time

binary = sys.argv[1]
master, slave = pty.openpty()
fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
for fd in (master, slave):
    flags = fcntl.fcntl(fd, fcntl.F_GETFD)
    fcntl.fcntl(fd, fcntl.F_SETFD, flags | fcntl.FD_CLOEXEC)
name = os.ttyname(slave)
proc = subprocess.Popen(
    [binary, "--tty", name],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    start_new_session=True,
)
proc.stdin.write(b"foo\nbar\n")
proc.stdin.close()


def give_up(label):
    print(label, "fz-pid", proc.pid, flush=True)
    pids = {str(os.getpid()), str(proc.pid)}
    try:
        listing = subprocess.check_output(
            ["ps", "-ax", "-o", "pid,ppid,stat,wchan,command"],
            text=True,
            timeout=2,
        )
        kept = []
        for line in listing.splitlines():
            fields = line.split()
            if not fields:
                continue
            if fields[0] == "PID" or fields[0] in pids or (len(fields) > 1 and fields[1] in pids):
                kept.append(line)
                if fields[0].isdigit():
                    pids.add(fields[0])
        print("\n".join(kept), flush=True)
        if sys.platform == "darwin":
            print(
                subprocess.check_output(
                    ["ps", "-M", "-p", str(proc.pid), "-o", "pid,stat,wchan,command"],
                    text=True,
                    timeout=2,
                ),
                flush=True,
            )
    except (OSError, subprocess.SubprocessError) as error:
        print("ps failed", error, flush=True)
    faulthandler.dump_traceback(file=sys.stderr, all_threads=True)
    try:
        proc.kill()
    except OSError:
        pass
    os._exit(86)


def drain_master():
    while True:
        try:
            chunk = os.read(master, 65536)
        except OSError:
            return
        if not chunk:
            return


def arm(label):
    timer = threading.Timer(2.0, give_up, args=(label,))
    timer.daemon = True
    timer.start()
    return timer


pending = arm("probe-blocked-during-read")
print("probe-before-read", flush=True)
buffered = b""
while b"foo" not in buffered:
    chunk = os.read(master, 4096)
    if not chunk:
        print("probe-eof-before-draw", flush=True)
        os._exit(2)
    buffered += chunk
pending.cancel()
print("probe-drew", flush=True)
threading.Thread(target=drain_master, daemon=True).start()

pending = arm("probe-blocked-during-write")
print("probe-before-write", flush=True)
view = memoryview(b"\x1b")
while view:
    wrote = os.write(master, view)
    if wrote == 0:
        print("probe-short-write", flush=True)
        os._exit(4)
    view = view[wrote:]
pending.cancel()
print("probe-wrote", flush=True)
print("probe-before-wait", flush=True)
deadline = time.monotonic() + 1.5
while time.monotonic() < deadline:
    code = proc.poll()
    if code is not None:
        print("probe-exited", code, flush=True)
        os._exit(0 if code == 1 else 3)
    time.sleep(0.02)
give_up("probe-fz-did-not-exit")
