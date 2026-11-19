# fz

`fz` is a minimal Rust port of fzy that preserves its observable command-line behavior on macOS and Linux.

- Build: `cargo build`
- Test: `cargo test`
- Check: `cargo clippy --all-targets --all-features -- --deny warnings`
- Benchmarks: `make bench-list`, `make bench-check`, `make bench` (selectors in `benches/README.md`; Full is manual-only)

## Hard invariants

- Preserve fzy's matching, ranking, key bindings, exit status, and stdin/stdout behavior in default single-select mode.
- Keep `--multi` strictly opt-in: Down enters results, Space toggles marks, and Enter outputs the selected rows. Cancellation/no-match returns 1; terminal failures return 2. Do not alter default fzy behavior to support it.
- Do not add configuration, shell integration, previews, or further selector features; simplify the Rust implementation instead of transliterating C.
- Keep the upstream checkout at `~/d/fzy` read-only and use it as the behavioral oracle.
- Preserve byte-based matching and platform-specific floating-point ordering, including one-ULP differences.
- Keep `rustix` for terminal/event access, `libc` on Linux for signal masks and on macOS for the blocked SIGWINCH mask and sigwait wake, and the existing benchmark dependency; ask before adding others.
- Keep the upstream MIT notice in `LICENSE`.

## Topics

- Matching, ranking, terminal behavior, and invariants: [docs/architecture.md](docs/architecture.md)
- Test suites and the upstream oracle: [docs/testing.md](docs/testing.md)
- Toolchain and release workflow: [docs/release.md](docs/release.md)
- Benchmarks and performance measurements: [benches/README.md](benches/README.md)

## Verification

- `cpu-guard --cores 6 -- make test`: Ground cold/warm 15.65s/6.99s; core Tests slice warm median 12.17s with `RUST_TEST_THREADS=2` and the pinned fzy oracle (five pairs; baseline 10.95s). Python 3 required.
- `cpu-guard --cores 6 -- make lint`: 2.34s (macOS arm64, warm target).
- CLI flags, byte input, filter output, errors and exit codes: `tests/cli.rs`; interactive keys, terminal output, cancellation and multi-select: `tests/acceptance.py`, `tests/default_tty.rs`.
- Differential CLI and interactive oracle cases in `tests/acceptance.py` skip locally without fzy; PR CI builds the pinned oracle; `benches/test_perf.py` is not in `make test`.
- Public matcher, scoring, rank and Choices API: `tests/core.rs`; Candidates: `tests/candidates.rs`; Picker: `tests/picker.rs`. CLI `run`: `tests/cli.rs`; foreground handoff, pending input and worker help: `tests/foreground.rs` and `tests/foreground_probe.py`.
- Fast Rust API tiers cover scorer/candidates/ranking; real CLI and PTY process tiers cover flags, errors, foreground ownership and cleanup. PTY acceptance dominates elapsed time; see `docs/deslop-tests.md`.
- Persistent file formats, wire formats, endpoints: none.
- Performance: `benches/bench.rs` and `benches/perf.py` cover matcher, filtering and terminal redraw; see `benches/README.md` for workloads and prerequisites.
- Library consumer: ish vendors fz; run ish tests against the candidate for affected API changes.
- PTY selector lifetime: `tests/test_pty_session.py` kills the driver with TERM and KILL and verifies that a selector ignoring TERM and HUP is reaped. Both PTY harnesses use `tests/pty_session.py`.
