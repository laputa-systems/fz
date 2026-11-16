use std::os::unix::process::CommandExt;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;
use std::time::{Duration, Instant};

static STOP: AtomicBool = AtomicBool::new(false);
static SIGNALS: Once = Once::new();

extern "C" fn request_stop(_: libc::c_int) {
    STOP.store(true, Ordering::Relaxed);
}

struct Harness {
    child: Child,
    pids: PathBuf,
}

impl Harness {
    fn start(script: &str, label: &str) -> Self {
        Self::start_with_binary(script, label, env!("CARGO_BIN_EXE_fz"), false)
    }

    fn start_with_binary(script: &str, label: &str, binary: &str, silent: bool) -> Self {
        SIGNALS.call_once(|| unsafe {
            libc::signal(libc::SIGTERM, request_stop as *const () as libc::sighandler_t);
            libc::signal(libc::SIGINT, request_stop as *const () as libc::sighandler_t);
            libc::signal(libc::SIGHUP, request_stop as *const () as libc::sighandler_t);
        });
        let pids = std::env::temp_dir().join(format!("fz-{label}-{}.pids", std::process::id()));
        std::fs::write(&pids, "").expect("create session list");
        let child = Command::new("python3")
            .arg(format!("{}/tests/{script}", env!("CARGO_MANIFEST_DIR")))
            .arg(binary)
            .env("FZ_SESSION_PIDS", &pids)
            .stdin(Stdio::null())
            .stdout(if silent { Stdio::null() } else { Stdio::inherit() })
            .stderr(if silent { Stdio::null() } else { Stdio::inherit() })
            .process_group(0)
            .spawn()
            .expect("python3 must be available to run the test harness");
        Self { child, pids }
    }

    fn wait(&mut self, limit: Duration) -> ExitStatus {
        let started = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().expect("test harness wait") {
                return status;
            }
            assert!(!STOP.load(Ordering::Relaxed), "test harness interrupted");
            assert!(started.elapsed() < limit, "test harness exceeded {limit:?}");
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        unsafe { libc::kill(-(self.child.id() as libc::pid_t), libc::SIGKILL); }
        let _ = self.child.wait();
        if let Ok(pids) = std::fs::read_to_string(&self.pids) {
            for pid in pids.lines().filter_map(|s| s.parse::<libc::pid_t>().ok()).filter(|&p| p > 1) {
                unsafe { libc::kill(-pid, libc::SIGKILL); }
            }
        }
        let _ = std::fs::remove_file(&self.pids);
        let _ = std::fs::remove_file(self.pids.with_extension("pids.new"));
    }
}

#[test]
fn bare_escape_cancels_without_a_resize_storm() {
    let status = Harness::start("escape_probe.py", "escape").wait(Duration::from_secs(8));
    assert!(status.success(), "bare escape on a fresh pty failed: {status}");
}

#[test]
fn fzy_acceptance_contract() {
    let status = Harness::start("acceptance.py", "acceptance").wait(Duration::from_secs(18));
    assert!(status.success(), "acceptance harness failed: {status}");
}

#[test]
fn selectors_stop_when_python_is_terminated() {
    let binary = std::env::temp_dir().join(format!("fz-stuck-{}", std::process::id()));
    std::fs::write(&binary, "#!/bin/sh\nexec sleep 60\n").unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
    for (label, signal) in [("cleanup-term", libc::SIGTERM), ("cleanup-kill", libc::SIGKILL)] {
        let mut harness = Harness::start_with_binary("acceptance.py", label, binary.to_str().unwrap(), true);
        let deadline = Instant::now() + Duration::from_secs(5);
        let pid = loop {
            if let Ok(pid) = std::fs::read_to_string(&harness.pids).unwrap_or_default().trim().parse::<libc::pid_t>() {
                break pid;
            }
            assert!(Instant::now() < deadline, "selector did not start");
            std::thread::sleep(Duration::from_millis(10));
        };
        assert_eq!(unsafe { libc::kill(harness.child.id() as libc::pid_t, signal) }, 0);
        assert!(!harness.wait(Duration::from_secs(4)).success());
        drop(harness);
        let deadline = Instant::now() + Duration::from_secs(2);
        while unsafe { libc::kill(pid, 0) } == 0 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        let alive = unsafe { libc::kill(pid, 0) } == 0;
        if alive { unsafe { libc::kill(-pid, libc::SIGKILL); } }
        assert!(!alive, "selector {pid} survived signal {signal}");
    }
    std::fs::remove_file(binary).unwrap();
}
