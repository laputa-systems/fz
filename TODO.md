Prefix: FUZ
Next: 3

# Open work
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

