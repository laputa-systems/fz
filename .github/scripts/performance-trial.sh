#!/bin/bash
set -euo pipefail

repo=$PWD
proof="$repo/.work/tmp/paired"
export CARGO_BUILD_JOBS=2 TMPDIR="$repo/.work/tmp"
mkdir -p "$proof"

build_fz() {
    local side=$1 revision=$2
    git worktree add --detach "$proof/source-$side" "$revision"
    (
        cd "$proof/source-$side"
        export CARGO_TARGET_DIR="$proof/target-$side"
        cargo build --release --locked
        cp "$CARGO_TARGET_DIR/release/fz" "$proof/fz-$side"
    )
}

build_fz before 17d0e665a6ed881765adaffa04957f12251d3e82
build_fz after 0ec537d53eca0a12af07687250111a48023ee8d3
cp "$proof/fz-before" "$proof/fz-control"
git init --quiet "$proof/fzy"
git -C "$proof/fzy" fetch --depth 1 https://github.com/jhawthorn/fzy.git 34b88869d022e861da4846c4463aea3ddfb3ff30
git -C "$proof/fzy" checkout --detach FETCH_HEAD
make -C "$proof/fzy" -j2 fzy
shasum -a 256 "$proof/fz-before" "$proof/fz-after" "$proof/fzy/fzy"
wc -c "$proof/fz-before" "$proof/fz-after"
rustc --version
uname -smr
sysctl -n hw.ncpu

gate="$repo/.work/tools/dotfiles/base/.local/bin/perf-gate-ready"
check_load() {
    if ! "$gate" "$@" > "$proof/gate.log" 2>&1; then
        printf '%s\n' 'host failed the timing readiness check' >&2
        cat "$proof/gate.log" >&2
        return 75
    fi
    if [[ $(cat "$proof/gate.log") == *'hot process tree'* ]]; then
        printf '%s\n' 'a separate CPU-intensive process made this timing sample unusable' >&2
        return 75
    fi
}

wait_for_quiet() {
    local attempts=$1 check
    for (( check = 0; check < attempts; check++ )); do
        if check_load -1; then return; fi
        sysctl -n vm.loadavg
        sleep 10
    done
    return 75
}

wait_for_quiet 48
qualified=0
for attempt in {1..12}; do
    result="$proof/focused-$attempt.json"
    if ! wait_for_quiet 6; then
        printf 'Discarded block %s before sampling\n' "$attempt"
        continue
    fi
    PYTHONPATH=benches python3 .github/scripts/focused-performance.py "$proof" > "$result"
    if ! check_load -1; then
        printf 'Discarded block %s after sampling\n' "$attempt"
        continue
    fi
    qualified=$((qualified + 1))
    printf 'BLOCK_JSON %s ' "$qualified"
    gzip -c "$result" | base64 | tr -d '\n'
    printf '\n'
    if (( qualified == 5 )); then exit 0; fi
done
printf 'Qualified %s of five blocks within the trial budget\n' "$qualified" >&2
exit 75
