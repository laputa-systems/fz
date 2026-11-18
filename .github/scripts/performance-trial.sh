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
cp "$proof/fz-before" "$proof/fz-after"
cmp "$proof/fz-before" "$proof/fz-after"
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
    pair=$((qualified + 1))
    attempt_dir="$proof/attempt-$attempt"
    mkdir -p "$attempt_dir"
    if (( pair % 2 )); then sides=(before after); else sides=(after before); fi
    for side in "${sides[@]}"; do
        result="$attempt_dir/$side.json"
        if ! wait_for_quiet 6; then
            printf 'Discarded attempt %s before the %s sample\n' "$attempt" "$side"
            continue 2
        fi
        if python3 benches/perf.py --warmup 1 --repetitions 5 \
            --fz "$proof/fz-$side" --fzy "$proof/fzy/fzy" \
            --data-dir "$proof/fixtures" --json "$result" > "$attempt_dir/$side.log" 2>&1; then
            if ! check_load -1; then
                printf 'Discarded attempt %s after the %s sample\n' "$attempt" "$side"
                continue 2
            fi
        else
            status=$?
            cat "$attempt_dir/$side.log" >&2
            exit "$status"
        fi
    done
    for side in "${sides[@]}"; do
        printf 'PAIR_JSON %s %s ' "$pair" "$side"
        gzip -c "$attempt_dir/$side.json" | base64 | tr -d '\n'
        printf '\n'
    done
    qualified=$pair
    if (( qualified == 5 )); then exit 0; fi
done
printf 'Qualified %s of five pairs within the trial budget\n' "$qualified" >&2
exit 75
