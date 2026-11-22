# Benchmarks

The benchmark suite has an in-process matcher/parser suite and a release-process comparison. The measured acceptance record is in [RESULTS.md](RESULTS.md).

## Entrypoint

`make bench-list` lists the named workloads, tiers, areas and estimated cost.
`make bench-check` checks prerequisites for the default Edit tier; `make bench`
runs it. Pass options with `BENCH_ARGS`, for example:

```sh
make bench BENCH_ARGS='--tier change --name rust-search'
make bench BENCH_ARGS='--tier change --area process --dry-run'
make bench-check BENCH_ARGS='--tier scenario --skip process-quick'
```

Tiers are ceilings: Edit tests the harness; Change runs a one-sample Rust
search and a one-repetition process smoke; Scenario adds the individually named Rust rows and
three-repetition process comparisons. Full adds the five-repetition process
suite and is manual-only: explicitly select `--tier full` to run it. Names,
areas and skips are repeatable; names and skips take workload names, areas
take area names. Unknown or empty selections fail before work starts. Dry
runs check prerequisites and print commands without writing results.
A Rust workload requires a measurement for its exact benchmark name, with
at least one sample, enough iterations to account for the samples, and a median.

Rust workloads require Cargo and the pinned toolchain. Process workloads require
`cargo build --release` and the pinned fzy checkout built at `.work/tmp/fzy`
following [the testing guide](../docs/testing.md); `--fz`, `--fzy` and
`--oracle` override paths. Preflight warns about stale fz builds. Runs write
logs, process JSON, fixture data and a summary under dated `.work/bench/`
directories. The summary records exclusions, status, duration and host readiness;
completion without a budget check is not a performance verdict. The readiness
check does not prevent exploratory measurements, but unqualified timing is
inconclusive. No workload is a replacement for a paired regression gate.

## In-process measurements

Run Rustybench with:

```sh
cargo bench --bench bench -- --format json
```

The suite in `bench.rs` measures matching, scoring, highlights, candidate parsing, and `Choices` searches. Its fixtures cover small and 100,000-record inputs, plus candidates around the 1024-byte scoring boundary.

`Choices` defaults to one worker; the CLI requests automatic sizing, capped at four. The parallel benchmark rows opt into that CLI setting. Rustybench's allocation profiler is thread-local, so parallel rows include worker time but omit allocations made by spawned workers. Treat those allocation fields as caller-thread diagnostics. The process measurements below include the complete CLI process.

## Release-process comparison

`perf.py` compares release binaries against upstream fzy. It checks filtered output before timing, measures process CPU time and peak RSS, and uses a PTY for terminal redraw latency and terminal-restoration checks. It requires `target/release/fz` and `target/upstream/fzy` by default; the paths can be overridden with `--fz` and `--fzy`.

Build the upstream oracle as described in [the testing guide](../docs/testing.md), then run:

```sh
cargo build --release
mkdir -p .work/tmp/perf-data
python3 benches/perf.py --smoke --fz target/release/fz --fzy .work/tmp/fzy/fzy --data-dir .work/tmp/perf-data
python3 benches/perf.py --quick --fz target/release/fz --fzy .work/tmp/fzy/fzy --data-dir .work/tmp/perf-data
python3 benches/perf.py --check-ratio 1.1 --json .work/tmp/process-perf.json --fz target/release/fz --fzy .work/tmp/fzy/fzy --data-dir .work/tmp/perf-data
```

Smoke mode uses a small fixture and one measured run. Quick mode keeps all workloads and uses three measured runs; the default uses one warmup and five measured runs. The default and serial (`-j1`) modes are reported separately. `--check-ratio` fails when any measured fz/fzy median ratio exceeds its positive limit.

The accepted in-process budget is stored in [`budget-macos-aarch64.json`](budget-macos-aarch64.json) for its recorded macOS arm64 host. This repository does not include a candidate-budget checker or run that budget in the release workflow; the file is a reference report. The release-process comparison is separate and runs through `perf.py`.

Run the process harness's own tests with:

```sh
python3 -m unittest benches/test_perf.py
```
