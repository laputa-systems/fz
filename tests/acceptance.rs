use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn bare_escape_cancels_without_a_resize_storm() {
    let mut child = Command::new("python3")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/escape_probe.py"))
        .arg(env!("CARGO_BIN_EXE_fz"))
        .stdin(Stdio::null())
        .spawn()
        .expect("python3 must be available to run the escape probe");
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().expect("escape probe wait") {
            assert!(
                status.success(),
                "bare escape on a fresh pty failed: {status}"
            );
            return;
        }
        if started.elapsed() > Duration::from_secs(8) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("escape probe still running after 8s");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn fzy_acceptance_contract() {
    let mut child = Command::new("python3")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/acceptance.py"))
        .arg(env!("CARGO_BIN_EXE_fz"))
        .spawn()
        .expect("python3 must be available to run the stdlib acceptance harness");
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("acceptance harness wait") {
            break status;
        }
        if started.elapsed() > Duration::from_secs(60) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("acceptance harness still running after 60s");
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    assert!(status.success(), "acceptance harness failed: {status}");
}
