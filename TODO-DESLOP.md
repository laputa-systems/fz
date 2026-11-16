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

## Tickets




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


### DSLP-8 · P2 · Mutation timeouts leave selector processes running

- **Area:** Tests · process lifetime
- **Where:** tests/acceptance.py:157-160, tests/acceptance.rs:31-45
- **Problem:** During the DSLP-4 cargo-mutants run, 30-second timeouts left mutated fz processes reparented to PID 1 and consuming CPU. For example, the src/lib.rs and src/cli.rs run with --timeout 30 --jobs 4 produced orphaned processes under .work/tmp/cargo-mutants-tests-*/target/debug/fz; the PTY harness starts each selector in a new session. This contaminates mutation timing and can compromise later checks.
- **Recommendation:** Make the acceptance process tree terminate and be reaped when the mutation runner times out, including selectors in separate sessions; demonstrate that a timed-out mutant leaves no live descendants before repeating the mutation proof.
- **Fix with:** test-audit
- **Effort:** M


### DSLP-9 · P2 · PTY name lookup causes false mutation catches on macOS

- **Area:** Tests · PTY isolation
- **Where:** tests/acceptance.py:221, .work/mutants-final-a1/mutants.out/log/src__score.rs_line_102_col_35.log
- **Problem:** During the four-job scoring mutation run, two acceptance cases failed at os.ttyname(self.slave) with OSError errno 34, unrelated to the mutations. One mutation was in a Linux-only branch, so its reported caught result was false on macOS. This intermittent failure can make mutation results unreliable and fail ordinary tests under PTY churn; the cause of the device-name lookup failure is not yet established.
- **Recommendation:** Reproduce the PTY lookup failure under the supported macOS test workload, establish its cause, and make the harness deterministic without retrying away errors; verify that unrelated PTY setup failures cannot count as mutation catches.
- **Fix with:** root-cause
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

