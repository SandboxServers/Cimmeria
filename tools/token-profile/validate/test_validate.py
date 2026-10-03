"""The attribution scorer and its command line."""

import contextlib
import io
import sqlite3
import tempfile
import unittest
from pathlib import Path

from report import fixture_db

from . import cli, score


def attribution_db(rows):
    db = sqlite3.connect(":memory:")
    db.execute("CREATE TABLE pr_attribution (request_id TEXT, pr_number INTEGER, method TEXT, weight REAL)")
    db.executemany("INSERT INTO pr_attribution VALUES (?, ?, ?, ?)", rows)
    return db


class ScoreTest(unittest.TestCase):
    def test_labelled_request_without_rows_counts_as_unattributed(self):
        db = attribution_db([("r1", 7, "branch", 1.0)])
        out = score.score(db, {"r1": 7, "r2": 7}, usd={"r1": 1.0, "r2": 3.0})
        self.assertEqual(out["labelled_requests"], 2)
        self.assertEqual(out["requests_missing"], 1)
        self.assertAlmostEqual(out["usd"]["total"], 4.0)
        self.assertAlmostEqual(out["usd"]["unattributed_share"], 0.75)
        self.assertAlmostEqual(out["usd"]["recall"], 0.25)
        self.assertAlmostEqual(out["requests"]["unattributed_share"], 0.5)

    def test_weight_rows_leave_out_counts_as_unattributed(self):
        db = attribution_db([("r1", 7, "branch", 0.5)])
        out = score.score(db, {"r1": 7}, usd={"r1": 2.0})
        self.assertAlmostEqual(out["usd"]["total"], 2.0)
        self.assertAlmostEqual(out["usd"]["correct_share"], 0.5)
        self.assertAlmostEqual(out["usd"]["unattributed_share"], 0.5)
        self.assertAlmostEqual(out["usd"]["precision"], 1.0)
        self.assertAlmostEqual(out["usd"]["recall"], 0.5)

    def test_full_rows_add_no_unattributed_weight(self):
        db = attribution_db([("r1", 7, "split", 0.5), ("r1", 8, "split", 0.5)])
        out = score.score(db, {"r1": 7}, usd={"r1": 2.0})
        self.assertAlmostEqual(out["usd"]["unattributed_share"], 0.0)
        self.assertAlmostEqual(out["usd"]["wrong_share"], 0.5)
        self.assertEqual(out["wrong_usd_share_by_method"], {"split": 0.5})


class CliTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.db_path, _ = fixture_db.build(self.tmp.name)

    def tearDown(self):
        self.tmp.cleanup()

    def run_cli(self, *argv):
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = cli.main(["--db", str(self.db_path), *argv])
        return code, out.getvalue(), err.getvalue()

    def test_writes_report(self):
        target = Path(self.tmp.name) / "check.json"
        code, _, _ = self.run_cli("--out", str(target))
        self.assertEqual(code, 0)
        self.assertIn("cimmeria-attribution-check/1", target.read_text(encoding="utf-8"))

    def test_unwritable_out_exits_2_with_a_message(self):
        # A directory can't be written as a file, on every platform.
        code, out, err = self.run_cli("--out", self.tmp.name)
        self.assertEqual(code, 2)
        self.assertIn("status=error", err)
        self.assertEqual(out, "")


if __name__ == "__main__":
    unittest.main()
