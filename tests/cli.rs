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
