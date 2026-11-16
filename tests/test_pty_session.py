import os
from pathlib import Path
import signal
import subprocess
import sys
import time
import unittest


class SessionLifetime(unittest.TestCase):
    def test_driver_death_reaps_a_selector_which_ignores_hangup_and_term(self):
        code = """
import os, pty, signal, sys
from pty_session import Session
master, slave = pty.openpty()
child = 'import os,signal,time;signal.signal(signal.SIGTERM,signal.SIG_IGN);signal.signal(signal.SIGHUP,signal.SIG_IGN);print(os.getpid(),flush=True);time.sleep(60)'
process = Session([sys.executable, '-c', child], slave)
print(process.stdout.readline().decode().strip(), flush=True)
signal.pause()
"""
        for signum in (signal.SIGTERM, signal.SIGKILL):
            with self.subTest(signal=signum):
                driver = subprocess.Popen([sys.executable, "-c", code],
                                          cwd=Path(__file__).parent,
                                          stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                          text=True)
                pid = None
                try:
                    import selectors
                    with selectors.DefaultSelector() as ready:
                        ready.register(driver.stdout, selectors.EVENT_READ)
                        self.assertTrue(ready.select(5), "selector did not start")
                    line = driver.stdout.readline()
                    self.assertTrue(line.strip().isdigit(), line)
                    pid = int(line)
                    driver.send_signal(signum)
                    self.assertEqual(driver.wait(timeout=5), -signum)
                    deadline = time.monotonic() + 5
                    while time.monotonic() < deadline:
                        try:
                            os.kill(pid, 0)
                        except ProcessLookupError:
                            break
                        time.sleep(0.01)
                    else:
                        self.fail("selector %d survived driver signal %d" % (pid, signum))
                finally:
                    if driver.poll() is None:
                        driver.kill()
                    driver.communicate(timeout=5)
                    if pid is not None:
                        try:
                            os.kill(pid, signal.SIGKILL)
                        except ProcessLookupError:
                            pass


if __name__ == "__main__":
    unittest.main()
