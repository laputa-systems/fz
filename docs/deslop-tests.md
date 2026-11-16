# Tests qualification

The core mutation lane is qualified for `src/score.rs`, `src/lib.rs` and
`src/cli.rs`. Production Rust is unchanged from main `0c6e74b`. Terminal
and termination mutation qualification remains in DSLP-4; these modules
contain another 160 and 31 mutants and belong in the next bounded slice.

## Measurements

Both CI runs use cargo-mutants 27.1.0 on Ubuntu 26.04 arm64, the pinned
nightly, default features, four mutation jobs and `RUST_TEST_THREADS=2`.
The command is `make mutants-diff BASE=origin/main`: baseline runs,
40-second test timeout, 300-second build timeout and a 1,200-second budget.
The config is `.cargo/mutants-gate.toml`; exact exclusions and their proofs
are in `.cargo/mutants.toml`.

| Run | Selected | Caught | Missed | Unviable | Timeout |
| --- | ---: | ---: | ---: | ---: | ---: |
| Before boundary tests, `79b9833` | 355 | 321 | 27 | 7 | 0 |
| Candidate, `45b051a` | 355 | 348 | 0 | 7 | 0 |

The [before run](https://github.com/laputa-systems/fz/actions/runs/43840923418)
uses main production code and functional assertions, with the corrected PTY
infrastructure backported. Its mutation job took 9m22s. An earlier baseline
with the old PTY harness was cancelled and discarded. The
[candidate run](https://github.com/laputa-systems/fz/actions/runs/43836406752)
took 10m40s including job setup and artifact upload. Both listings use the
same 21 exact exclusions. There are no accepted-risk exclusions.

The 27 before survivors cover worker limits and dispatch thresholds,
partial ranking, empty and out-of-range access, allocation size, long CLI
options, interrupted and failed input, worker help and foreground entry,
and exact 1,024-byte scoring. Each is caught by the candidate. The gate
retains per-mutant logs as the seven-day `mutation-evidence` artifact.
The candidate logs contain no PTY infrastructure marker. The sole broken
pipe occurs with a mutated `--workers` parser and accompanies an independent
CLI output assertion; it is causally related to that mutant.

Mac arm64 coverage uses `cpu-guard --cores 6 -- make coverage`, with the
pinned fzy oracle. Line coverage rises from 87.35% to 89.63%; branch coverage
from 73.53% to 75.92%. Scoring is 100% lines and 94.59% branches on both
sides. Coverage is a report, not a gate.

Five interleaved warm `make test` pairs used the same oracle, six-core
cap and `RUST_TEST_THREADS=2`. `perf-gate-ready -1` exited zero before and
after the collection. Baseline median was 10.95s (10.71–11.08s); candidate
12.17s (12.15–12.45s). The 1.21s increase fits the 20% or 30s budget.
PTY scenarios dominate elapsed time: one candidate pair's acceptance
binary took 10.97s, while each reported Rust API suite took at most 0.24s.
The foreground worker probe is a process scenario, not a micro-unit test.
All timed test exits were zero. No runtime speedup or regression is claimed.

## Boundaries and witnesses

The Rust API tests assert byte preservation, source identity, finite and
infinite scores, worker equivalence and partially ranked navigation. The
CLI tests check flags, output, errors and resource bounds. PTY scenarios
check real process and terminal behavior. CI builds fzy revision
`34b88869d022e861da4846c4463aea3ddfb3ff30` before both Linux and macOS
tests, so both existing differential cases run.

The foreground probe invokes the public `run_in_foreground` entry from a
background process group. It covers pending input, accept, cancel, serial
help, foreground-group restoration and exact terminal attributes. On Darwin,
FIONREAD processes the kernel's pending canonical input before attributes
are compared; the comparison does not mask flags.

Hand faults were restored after these checks:

| Fault | Detecting behavior |
| --- | --- |
| Skip foreground claiming in `run_in_foreground` | Real worker selection fails; restored native probe passes |
| Select the pivot two positions too far into the unsorted tail | Seed-24 navigation returns the wrong source index |
| Reject candidates of exactly 1,024 bytes | Finite-score assertion reports negative infinity |
| Reserve `input_length * 48` offsets | One-MiB Linux CLI scenario exceeds its 128-MiB address-space bound |
| Inject stdin errors only at fd 0, missing the duplicated fd | Native EIO scenario reports success instead of the required error |

Linux-only branches have real CI scenarios for EINTR, EIO and actual thread
creation. The Mac coverage report does not cover those arms. The >4-GiB
input-length switch is not allocated in this bounded scenario; the wide
offset representation is exercised through `from_records`, and the size
switch uses checked standard-library conversion. The remaining unrun score
branches are impossible finite-alignment states or equivalent flag and
backtrace operations, documented individually in the exclusion config.
Terminal and signal decision regions await the next DSLP-4 slice.

## Harness and gate controls

The PTY name-lookup fault fails before the controlling-terminal fix and
passes afterward. It forces `ttyname` to raise errno 34. The kernel's
intermittent lookup error did not reproduce in 20,000 allocations; its
kernel cause is unclaimed. Driver TERM and KILL tests verify that the
supervisor kills and reaps an otherwise uncooperative selector.

A forced SIGALRM after a successful Linux PTY read reproduced lost screen
bytes. Removing the Darwin-only alarm workaround from Linux fixes that
regression. The Darwin workaround remains. Error-injection input is supplied
before child launch, so expected EIO cannot race a parent pipe write.

The 11 gate self-tests cover whole-change union selection, docs-only passes,
deletion handling, missing evidence, infrastructure markers and result counts.
The [CI negative control](https://github.com/laputa-systems/fz/pull/35) at
`fda7f09` deletes `tests/core.rs` and adds an unasserted scoring branch.
Ordinary Linux/macOS CI and lint pass; mutations fail with 41 missed of 364.
Both arithmetic mutations inside the new branch survive. The first control's
bare numeric literal generated no body mutant, so it established only the
deletion control; it is not the branch proof. The control PR is closed and
will never merge. Its count differs because the branch adds three mutants
and moves six exact line-bound exclusions out of its deliberately changed
file; neither control config is copied back to the candidate.
