#!/usr/bin/env python3
"""Sample a live acceptance process from outside it, then kill it."""

import os
import signal
import subprocess
import sys
import time

pid = int(sys.argv[1])
time.sleep(30)
try:
    os.kill(pid, 0)
except OSError:
    raise SystemExit(0)

print("EXTERNAL-DUMP", pid, flush=True)
listing = subprocess.check_output(
    ["ps", "-ax", "-o", "pid=,ppid=,stat=,wchan=,command="],
    text=True,
    timeout=3,
)
rows = []
for line in listing.splitlines():
    fields = line.split(None, 4)
    if len(fields) >= 4 and fields[0].isdigit():
        rows.append(fields)
wanted = {str(pid)}
changed = True
while changed:
    changed = False
    for fields in rows:
        if fields[1] in wanted and fields[0] not in wanted:
            wanted.add(fields[0])
            changed = True
for fields in rows:
    if fields[0] in wanted:
        print(" ".join(fields), flush=True)
if sys.platform == "darwin":
    for item in sorted(wanted):
        print("THREADS", item, flush=True)
        subprocess.run(
            ["ps", "-M", "-p", item, "-o", "pid,stat,wchan,command"],
            timeout=3,
            check=False,
        )
        subprocess.run(["sample", item, "1"], timeout=8, check=False)
try:
    os.kill(pid, signal.SIGKILL)
except OSError as error:
    print("kill failed", error, flush=True)
