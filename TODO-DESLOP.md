Prefix: DSLP
Next: 8

Deslop campaign backlog ([plan](https://github.com/laputa-systems/stuff/blob/main/docs/campaigns/deslop.md), tier A). One agent works this file with the `deslop` skill: take the lowest-numbered open ticket whose area does not start with `Needs owner ·`. After DSLP-5, work the open P1 and P2 findings, then fill in `## Report` below.

## Report

Ground base: `1a42f7c3bebecdd63331524366492252ccf23870`; macOS arm64, fresh target: `make test` 15.65s cold, 6.99s warm; `make lint` 2.34s, all exit 0 under `cpu-guard --cores 6`. The acceptance harness ran 24 cases, skipping two differential oracle cases because fzy was absent; PR CI does not provision an oracle, and `benches/test_perf.py` is outside `make test`. Python comprises test harnesses and benchmark tooling, not shipped runtime. No vendored trees in this repository; ish vendors fz with its own UPSTREAM. Performance workloads: `benches/bench.rs` and `benches/perf.py`; see `benches/README.md`. CI changed from `cargo test` to `make test`. No runtime changes; consumer and performance gates unaffected.

## Tickets



### DSLP-3 · P2 · Types: strict profile and typing findings

- **Area:** deslop · types
- **Where:** type checker and lint configuration; Rust/Python sources
- **Problem:** The strict-profile candidates in the deslop skill's tools reference have not been measured here.
- **Recommendation:** Run the deslop Types step. Accept: clean at the adopted profile, escape hatches not increased, witnesses recorded.
- **Fix with:** deslop
- **Effort:** M

### DSLP-4 · P2 · Tests: coverage, boundary gaps, mutation

- **Area:** deslop · tests
- **Where:** test suites and CI
- **Problem:** Coverage and mutation results are unmeasured; no deslop mutation lane exists.
- **Recommendation:** Run the deslop Tests step. Accept: no undispositioned survivor in critical code, the test cost budget holds, and the lane is green where the tools are mirrored.
- **Fix with:** deslop
- **Effort:** M

### DSLP-5 · P2 · Dead code: tools and debloat

- **Area:** deslop · dead code
- **Where:** whole repository
- **Problem:** No dead-code scan has run.
- **Recommendation:** Run the deslop Dead code step. Accept: tools clean or allowlisted with reasons; walk-backs filed as Needs owner tickets.
- **Fix with:** deslop
- **Effort:** M


### DSLP-6 · P3 · PR CI skips the fzy differential acceptance cases

- **Area:** Tests · CI
- **Where:** tests/acceptance.py:828, .github/workflows/ci.yml
- **Problem:** The acceptance harness skips its two differential cases when fzy is not installed. PR CI does not install fzy, so those cases never run there; a green CI says nothing about parity with fzy.
- **Recommendation:** Provision a pinned fzy in CI (or fail rather than skip when CI=true), and confirm in the job log that both cases ran.
- **Fix with:** test-audit
- **Effort:** S


### DSLP-7 · P3 · Cover the foreground worker terminal handoff

- **Area:** Tests · embedding
- **Where:** src/cli.rs:214, src/terminal.rs:545, src/terminal.rs:590, tests/acceptance.py:758
- **Problem:** Test-gap finding: run_in_foreground is an exported embedding entry point, but no test calls it. Its foreground process-group handoff, restoration and pending-input path are distinct from the standalone CLI exercised by acceptance tests; regressions there can break ish interactive substitutions without those tests noticing. Critical mutation modules: src/cli.rs and src/terminal.rs.
- **Recommendation:** Add a real PTY worker-boundary scenario that calls run_in_foreground, checks pending input and selection, and verifies the original foreground group and terminal state after acceptance and cancellation. Exercise worker_help's serial default at that boundary.
- **Fix with:** test-audit
- **Effort:** M

