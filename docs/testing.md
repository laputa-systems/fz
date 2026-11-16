# Testing

Run the Rust suites and the Python PTY acceptance harness with:

```sh
cargo test
```

Python 3 is required for the acceptance harness. The Rust tests cover scoring, candidate parsing, ranking, worker behavior, and CLI output. The acceptance harness covers terminal interaction. Its differential cases run when `FZY_ORACLE`, `target/upstream/fzy`, `~/d/fzy/fzy`, or `~/d/fzy/build/fzy` is an executable. Otherwise they skip.

To run the differential cases, build the pinned upstream fzy revision in a disposable copy. The repository's `~/d/fzy` checkout remains read-only:

```sh
mkdir -p .work/tmp
git clone https://github.com/jhawthorn/fzy.git .work/tmp/fzy
git -C .work/tmp/fzy checkout --detach 34b88869d022e861da4846c4463aea3ddfb3ff30
make -C .work/tmp/fzy fzy test
FZY_ORACLE="$PWD/.work/tmp/fzy/fzy" cargo test
```

The Python harness is `tests/acceptance.py`; `tests/acceptance.rs` runs it during `cargo test`. `tests/cli.rs`, `tests/candidates.rs`, and `tests/core.rs` cover the CLI, candidate storage, and core matcher and ranking behavior.

CI builds the pinned fzy oracle before `make test`, so both differential cases
run there. The foreground probe calls `run_in_foreground` from a background
worker. It checks selection, cancellation, pending input, serial help,
foreground-group restoration and terminal attributes.

Linux CLI tests also need a C compiler. They compile two small interposers
under `.work/tmp` to inject stdin read errors and count actual thread creation.
Only the spawned test binary loads those libraries through `LD_PRELOAD`.

`make coverage` reports line and branch coverage. Coverage is not a gate.
`make mutants-selftest` checks the mutation gate's selection and result parser.
`make mutants-diff BASE=origin/main` runs the Linux mutation lane and requires
Python 3.11 or newer. The gate selects changed production lines together with
the configured critical scope when tests, fixtures or build configuration change.
It rejects incomplete results, unresolved timeouts and infrastructure markers
from the PTY harness. The critical scope is scoring, candidate storage, ranking
and CLI parsing. The core lane is qualified; see [the evidence](deslop-tests.md). Terminal
and signal mutation qualification remains in DSLP-4.
