use std::os::unix::ffi::OsStringExt;
use std::process::Command;

fn main() {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() == Some(std::ffi::OsStr::new("--worker")) {
        let pending = std::env::var_os("FZ_PENDING_INPUT")
            .map(|path| std::fs::File::open(path).expect("open pending worker input"));
        let status = fz::cli::run_in_foreground(args.map(|arg| arg.into_vec()), pending);
        std::process::exit(status);
    }
    let status = Command::new("python3")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/foreground_probe.py"))
        .arg(std::env::current_exe().expect("foreground test executable"))
        .status()
        .expect("python3 must be available for the worker boundary test");
    assert!(status.success(), "foreground worker probe failed: {status}");
}
