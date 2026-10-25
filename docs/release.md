# Release

The repository pins its compiler in [`rust-toolchain.toml`](../rust-toolchain.toml). It selects a nightly toolchain with `rust-src`, `llvm-tools`, and `clippy`. The standard Cargo release build uses the prebuilt standard library; release artifacts rebuild it with the pinned toolchain.

## Local and workflow builds

Build the host release binary with `cargo build --release`. On macOS arm64, the release artifact build and verification are:

```sh
make release TARGET=aarch64-apple-darwin
make verify-release TARGET=aarch64-apple-darwin
```

The release workflow runs `make release` and `make verify-release` inside the builder defined in [`Dockerfile`](../Dockerfile). Verification rejects Linux binaries with an ELF interpreter or shared-library dependency.

## Published artifacts

The manual [`release` workflow](../.github/workflows/release.yml) builds and tests each target in the published matrix, checks Linux binaries for static linking, and creates ZIP archives on a prerelease tagged `release-<commit-sha>`. See the user-facing [installation instructions](../README.md) for the target matrix and archive names.
