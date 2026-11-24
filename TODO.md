Prefix: FUZ
Next: 5

# Open work
### FUZ-3 · P2 · Resolve the consumer input framing test stall

- **Area:** Consumer validation
- **Where:** https://github.com/joshuarli/ish/issues/29; ish src/input.rs:1107
- **Problem:** An ish full consumer test timed out waiting for a sentinel in the parallel input framing scenario; its cause and any connection to fz are unproven, and the upstream consumer issue remains open.
- **Recommendation:** Resolve the tracked ish failure and rerun the full consumer suite before relying on a fresh full-suite qualification.
- **Fix with:** root-cause
- **Effort:** M

### FUZ-1 · P3 · Measure regressions against a same-version baseline

- **Area:** Benchmarks · regression detection
- **Where:** benches/perf.py; benches/RESULTS.md; benches/budget-macos-aarch64.json
- **Problem:** The process ratio compares fz with fzy rather than a previous fz build, while the Rust budget is a same-host reference without an integrated candidate check. A shared slowdown or a change within the fzy ratio can escape regression detection.
- **Recommendation:** Add a reproducible interleaved baseline-versus-candidate process comparison with host qualification and an explicit uncertainty rule; assess the existing Rust budget on its intended host.
- **Fix with:** bench-pyramid
- **Effort:** M

### FUZ-2 · P3 · Attribute process and worker performance costs

- **Area:** Benchmarks · attribution
- **Where:** benches/bench.rs; benches/perf.py; benches/README.md
- **Problem:** Parallel Rust allocation measurements omit worker allocations, and process CPU and RSS totals do not identify hot code, allocation sites or syscall and I/O causes. The synthetic path fixtures also cannot establish performance on a real user input distribution.
- **Recommendation:** Capture a representative anonymized input and document separate sampling and allocation or syscall profiling recipes for the CLI and workers, with profiling overhead excluded from timed runs.
- **Fix with:** bench-pyramid
- **Effort:** M

### FUZ-4 · P4 · Exercise the large input offset transition

- **Area:** Candidate storage testing
- **Where:** src/lib.rs:245; tests/candidates.rs
- **Problem:** Inputs larger than 4 GiB select wide offsets, but the input-length transition has not been exercised with an actual large input; from_records covers the wide representation without testing that transition.
- **Recommendation:** Add a resource-aware boundary check for the actual input-length transition while preserving coverage of candidate contents.
- **Fix with:** none
- **Effort:** M

