Prefix: FZY
Next: 2

# Backlog: fz

### FZY-1 · P1 · macOS acceptance hangs on bare Escape

- **Area:** Concurrency
- **Where:** `tests/acceptance.py` (`test_escape_timeout_and_fragmented_arrow_sequence`), `src/terminal.rs`
- **Problem:** GitHub-hosted macOS CI (Darwin 25.6.0, macOS 26.7.2, Python 3.15.2, Rust 1.99.1) hangs in `test fzy_acceptance_contract` after `test_escape_cancel_survives_sigwinch` returns. The next test is `test_escape_timeout_and_fragmented_arrow_sequence`. The job hits `timeout-minutes: 15` and is cancelled, leaving a Python process and the `acceptance` binary. Runs: [42535328256](https://github.com/laputa-systems/fz/actions/runs/42535328256) on `main` and [42538854372](https://github.com/laputa-systems/fz/actions/runs/42538854372) on the sigwait fix. A Darwin 27 host returns from that wait, and a C probe of `select$DARWIN_EXTSN$NOCANCEL` with a 25ms timeval returned in about 29ms. The wall-clock deadline is only checked after `select` returns, so a `select` that does not return never cancels the bare Escape. Unblocks when that test returns on the GitHub macOS runner.
- **Recommendation:** Make the 25ms bare-Escape wait return on Darwin 25 without a wake-pipe write. Do not raise the job timeout and do not delete the test.
- **Fix with:** upgrade-the-world
- **Effort:** M
