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

Core Tests slice: [qualification report](docs/deslop-tests.md). The 27 baseline survivors are caught; line/branch coverage is 89.63%/75.92% on macOS. Five interleaved warm pairs remain within the test-cost budget. Linux and macOS CI provision the pinned fzy oracle. Foreground worker selection/restoration and Linux input/resource boundaries have real process probes. The remaining Tests component is terminal/signal mutation qualification in DSLP-4.

## Tickets




### DSLP-4 · P2 · Tests: qualify terminal lifecycle mutation coverage

- **Area:** deslop · tests
- **Where:** src/terminal.rs; src/termination.rs; tests/foreground_probe.py; .cargo/mutants-gate.toml
- **Problem:** The core Tests slice is qualified: 355 selected mutants, 348 caught, 7 unviable, zero missed or timed out; before/after coverage and test-cost checks pass. Terminal ownership, poll/read error paths and signal cleanup remain outside the current configured scope. They contain 160 terminal and 31 termination mutants and must receive their own survivor dispositions before the Tests step is complete. See docs/deslop-tests.md.
- **Recommendation:** Extend the bounded critical scope to terminal ownership and signal lifecycle, run its before/after Scenario evidence, disposition every survivor and unrun decision region, and qualify the combined lane within its budget (shard if measured cost requires it). Accept: make test/lint/mutants-selftest pass, handoff/cleanup faults are caught, and no undispositioned survivor remains in that scope. Production changes also require ish consumer and performance gates.
- **Fix with:** deslop
- **Effort:** M

### DSLP-5 · P2 · Dead code: tools and debloat

- **Area:** deslop · dead code
- **Where:** whole repository
- **Problem:** No dead-code scan has run.
- **Recommendation:** Run the deslop Dead code step. Accept: tools clean or allowlisted with reasons; walk-backs filed as Needs owner tickets.
- **Fix with:** deslop
- **Effort:** M
