Prefix: DSLP
Next: 13

Deslop campaign backlog ([plan](https://github.com/laputa-systems/stuff/blob/main/docs/campaigns/deslop.md), tier A). One agent works this file with the `deslop` skill: take the lowest-numbered open ticket whose area does not start with `Needs owner ·`. After DSLP-5, work the open P1 and P2 findings, then fill in `## Report` below.

## Report

Bench audit: Rustybench's `has_match`, `score`, and `positions` rows isolate
matching, ranking primitives and highlights on paths and the 1024-byte boundary;
`candidates_parse` covers input parsing at small, numeric, path and long sizes;
`choices_search` covers dense, selective, no-match, long and parallel searches;
the interactive Choices sequence covers refinement and edits. These synthetic
rows consume results but do not independently assert them. Process filter rows
cover small, numeric, path and long inputs, default and serial modes, checking
complete output against fzy before timing. PTY rows check ranked redraws and
terminal restoration, and measure redraw and process RSS. CPU is per child;
parallel Rust allocation data omits worker allocations. The process medians use
interleaved fz/fzy runs, five samples by default; the accepted 1.1 ratio is
relative to fzy, not an earlier fz. The Rust budget is a same-host reference,
not an integrated gate. Fixture digests and binary hashes are recorded by the
process harness; Rust runs lack an equivalent correctness oracle and artifact
identity. Host readiness must qualify timing; a busy run is inconclusive.

Ranked gaps: DSLP-11 (P3) covers missed same-version regressions and the
unintegrated budget; DSLP-12 (P3) covers representative inputs and attribution
using separate CPU sampling, allocation and syscall profiling. Current data
cannot resolve those questions. Sampling and tracing add overhead and must stay
outside timed runs; no profiling overhead was measured in this audit.


Ground base: `1a42f7c3bebecdd63331524366492252ccf23870`; macOS arm64, fresh target: `make test` 15.65s cold, 6.99s warm; `make lint` 2.34s, all exit 0 under `cpu-guard --cores 6`. The acceptance harness ran 24 cases, skipping two differential oracle cases because fzy was absent; PR CI does not provision an oracle, and `benches/test_perf.py` is outside `make test`. Python comprises test harnesses and benchmark tooling, not shipped runtime. No vendored trees in this repository; ish vendors fz with its own UPSTREAM. Performance workloads: `benches/bench.rs` and `benches/perf.py`; see `benches/README.md`. CI changed from `cargo test` to `make test`. No runtime changes; consumer and performance gates unaffected.

Types (DSLP-3): Rust escape-hatch matching lines 90 → 90; Python 0 → 0. Candidate counts below are unique source locations on macOS, with test-only unwrap/expect excluded; real = demonstrated defect or safely deleted escape hatch, FP = acceptable code flagged. No setting adopted.

| Setting | Findings | Real | FP | Decision |
| --- | ---: | ---: | ---: | --- |
| unsafe_code forbid | 51 | 0 | 51 | reject: platform FFI and entry point |
| wildcard_enum_match_arm | 0 | 0 | 0 | reject: no witness |
| match_wildcard_for_single_variants | 0 | 0 | 0 | reject: no witness |
| cast_possible_truncation | 12 | 0 | 12 | reject: wake token and test/benchmark generators |
| cast_sign_loss | 1 | 0 | 1 | reject: wake token |
| unwrap_used outside tests | 5 | 0 | 5 | reject: internal invariants and thread panic propagation |
| expect_used outside tests | 4 | 0 | 4 | reject: internal invariants |
| redundant_clone | 0 | 0 | 0 | reject: no witness |
| needless_pass_by_value | 2 | 0 | 2 | reject: ownership handoff and local row value |

No contracts changed; witnesses not applicable. macOS lint and test passed; two fzy oracle cases skipped. Linux-specific lint paths were not measured on this host.

Tests: [qualification report](docs/deslop-tests.md). The combined core, terminal and signal lane selects 448 mutants: 432 caught, 16 unviable, zero missed or timed out, with exact survivor dispositions and no accepted harness failures. Native macOS line/branch coverage is 90.94%/77.49%. Five interleaved warm test pairs satisfy the cost budget; five release terminal workload pairs satisfy the runtime gate. Linux and macOS CI provision the pinned fzy oracle. Ish consumer validation and its subsequently repaired importer are documented in that report. DSLP-4 and DSLP-5 are complete.

Dead code: Vulture 2.16 identified one unused `Dataset.description` field and
its four constructor values in `benches/perf.py`; these five lines are removed.
Repository callers, benchmark tests, CLI output/JSON serialization, docs,
manifests, workflows and ish's source-only vendor importer have no consumer
of that field. `dataclasses.asdict` serializes `Sample`, not `Dataset`.
Unused signal-frame and classmethod-receiver names are marked with underscores;
their callbacks and unittest hooks remain. Vulture's two named allowlist
entries are unittest-discovered `Acceptance` and `SessionLifetime`, with
reasons in `pyproject.toml`. Use explicit `benches`, `scripts`, `tests` roots:
excluding `.work` against an absolute worktree path would hide the whole scan.

`cargo machete`, Rust warnings-denied checks and Ruff F401/F841 are clean;
Vulture is clean with those two entries. Native macOS `make test lint
mutants-selftest`, the benchmark harness's three tests and release-binary
smoke pass. Linux, macOS, lint and mutation CI passed at reviewed head
`1121fa923` before [PR #39](https://github.com/laputa-systems/fz/pull/39)
merged as `60fc1e95dc166c98720c36bab5bb61a0d381e696`. Shipped Rust,
public APIs and dependencies are unchanged; consumer and runtime performance
checks are unaffected because this field is never read by the workload.

Debloat covered CLI parsing/dispatch, scoring and candidate/rank/picker layers,
OS terminal/signal ownership, worker dispatch, tests, benchmarks, dependencies,
CI and documentation. Existing layers serve distinct callers or real OS
boundaries. The supervisor prevents demonstrated orphan processes; the mutation
gate prevents demonstrated false passes; paired benchmark/oracle tooling guards
declared behavior and cost. No further structural cut was established. No live
feature, recovery path, compatibility behavior or declared gate is removed;
there are no walk-back findings.

**Final scorecard**

Ground through Dead code is complete. There are no open findings to transfer
to `TODO.md`. Scope is this repository and its existing ish consumer; fleet
rule rollout is outside this report.

| Measure | Ground | Final | Interpretation |
| --- | ---: | ---: | --- |
| Shipped Rust physical lines, `src/*.rs` | 2,654 | 2,643 | Restoration uses the existing Rustix EINTR helper |
| Test physical lines, `tests/` | 1,985 | 2,712 | Additional process, OS-error and public-boundary assertions |
| Rust escape-hatch matching lines | 90 | 118 | Added test assertions and FFI; no shipped escape hatch added |
| Python escape-hatch matching lines | 0 | 0 | Unchanged |
| Native macOS line coverage | 87.35% | 90.94% | Linux-only arms are measured by Linux scenarios |
| Native macOS branch coverage | 73.53% | 77.49% | Report only; not an acceptance threshold |
| Qualified Linux mutation survivors | 27 of 355 before core tests | 0 of 448 | Broader final scope; counts are not a matched score comparison |

Counts compare Ground `1a42f7c3bebecdd63331524366492252ccf23870` with
`60fc1e95dc166c98720c36bab5bb61a0d381e696`. Physical lines count repository
text lines, including comments and blanks. Escape-hatch counts are matching
lines for `unsafe`, allow attributes, `unwrap()` and `expect(` across tracked
Rust files, including tests. The increase is confined to test code. These
counts are structural measurements, not a claim that added tests improve
quality by their size.

Per-step costs use the measurements above and in the linked report. Ground's
single warm test was 6.99 seconds; it is not a paired performance baseline.
Types changed no production behavior or checks. The core Tests slice moved
paired warm medians from 10.95 to 12.17 seconds; terminal Tests from 12.155
to 13.116 seconds. Each meets the declared 20% or 30-second allowance. Dead
code removes an unread benchmark field and changes no timed work. The terminal
release workload's ten redraw comparisons have overlapping ranges, stable
RSS and a smaller binary; no general speedup is claimed.

Completion review inspected the cumulative production diff, survivor
dispositions and retained evidence at `60fc1e95`; it established no further
defect. It was a source/evidence review, not another execution of the gates.
The native checks and all required CI checks at that head remain the
execution evidence. This report changes documentation only.

The five seeded P2 steps (DSLP-1–5) and both discovered P2 harness findings
(DSLP-8/9) are closed, as are the two P3 boundary findings (DSLP-6/7).
No P1, accepted-risk exclusion, dependency removal or escaped shipped
regression was established. Dead code removed five unread lines and retained
all dependencies. Ground, Diagnose and Types merged in PRs #29–31; harness
repairs in #32–33; Tests and its report in #34, #37–38; Dead code in #39.
The deliberately failing gate control #35 remains closed and unmerged.
Core and terminal candidate mutation jobs cost 10m40s and 14m34.6s,
respectively, including their different reported setup boundaries; the
bounded gate itself took 874.6 seconds for the terminal candidate. Total
campaign CI minutes were not collected, so these are individual job costs.

Method feedback: qualify failure classification before counting catches;
exercise the public foreground boundary rather than only private ranking
helpers; keep platform exclusions exact and justified; make whole-change
selection reject both deleted assertions and newly uncovered branches; and
scan explicit Python roots inside nested worktrees. The ish importer defect
is fixed in its owning repository. Its separate input-decoder timeout is
recorded there with cause unproven, rather than attributed to this change.
No shared skill or fleet policy change is required by this slice.

## Current release timing

[Hosted run 44419549107](https://github.com/laputa-systems/fz/actions/runs/44419549107)
collected five complete alternating pairs from `17d0e665` and `0ec537d5`
with the pinned fzy reference. Four attempts failed a readiness sample
and were discarded in full. The accepted runs used the same native
three-core macOS ARM64 runner, four fixtures, one warmup and five timed
repetitions per side. Each side passed readiness checks before and after.

The median of the five run medians exceeded both 2% and the baseline
range width in the following rows. Times and ranges are milliseconds.

| Workload and metric | Before median (range) | After median (range) |
| --- | ---: | ---: |
| Small, longer query, default wall | 3.251 (2.788–4.629) | 5.255 (3.631–8.257) |
| Small, longer query, default CPU | 2.174 (1.974–2.868) | 3.502 (2.498–6.237) |
| Small, longer query, default user CPU | 1.155 (1.043–1.381) | 1.547 (1.259–2.626) |
| Default terminal, selective redraw | 2.474 (2.390–2.553) | 2.775 (2.369–3.146) |
| Default terminal, exact redraw | 2.529 (2.487–2.635) | 2.987 (2.459–3.102) |
| Default terminal, longer redraw | 2.534 (2.443–2.588) | 2.759 (2.411–2.998) |

All six before and after ranges overlap. The unchanged fzy reference
also rose from 3.239 to 4.306 ms in the small default longer-query wall
row. Terminal peak RSS medians decreased by 16 KiB in both modes. The
release binary was 240 bytes larger. The timings leave DSLP-10 open.

Reproduce with the [exact proof script](https://github.com/laputa-systems/fz/blob/7914f6bc66a53fab76f86b71f03c6c9d73df5b66/.github/scripts/performance-trial.sh)
and its companion workflow. Recover the ten numbered `PAIR_JSON` log
lines by decoding the base64 payload and decompressing it with gzip.
Check the four fixture hashes, the pinned fzy hash, and the two fz
binary hashes across the five pairs. Compute the median and minimum
and maximum across the five run medians for each side and workload.

[The identical-binary control](https://github.com/laputa-systems/fz/actions/runs/44421140205)
used five qualified pairs with both Fz files copied from the baseline
and kept the same fzy reference. None of the six rows above exceeded
the bound. A different row, default dense 100k-path user CPU, went
from 19.726 to 23.208 ms with a baseline range of 19.337–22.641 ms.

[The independent repeat](https://github.com/laputa-systems/fz/actions/runs/44422469442)
used five qualified pairs and the exact binary and fixture digests
from the first comparison. Across the 254 Fz measurements, no median
increase exceeded both 2% and the baseline range width. The same
small default longer-query wall row rose again. Fz went from 2.585
to 3.473 ms within the 2.157–6.387 ms baseline range. The unchanged
fzy reference went from 2.688 to 3.342 ms. The exact and selective
redraw medians decreased, and the longer redraw's increase was
within its baseline spread.

[The focused trial](https://github.com/laputa-systems/fz/actions/runs/44423775482)
ran the small default longer-query invocation with the two exact Fz
release binaries, a byte-identical copy of the baseline and the same
pinned fzy source. All five blocks passed readiness before and after.
Each block rotated the four executables through all four positions
over 40 rounds. All 800 timed invocations exited successfully. The
Fz hashes and input digest match the full comparisons. The fzy build
hash is consistent within the focused trial.

Each row below uses the median of the 40 paired percentage changes
per block, then reports the median and range across the five blocks.

| Measure | Candidate vs baseline | Identical copy vs baseline |
| --- | ---: | ---: |
| Wall time | −0.23% (−1.35 to +1.92) | +2.85% (−0.35 to +10.11) |
| Total CPU | +0.19% (−2.79 to +2.91) | +2.10% (−1.37 to +7.92) |
| User CPU | −0.63% (−3.82 to +1.13) | +2.23% (−0.54 to +7.31) |
| Peak RSS | +0.89% (+0.89 to +0.89) | 0% (0 to 0) |

The small-input rise from the two full sweeps did not recur when
the children ran consecutively. The independent full sweep meets
the existing bound. To reproduce the focused case, use the
[proof script at 8d6e375](https://github.com/laputa-systems/fz/blob/8d6e375ee41b51cec6a718ea6d94efdb388f350e/.github/scripts/performance-trial.sh)
with the same hosted job and decode the five `BLOCK_JSON` lines.

The final consumer check imported
`0ec537d53eca0a12af07687250111a48023ee8d3` with
`scripts/vendor-fz.sh` into a detached ish checkout at
`fc62468e2428f9aea6fcd611b0820ab709ce0c65`. The lockfile matched
the checked-in ish lockfile. All four worker tests and six fuzzy
PTY cases passed, along with workspace Clippy. The full `make test`
run on macOS ARM64 completed with 1,550 passes and eight ignored
cases across its top-level test reports. Reproduce with
`CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1` and
`cpu-guard --cores 3 -- make test` in the isolated checkout. The
earlier two-core and four-minute limits interrupted separate
attempts. The completed run used an eight-minute wall limit.

DSLP-10 is closed with the independent full timing repeat, the
focused paired control and the current consumer check.

## Tickets
### DSLP-11 · P3 · Measure regressions against a same-version baseline

- **Area:** Benchmarks · regression detection
- **Where:** benches/perf.py; benches/RESULTS.md; benches/budget-macos-aarch64.json
- **Problem:** The process ratio compares fz with fzy rather than a previous fz build, while the Rust budget is a same-host reference without an integrated candidate check. A shared slowdown or a change within the fzy ratio can escape regression detection.
- **Recommendation:** Add a reproducible interleaved baseline-versus-candidate process comparison with host qualification and an explicit uncertainty rule; assess the existing Rust budget on its intended host.
- **Fix with:** bench-pyramid
- **Effort:** M


### DSLP-12 · P3 · Attribute process and worker performance costs

- **Area:** Benchmarks · attribution
- **Where:** benches/bench.rs; benches/perf.py; benches/README.md
- **Problem:** Parallel Rust allocation measurements omit worker allocations, and process CPU and RSS totals do not identify hot code, allocation sites or syscall and I/O causes. The synthetic path fixtures also cannot establish performance on a real user input distribution.
- **Recommendation:** Capture a representative anonymized input and document separate sampling and allocation or syscall profiling recipes for the CLI and workers, with profiling overhead excluded from timed runs.
- **Fix with:** bench-pyramid
- **Effort:** M
