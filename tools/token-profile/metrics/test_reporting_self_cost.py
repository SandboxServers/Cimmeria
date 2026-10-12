"""The self-session cost (CM-08i): pricing by tier and role, unpriced rows, and the database query's filters."""

import sqlite3
import tempfile
import unittest
from pathlib import Path

from ingest.prices import CURRENT
from metrics import cost
from reporting import self_cost

ZERO = {"input_tokens": 0, "output_tokens": 0, "cache_read": 0, "cache_write_5m": 0, "cache_write_1h": 0}
# Only the columns SELF_SQL reads. Requests r1 and r2 are in the campaign (by their own row, and by their PR);
# r3 is on another campaign's PR, r5 has no attribution row, and r4 belongs to session s2.
DB_SCHEMA = """
CREATE TABLE requests (request_id TEXT PRIMARY KEY, session_id TEXT, model TEXT, input_tokens INT,
    output_tokens INT, cache_read INT, cache_write_5m INT, cache_write_1h INT);
CREATE TABLE pr_attribution (request_id TEXT, pr_number INT, campaign TEXT);
CREATE TABLE prs (pr_number INT PRIMARY KEY, campaign TEXT);
"""


def _row(model, inp=1000, out=1000, **extra):
    """One SELF_SQL row as a dict (the zero token counts are the ones a row without them carries)."""
    return {**ZERO, "model": model, "input_tokens": inp, "output_tokens": out, **extra}


class PriceSessionsTest(unittest.TestCase):
    def test_prices_by_tier_and_role(self):
        rows = [_row("claude-opus-5-5"), _row("claude-haiku-5-5")]
        out = self_cost.price_sessions(rows)
        self.assertEqual(out["requests"], 2)
        self.assertEqual(out["unpriced"], 0)
        self.assertAlmostEqual(out["by_tier"]["opus"] + out["by_tier"]["haiku"], out["total_usd"], places=12)
        self.assertAlmostEqual(out["by_role"]["coordinator"], out["total_usd"], places=12)
        self.assertEqual(set(out["by_role"]), {"coordinator"})

    def test_unpriced_counted_not_summed(self):
        rows = [_row("claude-haiku-5-5"), _row("claude-no-such-model-9")]
        out = self_cost.price_sessions(rows)
        priced = cost.request_usd({"model": "claude-haiku-5-5", "weight": 1.0,
                                   "tokens": {"input": 1000, "output": 1000}}, CURRENT)
        self.assertEqual(out["requests"], 2)
        self.assertEqual(out["unpriced"], 1)
        self.assertAlmostEqual(out["total_usd"], priced, places=12)
        self.assertEqual(set(out["by_tier"]), {"haiku"})


class LoadSelfCostTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.path = Path(self.tmp.name) / "profile.sqlite"
        db = sqlite3.connect(self.path)
        db.executescript(DB_SCHEMA)
        db.executemany("INSERT INTO requests VALUES (?, ?, ?, 1000, 1000, 0, 0, 0)", [
            ("r1", "s1", "claude-opus-5-5"), ("r2", "s1", "claude-haiku-5-5"),
            ("r3", "s1", "claude-opus-5-5"), ("r4", "s2", "claude-haiku-5-5"), ("r5", "s1", "claude-haiku-5-5")])
        db.executemany("INSERT INTO pr_attribution VALUES (?, ?, ?)", [
            ("r1", None, "campaign-metrics"), ("r2", 101, None), ("r3", 102, None), ("r4", 103, None)])
        db.executemany("INSERT INTO prs VALUES (?, ?)", [(101, "campaign-metrics"), (102, "other"), (103, None)])
        db.commit()
        db.close()

    def tearDown(self):
        self.tmp.cleanup()

    def test_excludes_requests_already_in_campaign(self):
        out = self_cost.load_self_cost(self.path, ["s1"])
        # r1 and r2 are held by the rollup already; r3 (another campaign's PR) and r5 (no attribution) count.
        self.assertEqual(out["requests"], 2)
        self.assertEqual(out["sessions"], 1)

    def test_only_listed_sessions(self):
        self.assertEqual(self_cost.load_self_cost(self.path, ["s2"])["requests"], 1)
        self.assertEqual(self_cost.load_self_cost(self.path, ["s1", "s2"])["requests"], 3)

    def test_empty_ids_returns_zeros_without_db(self):
        out = self_cost.load_self_cost(Path(self.tmp.name) / "missing.sqlite", [])
        self.assertEqual(out["requests"], 0)
        self.assertEqual(out["total_usd"], 0.0)
        self.assertEqual(out["sessions"], 0)
        self.assertFalse((Path(self.tmp.name) / "missing.sqlite").exists())


if __name__ == "__main__":
    unittest.main()
