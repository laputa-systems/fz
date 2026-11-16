use std::io::Write;
use std::process::{Command, Output, Stdio};

fn run(args: &[&str], input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_fz"))
        .args(args).stdin(Stdio::piped()).stdout(Stdio::piped())
        .stderr(Stdio::piped()).spawn().unwrap();
    // A program that fails before reading stdin (such as a missing tty) closes the pipe first.
    match child.stdin.take().unwrap().write_all(input) {
        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => {}
        result => result.unwrap(),
    }
    child.wait_with_output().unwrap()
}

#[test]
fn filter_preserves_bytes_delimiters_and_stable_ties() {
    for (args, input, expected) in [
        (vec!["-e", ""], b"\nfoo\n\nbar\nfoo".as_slice(), b"foo\nbar\nfoo\n".as_slice()),
        (vec!["-0e", ""], b"foo\nbar\0\0baz\0", b"foo\nbar\nbaz\n"),
        (vec!["--read-null", "--show-matches="], b"foo\nbar\0\0baz\0", b"foo\nbar\nbaz\n"),
        (vec!["--workers=1", "--show-info", "--show-matches=ba"], b"foo\nbar\nbaz\n", b"bar\nbaz\n"),
        (vec!["--show-matches="], b"\xff\ncarriage\r\n", b"\xff\ncarriage\r\n"),
        (vec!["-e", ""], b"foo\0ignored\nbar\n", b"foo\n"),
        (vec!["-e", "x"], b"foo\nbar\n", b""),
        (vec!["-e", "F"], b"foo\nFoo\nFOO\n", b"Foo\nFOO\n"),
    ] {
        let result = run(&args, input);
        assert_eq!(result.status.code(), Some(0), "{args:?}: {:?}", result.stderr);
        assert_eq!(result.stdout, expected, "{args:?}");
        assert!(result.stderr.is_empty());
    }
}

#[test]
fn filter_accepts_attached_short_and_long_values() {
    for (args, expected) in [
        (vec!["-efoo"], b"foo\n".as_slice()),
        (vec!["--show-matches=bar"], b"bar\n".as_slice()),
        (vec!["-sef"], b"0.890000\tfoo\n".as_slice()),
    ] {
        let result = run(&args, b"foo\nbar\n");
        assert!(result.status.success(), "{args:?}: {:?}", result.stderr);
        assert_eq!(result.stdout, expected, "{args:?}");
    }
}

#[test]
fn filter_formats_finite_and_infinite_scores_like_fzy() {
    let result = run(&["-se", "f"], b"f\nfoo\nbar\n");
    assert!(result.status.success());
    assert_eq!(result.stdout, b"inf\tf\n0.890000\tfoo\n");
    assert_eq!(run(&["-se", ""], b"foo\n").stdout, b"-inf\tfoo\n");
}

#[test]
fn invalid_lines_and_positional_arguments_fail_without_stdout() {
    for args in [vec!["-l", "2"], vec!["-l", "bad"], vec!["unexpected"]] {
        let result = run(&args, b"");
        assert_eq!(result.status.code(), Some(1));
        assert!(result.stdout.is_empty());
        assert!(!result.stderr.is_empty());
    }
}

#[test]
fn multi_and_noninteractive_filter_are_incompatible() {
    let result = run(&["--multi", "--show-matches=x"], b"x\n");
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());
    assert!(String::from_utf8_lossy(&result.stderr).contains("cannot be combined"));
}

#[test]
fn multi_reports_terminal_errors_separately_from_cancellation() {
    let result = run(&["--multi", "--tty", "/definitely/missing/fz-tty"], b"foo\n");
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
    assert!(String::from_utf8_lossy(&result.stderr).contains("fz:"));
}

#[test]
fn help_and_version_do_not_require_a_terminal() {
    let help = run(&["--help"], b"");
    assert!(help.status.success());
    assert!(help.stdout.is_empty());
    assert!(String::from_utf8_lossy(&help.stderr).contains("Usage: fz"));
    let version = run(&["--version"], b"");
    assert!(version.status.success());
    assert!(String::from_utf8_lossy(&version.stdout).starts_with("fz "));
}

#[test]
fn built_help_ends_with_owned_tool_footer() {
    let footer = "Maintained by Joshua Li: https://github.com/laputa-systems/fz\n\
Bugs and features: TODO.md in that repository (follow its AGENTS.md), or its issue tracker.\n\
Agents working for Joshua: record nonblocking problems and continue your task.\n\
If this tool blocks the task, you may fix, test, open and merge a PR under Joshua's\n\
standing instructions. Release only when needed to deliver the verified fix.\n\
Procedure: https://github.com/joshuarli/usr/blob/main/dotfiles/base/.agents/instructions/owned-tools.md\n";
    for arg in ["--help", "-h"] {
        let output = run(&[arg], b"");
        assert!(output.status.success());
        assert!(output.stdout.is_empty());
        assert!(output.stderr.ends_with(footer.as_bytes()));
    }
    for args in [&["--version"][..], &["--unknown"][..], &["-e", "x"][..]] {
        let output = run(args, b"x\n");
        assert!(!output.stdout.windows(b"Maintained by Joshua Li:".len()).any(|w| w == b"Maintained by Joshua Li:"));
        assert!(!output.stderr.windows(b"Maintained by Joshua Li:".len()).any(|w| w == b"Maintained by Joshua Li:"));
    }
}

#[test]
fn missing_tty_is_failure_without_selection() {
    let result = run(&["--tty", "/definitely/missing/fz-tty"], b"foo\n");
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn native_input_errors_and_worker_limits() {
    use std::time::{Duration, Instant};

    struct Scratch(std::path::PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".work/tmp");
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join(format!("read-errors-{}", std::process::id()));
    std::fs::create_dir(&path).unwrap();
    let scratch = Scratch(path);
    let library = scratch.0.join("read-error.so");
    let status = Command::new("cc").args(["-shared", "-fPIC"])
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/read_error.c"))
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/thread_count.c"))
        .arg("-o").arg(&library).args(["-ldl", "-pthread"]).status().unwrap();
    assert!(status.success());
    let input_file = scratch.0.join("input");
    std::fs::write(&input_file, b"alpha\n").unwrap();
    for (error, code, stdout) in [(libc::EINTR, 0, b"alpha\n".as_slice()),
                                  (libc::EIO, 1, b"".as_slice())] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_fz"))
            .args(["-e", "a"]).env("LD_PRELOAD", &library)
            .env("FZ_READ_ERROR", error.to_string())
            .stdin(std::fs::File::open(&input_file).unwrap()).stdout(Stdio::piped()).stderr(Stdio::piped())
            .spawn().unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while child.try_wait().unwrap().is_none() {
            if Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("stdin error {error} did not terminate");
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(code));
        assert_eq!(output.stdout, stdout);
        assert_eq!(output.stderr.is_empty(), code == 0);
    }
    let mut input = b"zzzzzzzzzz\n".repeat(20_000);
    input.extend_from_slice(b"aaaa\n");
    for (workers, count) in [("1", b"0\n".as_slice()), ("4", b"3\n".as_slice())] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_fz"))
            .args(["--workers", workers, "-e", "aaaa"])
            .env("LD_PRELOAD", &library).env("FZ_COUNT_THREADS", "1")
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
            .spawn().unwrap();
        child.stdin.take().unwrap().write_all(&input).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, b"aaaa\n");
        assert_eq!(output.stderr, count);
    }
    std::fs::write(&input_file, b"x\n".repeat(524_288)).unwrap();
    let output = Command::new("python3").args(["-c",
        "import os,resource,sys; resource.setrlimit(resource.RLIMIT_AS,(128*1024*1024,128*1024*1024)); os.execv(sys.argv[1],sys.argv[1:])",
        env!("CARGO_BIN_EXE_fz"), "--workers", "1", "-e", "z"])
        .stdin(std::fs::File::open(&input_file).unwrap()).output().unwrap();
    assert!(output.status.success(), "1 MiB of records exceeded 128 MiB: {:?}", output);
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}
