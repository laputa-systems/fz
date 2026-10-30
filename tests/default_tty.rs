use ptytest::{CommandSpec, ExitStatus, PtyTest, Scenario, Size, TestEnv};
use std::time::Duration;

// Selecting through the default /dev/tty must redraw after every key.
// macOS poll() reports POLLNVAL for /dev/tty, which the input wait loop
// treated as readable input: every zero-timeout drain skipped its draw, so
// the screen froze after the first frame while state kept advancing.
// This test omits --tty so fz uses /dev/tty, then requires a visible mark
// after Down+Space; without the select-based wait it times out here.
#[test]
fn multi_select_redraws_after_navigation_on_default_tty() -> Result<(), ptytest::PtyTestError> {
    let scenario = Scenario::new("fz-default-tty-multi-redraw")?
        .command(
            CommandSpec::new(env!("CARGO_BIN_EXE_fz"))
                .args(["--multi", "--query=alpha", "--prompt=TERM> "]),
        )
        .size(Size::new(80, 24)?)
        .environment(TestEnv::hermetic()?);
    let mut terminal = PtyTest::spawn(scenario)?;
    // Stdin is the slave: candidates arrive in canonical mode, EOF ends the
    // candidate phase, then fz opens /dev/tty (the same slave) to interact.
    terminal.send_text(
        terminal.deadline(Duration::from_secs(3)),
        "alpha one\nalpha two\nbeta\n",
    )?;
    terminal.send_eof(terminal.deadline(Duration::from_secs(3)))?;
    terminal.wait_for_screen(
        terminal.deadline(Duration::from_secs(3)),
        "initial multi-select screen",
        |screen| screen.contains("[2/3] [0 selected]"),
    )?;
    // Down enters the result list; Space marks the focused row.
    terminal.send_bytes(terminal.deadline(Duration::from_secs(3)), b"\x1b[B")?;
    terminal.send_bytes(terminal.deadline(Duration::from_secs(3)), b" ")?;
    terminal.wait_for_screen(
        terminal.deadline(Duration::from_secs(3)),
        "marked first row",
        |screen| screen.contains("[2/3] [1 selected]") && screen.contains("[x] alpha one"),
    )?;
    terminal.send_bytes(terminal.deadline(Duration::from_secs(3)), b"\r")?;
    let status = terminal.wait_for_exit(terminal.deadline(Duration::from_secs(3)))?;
    assert_eq!(status, ExitStatus::Code(0));
    let status = terminal.finish(terminal.deadline(Duration::from_secs(3)))?;
    assert_eq!(status, ExitStatus::Code(0));
    Ok(())
}
