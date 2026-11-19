import argparse
from pathlib import Path
import unittest
from unittest.mock import patch
import sys

from benches import run


def options(**kwargs):
    values = dict(tier='change', name=[], area=[], skip=[])
    values.update(kwargs)
    return argparse.Namespace(**values)


class SelectionTests(unittest.TestCase):
    def test_tier_areas_and_exclusions(self):
        selected, excluded = run.select(options(area=['process']))
        self.assertEqual(selected, ['process-smoke'])
        self.assertIn(('process-quick', 'above tier ceiling'), excluded)
        self.assertIn(('rust-search', 'area not selected'), excluded)

    def test_unknown_and_empty_are_errors(self):
        for args in (options(name=['missing']), options(area=['matcher'], skip=['rust-search'])):
            with self.subTest(args=args), self.assertRaises(ValueError):
                run.select(args)

    def test_names_are_not_areas(self):
        with self.assertRaisesRegex(ValueError, 'unknown name'):
            run.select(options(name=['process']))

    def test_failed_preflight_starts_nothing(self):
        with patch.object(sys, 'argv', ['run.py', '--tier', 'change', '--name', 'process-smoke']), patch('builtins.print'), patch('benches.run.preflight', return_value=['missing binary']), patch('benches.run.subprocess.run') as execute, patch('pathlib.Path.mkdir') as mkdir:
            self.assertEqual(run.main(), 2)
            execute.assert_not_called()
            mkdir.assert_not_called()

    def test_dry_run_writes_nothing(self):
        with patch.object(sys, 'argv', ['run.py', '--dry-run']), patch('builtins.print'), patch('benches.run.preflight', return_value=[]), patch('benches.run.platform.platform', return_value='test'), patch('benches.run.subprocess.run') as execute, patch('pathlib.Path.mkdir') as mkdir:
            self.assertEqual(run.main(), 0)
            execute.assert_not_called()
            mkdir.assert_not_called()

    def test_preflight_rejects_missing_binary(self):
        with patch('benches.run.shutil.disk_usage') as disk:
            disk.return_value.free = 1_000_000_000
            errors = run.preflight(['process-smoke'], Path('/missing/fz'), Path('/missing/fzy'), Path('/missing/oracle'))
        self.assertTrue(any('fz missing' in error for error in errors))
        self.assertTrue(any('pinned fzy' in error for error in errors))


if __name__ == '__main__':
    unittest.main()
