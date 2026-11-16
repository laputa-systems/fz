# Tests qualification

The mutation lane is qualified for `src/score.rs`, `src/lib.rs`, `src/cli.rs`,
`src/terminal.rs` and `src/termination.rs`. The terminal slice merged as
`17d0e665a6ed881765adaffa04957f12251d3e82`. Its restoration loop delegates
to Rustix's existing EINTR helper; public contracts are unchanged. DSLP-4
is complete, including the subsequent consumer and terminal workload checks.

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
Terminal and signal decision regions have the dispositions below.

## Terminal and signal follow-up

The first Linux extension at `0973191` selected 546 mutants: 422 caught,
106 missed, 18 unviable, no timeouts. Fifteen were test helpers because the
tool does not recognize a combined `cfg(all(test, target_os = "linux"))`.
Splitting the attributes preserves compiled behavior and removes them from
production discovery. At `d499615`, the 531 production decisions produced
427 caught, 88 missed, 16 unviable and no timeouts in
[run 43856529550](https://github.com/laputa-systems/fz/actions/runs/43856529550).

The remaining survivor dispositions are exact entries in `.cargo/mutants.toml`:
64 macOS decisions are unobservable in the Linux lane and remain covered by
native macOS acceptance and foreground probes. Eleven changed sentinels
cannot differ for valid fixed signals and initialized ABI records; three
OR/XOR flag mutations pass identical disjoint bits. Two signalfd guards
cannot differ under the [Linux record and readiness contract](https://man7.org/linux/man-pages/man2/signalfd.2.html).
The signal preconditions follow [sigsetops](https://man7.org/linux/man-pages/man3/sigsetops.3.html)
and [sigaction](https://man7.org/linux/man-pages/man2/sigaction.2.html).
One redundant field destructor follows Terminal's completed restoration.
One private display body is diagnostic glue; the CLI downcasts that error
into signal status without printing it. No observable behavior is excluded
as an owner-accepted risk.

New public-boundary probes check exact caller signal masks and dispositions,
two foreground calls in one process, ignored HUP, termination delivered on
a helper thread, pending-file EOF, real TTY EINTR/EIO, signalfd refusal,
resize during an unexpired arrow sequence and fallback geometry when one
terminal dimension is zero. A disposable Linux syscall filter refuses the
readiness poll with ENOMEM: Rustix's raw backend bypasses C interceptors,
so a libc interposition alone was an invalid probe. Failure to install the
filter emits the infrastructure marker. The shipped backend is unchanged.
Terminal restoration uses Rustix's existing `retry_on_intr` contract instead
of duplicating its EINTR loop; cleanup still runs for every guard afterward.

Restored negative controls each fail the intended new assertion: an early
Escape cancellation after resize, zero-size geometry, ignoring poll errors,
and replacing the ignored-HUP predicate. The HUP fault exits 129 instead of
selecting `beta`. Native macOS `make test lint mutants-selftest` passes with
the pinned fzy oracle; native Linux foreground and acceptance pass, with
the two differential cases exercised in the pinned-oracle CI jobs. The first final-scope run selected
448 and reported 432 caught, zero missed, 16 unviable and no timeouts, but
the gate rejected its counts: the supervisor tried to install a handler for
SIGKILL when cleanup killed a faulty selector. That raised EINVAL and the
infrastructure marker. A new supervisor self-test reproduces the incorrect
exit 1 before the fix and the propagated SIGKILL afterward. The gate's
rejection is retained; those catch counts are not accepted evidence.

The next run was also rejected: three resize/arrow scenarios mislabeled a
selector's real diagnostic as a missed delivery window. Late attempts now
must still satisfy the complete bare-Escape cancellation contract. A hand
fault disabling the signalfd guard fails on the actual diagnostic without
an infrastructure marker. No rejected run establishes qualification.

The [final run](https://github.com/laputa-systems/fz/actions/runs/43870572243)
at reviewed head `e70ae7820768be56c91fe1ce14a42326454b6854` selected 448:
432 caught, zero missed, 16 unviable, zero timeouts, with no infrastructure
markers. Gate wall time was 874.6 seconds within its 1,200-second budget.
The configuration SHA-256 was
`af91a513b5b689bb227738d7dc2c74bcbe772e011d0ee57cdec6ca78d5e4776d`.
The 103 exact exclusions comprise 64 macOS decisions, 38 equivalents and
one private diagnostic body. All Linux, macOS, lint and mutation checks
passed on that head before PR #37 merged.

Consumer validation uses an isolated ish `ff70621f` clone and its importer,
with `vendor/fz/UPSTREAM` resolving the merged candidate `17d0e665`.
`cpu-guard --cores 6 -- make test` passes, including the real worker and
PTY scenarios; existing ignored tests remain ignored. The importer retains
explicit upstream test targets without copying their files. Removing only
that unavailable manifest target in the fixture allows Cargo to load the
exact candidate library sources. The importer defect is now repaired by
[ish PR #30](https://github.com/joshuarli/ish/pull/30), merged as
`3930ca270adf3a9aea46c4a7b1af092b74e2ca8a`: library-only imports remove
explicit test targets whose files they do not copy. Its disposable Cargo
regression fails before the repair and passes afterward.

The repaired importer also loads final fz `60fc1e95` without a manifest
adjustment. All four ish worker scenarios and workspace Clippy pass with
that imported source. A fresh full consumer run passed 320 library tests
but timed out in the unchanged input framing test at `src/input.rs:1107`;
[ish issue #29](https://github.com/joshuarli/ish/issues/29) records that
failure and its unproven cause. It is not a passing full run. The earlier
full consumer proof at `17d0e665` remains valid for shipped Rust, which is
identical at `60fc1e95`; later fz changes remove only an unread benchmark
field and rename unused Python callback parameters.

An initial busy-host gate deferred runtime measurement. After the host gate
passed, five interleaved baseline/candidate pairs ran the declared terminal
workload on macOS arm64, default features, identical lockfiles and release
profiles. Each side used `python3 benches/perf.py --interactive-only`, one
warmup and five repetitions, the pinned fzy oracle and the 100k-path fixture.
Every side passed `perf-gate-ready` before and `perf-gate-ready -1` after.
The baseline is core head `2157b68`; the candidate is merged `17d0e665`.

Each cell is the median of five run medians, with their range, in milliseconds.

| Mode / redraw | Before | After |
| --- | ---: | ---: |
| default / dense | 2.769 (2.748–3.113) | 2.771 (2.734–2.820) |
| default / selective | 0.805 (0.778–0.922) | 0.792 (0.782–0.812) |
| default / no match | 0.326 (0.315–0.477) | 0.320 (0.314–0.334) |
| default / exact | 0.846 (0.828–0.871) | 0.819 (0.806–0.861) |
| default / longer | 0.824 (0.820–0.966) | 0.818 (0.807–0.827) |
| serial / dense | 9.982 (9.793–10.286) | 9.974 (9.965–10.238) |
| serial / selective | 2.674 (2.589–2.742) | 2.663 (2.634–2.793) |
| serial / no match | 0.899 (0.872–0.914) | 0.910 (0.887–0.932) |
| serial / exact | 2.744 (2.644–2.865) | 2.727 (2.702–2.807) |
| serial / longer | 2.713 (2.644–2.841) | 2.740 (2.708–2.794) |

All latency ranges overlap; no regression exceeds both the observed spread
and 2%. Peak RSS medians are 9,732,096 bytes on both sides in default mode;
serial rises from 8,880,128 to 8,896,512 bytes (0.18%). Release size is
600,144 bytes before and 583,632 after; no size growth. This check covers
the terminal-restoration path; unchanged matcher/parser workloads are unaffected.

After-coverage on native macOS is 90.94% lines and 77.49% branches, against
89.63% and 75.92% before this slice. Five interleaved warm `make test`
pairs, with the pinned fzy oracle and two Rust test threads, have medians
12.155 seconds before and 13.116 after; the increase is inside the 20% or
30-second budget. The host gate returned zero before and after. This cost
and coverage evidence precedes the final SIGKILL supervisor regression;
that small additional scenario is verified separately.

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
