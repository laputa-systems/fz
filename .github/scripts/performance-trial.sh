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
if cmp -s "$proof/fz-before" "$proof/fz-after"; then
    printf '%s\n' 'expected different release binaries' >&2
    exit 1
fi
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
        return 75
    fi
    if [[ $(cat "$proof/gate.log") == *'hot process tree'* ]]; then
        printf '%s\n' 'a separate CPU-intensive process made this timing sample unusable' >&2
        return 75
    fi
}

wait_for_quiet() {
    for attempt in {1..12}; do
        if check_load -1; then return; fi
        sysctl -n vm.loadavg
        sleep 10
    done
    return 75
}

for pair in 1 2 3 4 5; do
    if (( pair % 2 )); then sides=(before after); else sides=(after before); fi
    for side in "${sides[@]}"; do
        result="$proof/$pair-$side.json"
        wait_for_quiet
        python3 benches/perf.py --warmup 1 --repetitions 5 \
            --fz "$proof/fz-$side" --fzy "$proof/fzy/fzy" \
            --data-dir "$proof/fixtures" --json "$result"
        check_load -1
        printf 'PAIR_JSON %s %s ' "$pair" "$side"
        gzip -c "$result" | base64 | tr -d '\n'
        printf '\n'
    done
done
