# Release

The repository pins its compiler in [`rust-toolchain.toml`](../rust-toolchain.toml). It selects a nightly toolchain with `rust-src`, `llvm-tools`, and `clippy`. The standard Cargo release build uses the prebuilt standard library; release artifacts rebuild it with the pinned toolchain.

## Local and workflow builds

Build the host release binary with `cargo build --release`. On macOS arm64, the release artifact build and verification are:

```sh
make release TARGET=aarch64-apple-darwin
make verify-release TARGET=aarch64-apple-darwin
```

The release workflow runs the Linux Makefile targets inside the builder defined in [`Dockerfile`](../Dockerfile): `make release` and `make verify-release` for static binaries, and `make release-dynamic` and `make verify-release-dynamic` for dynamic binaries. Static verification checks that a binary has no dynamic interpreter or dependencies; dynamic verification checks for the musl loader and dependencies.

## Published artifacts

The manual [`release` workflow](../.github/workflows/release.yml) builds and tests each target in the published matrix, verifies both Linux link modes, and creates ZIP archives on a prerelease tagged `release-<commit-sha>`. Linux archives contain both dynamic and static builds. See the user-facing [installation instructions](../README.md) for the target matrix and archive names.
