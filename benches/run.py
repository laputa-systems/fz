#!/usr/bin/env python3
"""Select and run the existing benchmark harnesses."""
import argparse
import datetime
import os
from pathlib import Path
import platform
import re
import shlex
import shutil
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parent.parent
TIERS = ('edit', 'change', 'scenario', 'full')
WORK = {
    'harness-tests': ('edit', 'harness', 'Process harness correctness tests', 'seconds'),
    'rust-search': ('change', 'matcher', '100k selective path search, one sample', 'minutes'),
    'process-smoke': ('change', 'process', 'Small filtering and PTY redraw, one repetition', 'minutes'),
    'process-quick': ('scenario', 'process', 'All filter datasets and PTY redraw, three repetitions', 'tens of minutes'),
    'process-full': ('full', 'process', 'All filter datasets and PTY redraw, five repetitions', 'tens of minutes'),
}
for row in re.findall(r'#\[rustybench::bench(?:\([^\n]*\))?\]\nfn (\w+)', (ROOT / 'benches/bench.rs').read_text()):
    WORK['rust-' + row] = ('scenario', 'matcher', 'Rustybench ' + row, 'minutes')


def select(args):
    for values, valid, kind in ((args.name, WORK, 'name'), (args.area, {v[1] for v in WORK.values()}, 'area'), (args.skip, WORK, 'skip')):
        unknown = set(values) - set(valid)
        if unknown:
            raise ValueError(f'unknown {kind}: {", ".join(sorted(unknown))}')
    selected = []
    excluded = []
    for name, (tier, area, _, _) in WORK.items():
        reason = ('above tier ceiling' if TIERS.index(tier) > TIERS.index(args.tier) else
                  'not named' if args.name and name not in args.name else
                  'area not selected' if args.area and area not in args.area else
                  'skipped' if name in args.skip else None)
        (excluded if reason else selected).append((name, reason) if reason else name)
    if not selected:
        raise ValueError('selection is empty')
    return selected, excluded


def preflight(selected, fz, fzy, oracle):
    errors = []
    if any(name.startswith('rust-') for name in selected) and not shutil.which('cargo'):
        errors.append('cargo missing; install the pinned Rust toolchain (rust workloads)')
    process = [name for name in selected if name.startswith('process-')]
    if process:
        for label, path in (('fz', fz), ('fzy', fzy)):
            if not path.is_file() or not os.access(path, os.X_OK):
                errors.append(f'{label} missing or not executable: {path}; build it (needed by {", ".join(process)})')
        if not (oracle / '.git').exists() or subprocess.run(['git', '-C', str(oracle), 'rev-parse', 'HEAD'], capture_output=True, text=True).stdout.strip() != '34b88869d022e861da4846c4463aea3ddfb3ff30':
            errors.append(f'pinned fzy checkout missing at {oracle}; follow docs/testing.md (needed by {", ".join(process)})')
        if fz.exists() and any(p.stat().st_mtime > fz.stat().st_mtime for p in list((ROOT / 'src').rglob('*.rs')) + [ROOT / 'Cargo.toml', ROOT / 'Cargo.lock']):
            print('warning: release fz may be stale; run cargo build --release', file=sys.stderr)
    if shutil.disk_usage(ROOT).free < 100_000_000:
        errors.append('less than 100 MB free; free disk space for selected workloads')
    return errors


def command(name, directory, fz, fzy):
    if name == 'harness-tests':
        return [sys.executable, '-B', '-m', 'unittest', 'benches.test_perf']
    if name.startswith('rust-'):
        cmd = ['cargo', 'bench', '--bench', 'bench', '--', '--format', 'json']
        return cmd + (['--sample-count', '1', '--sample-size', '1', 'choices_search_paths_100k_selective'] if name == 'rust-search' else [name.removeprefix('rust-')])
    cmd = [sys.executable, 'benches/perf.py', '--fz', str(fz), '--fzy', str(fzy), '--data-dir', str(directory / 'data'), '--json', str(directory / (name + '.json'))]
    return cmd + (['--smoke'] if name == 'process-smoke' else ['--quick'] if name == 'process-quick' else [])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--tier', choices=TIERS, default='edit')
    parser.add_argument('--name', action='append', default=[])
    parser.add_argument('--area', action='append', default=[])
    parser.add_argument('--skip', action='append', default=[])
    parser.add_argument('--list', action='store_true')
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--dry-run', action='store_true')
    parser.add_argument('--fz', type=Path, default=ROOT / 'target/release/fz')
    parser.add_argument('--fzy', type=Path, default=ROOT / '.work/tmp/fzy/fzy')
    parser.add_argument('--oracle', type=Path, default=ROOT / '.work/tmp/fzy')
    args = parser.parse_args()
    if args.list:
        for name, (tier, area, purpose, cost) in WORK.items():
            print(f'{name:16} {tier:8} {area:8} {purpose}; estimated {cost}')
        return 0
    try:
        selected, excluded = select(args)
    except ValueError as error:
        print(error, file=sys.stderr)
        return 2
    for name, reason in excluded:
        print(f'excluded {name}: {reason}')
    errors = preflight(selected, args.fz, args.fzy, args.oracle)
    if errors:
        for error in errors:
            print(f'preflight: {error}', file=sys.stderr)
        return 2
    print(f'preflight OK: {", ".join(selected)}; host {platform.platform()}')
    if args.check:
        return 0
    directory = ROOT / '.work/bench' / datetime.datetime.now().strftime('%Y%m%d-%H%M%S-%f')
    if args.dry_run:
        for name in selected:
            print(shlex.join(command(name, directory, args.fz, args.fzy)))
        return 0
    directory.mkdir(parents=True)
    summary = [f'host: {platform.platform()}', f'commit: {subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()}', f'dirty: {bool(subprocess.check_output(["git", "status", "--porcelain"], text=True).strip())}']
    summary.extend(f'excluded {name}: {reason}' for name, reason in excluded)
    failed = False
    for name in selected:
        cmd = command(name, directory, args.fz, args.fzy)
        timed = name != 'harness-tests'
        ready = shutil.which('perf-gate-ready')
        before = subprocess.run([ready], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode if timed and ready else None
        start = time.monotonic()
        with (directory / (name + '.log')).open('w') as log:
            result = subprocess.run(cmd, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT)
        duration = time.monotonic() - start
        after = subprocess.run([ready, '-1'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode if timed and ready and before == 0 else None
        validity = 'qualified host' if before == 0 and after == 0 else 'inconclusive: host readiness not established' if timed else 'not timed'
        line = f'{name}: exit {result.returncode}; {duration:.2f}s; {validity}; no budget checked, completion is not a performance verdict'
        print(line, flush=True)
        summary.extend([f'command: {shlex.join(cmd)}', line])
        failed |= result.returncode != 0
    (directory / 'summary.txt').write_text('\n'.join(summary) + '\n')
    print(f'output: {directory}')
    return int(failed)


if __name__ == '__main__':
    sys.exit(main())
