# fz

`fz` is a minimal Rust port of fzy that preserves its observable command-line behavior on macOS and Linux.

- Build: `cargo build`
- Test: `cargo test`
- Check: `cargo clippy --all-targets --all-features -- --deny warnings`

## Hard invariants

- Preserve fzy's matching, ranking, key bindings, exit status, and stdin/stdout behavior.
- Do not add configuration, shell integration, previews, or multi-select; simplify the Rust implementation instead of transliterating C.
- Keep the upstream checkout at `~/d/fzy` read-only and use it as the behavioral oracle.
- Preserve byte-based matching and platform-specific floating-point ordering, including one-ULP differences.
- Keep `rustix` for terminal/event access, `libc` on Linux for signal masks, and the existing benchmark dependency; ask before adding others.
- Keep the upstream MIT notice in `LICENSE`.

## Topics

- Matching, ranking, terminal behavior, and invariants: [docs/architecture.md](docs/architecture.md)
- Test suites and the upstream oracle: [docs/testing.md](docs/testing.md)
- Toolchain and release workflow: [docs/release.md](docs/release.md)
- Benchmarks and performance measurements: [benches/README.md](benches/README.md)
