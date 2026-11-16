Prefix: DSLP
Next: 10

Deslop campaign backlog ([plan](https://github.com/laputa-systems/stuff/blob/main/docs/campaigns/deslop.md), tier A). One agent works this file with the `deslop` skill: take the lowest-numbered open ticket whose area does not start with `Needs owner ·`. After DSLP-5, work the open P1 and P2 findings, then fill in `## Report` below.

## Report

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

Tests: [qualification report](docs/deslop-tests.md). The combined core, terminal and signal lane selects 448 mutants: 432 caught, 16 unviable, zero missed or timed out, with exact survivor dispositions and no accepted harness failures. Native macOS line/branch coverage is 90.94%/77.49%. Five interleaved warm test pairs satisfy the cost budget; five release terminal workload pairs satisfy the runtime gate. Linux and macOS CI provision the pinned fzy oracle. Ish consumer tests load merged candidate `17d0e665` and pass, with the documented manifest-only fixture adjustment. DSLP-5 remains open.

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
smoke pass. Linux and macOS CI remain required before merge. Shipped Rust,
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

## Tickets
