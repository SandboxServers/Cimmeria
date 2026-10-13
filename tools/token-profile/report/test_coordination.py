"""The coordination overhead report over a three-request fixture: main Opus, packet-coder Haiku, reviewer Sonnet."""

import contextlib
import io
import sqlite3
import tempfile
import unittest
from pathlib import Path

from . import cli, coordination
from .db import open_db
from .scrub import Scrubber

SCHEMA = Path(__file__).resolve().parent.parent / "schema.sql"
NEWEST = "2026-10-12"
OLDER = "2026-10-01"

# Per MTok: input, output, cache_read, cache_write_5m, cache_write_1h. The older table doubles every rate,
# so a report that priced with it would show double the USD and fail the exact checks below.
PRICES = {
    NEWEST: {"claude-opus-5-5": (5, 25, 0.5, 6.25, 10), "claude-haiku-5-5": (1, 5, 0.1, 1.25, 2),
             "claude-sonnet-5-5": (3, 15, 0.3, 3.75, 6)},
    OLDER: {"claude-opus-5-5": (10, 50, 1, 12.5, 20)},
}

# request, session, agent, trigger, ts, model, input, output, cache_read, cache_write_5m, context
REQUESTS = [
    ("R_MAIN", "S1", None, "T_MAIN", "2026-10-01T12:00:00.000Z", "claude-opus-5-5", 1000, 100, 50000, 0, 51000),
    ("R_CODER", "S1", "A_CODER", "T_CODER", "2026-10-01T12:01:00.000Z", "claude-haiku-5-5", 500, 200, 20000, 1000,
     21500),
    ("R_REV", "S1", "A_REV", "T_REV", "2026-10-01T12:02:00.000Z", "claude-sonnet-5-5", 800, 300, 30000, 0, 30800),
]

# Exact list-price USD of the three requests at the newest table.
USD = {"main thread": 0.0325, "packet-coder": 0.00475, "packet-reviewer": 0.0159}
TOTAL = sum(USD.values())


def make_db(path):
    db = sqlite3.connect(path)
    db.executescript(SCHEMA.read_text(encoding="utf-8"))
    for version, rows in PRICES.items():
        for model, (i, o, cr, cw5, cw1) in rows.items():
            db.execute("INSERT INTO price_tables (version, model, input, output, cache_read, cache_write_5m,"
                       " cache_write_1h, source) VALUES (?, ?, ?, ?, ?, ?, ?, 'test')",
                       (version, model, i, o, cr, cw5, cw1))
    db.execute("INSERT INTO sessions (session_id, project_dir, first_ts, last_ts, entrypoint, is_coordinator)"
               " VALUES ('S1', 'proj', '2026-10-01T12:00:00.000Z', '2026-10-01T12:05:00.000Z', 'cli', 1)")
    db.execute("INSERT INTO sessions (session_id, project_dir, first_ts, last_ts, entrypoint, is_coordinator)"
               " VALUES ('S2', 'proj', '2026-10-01T13:00:00.000Z', '2026-10-01T13:05:00.000Z', 'cli', 0)")
    for agent, custom in (("A_CODER", "packet-coder"), ("A_REV", "packet-reviewer")):
        db.execute("INSERT INTO agents (agent_id, session_id, agent_type, custom_agent_type, meta_missing)"
                   " VALUES (?, 'S1', 'general-purpose', ?, 0)", (agent, custom))
    for trigger, agent, kind in (("T_MAIN", None, "human_prompt"), ("T_CODER", "A_CODER", "subagent_prompt"),
                                 ("T_REV", "A_REV", "subagent_prompt")):
        db.execute("INSERT INTO triggers (trigger_id, session_id, agent_id, ts, kind, rule)"
                   " VALUES (?, 'S1', ?, '2026-10-01T12:00:00.000Z', ?, 'R1')", (trigger, agent, kind))
    for rid, session, agent, trigger, ts, model, inp, out, cr, cw5, ctx in REQUESTS:
        db.execute("INSERT INTO requests (request_id, session_id, agent_id, trigger_id, message_uuid, records_seen,"
                   " ts, model, input_tokens, output_tokens, cache_read, cache_write_5m, cache_write_1h,"
                   " context_tokens) VALUES (?, ?, ?, ?, ?, 1, ?, ?, ?, ?, ?, ?, 0, ?)",
                   (rid, session, agent, trigger, "m-" + rid, ts, model, inp, out, cr, cw5, ctx))
    db.commit()
    db.close()


class CoordinationTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.path = Path(cls.tmp.name) / "profile.sqlite"
        make_db(cls.path)

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def build(self, session=None):
        db = open_db(self.path)
        try:
            return coordination.build(db, Scrubber(use_local=False), session=session)
        finally:
            db.close()

    def test_roles_usd_and_shares(self):
        rep = self.build()
        roles = {r["role"]: r for r in rep["by_role"]}
        self.assertEqual(set(roles), set(USD))
        self.assertEqual(rep["by_role"][0]["role"], "main thread")
        for name, want in USD.items():
            self.assertAlmostEqual(roles[name]["usd"], want, places=9)
            self.assertAlmostEqual(roles[name]["share"], want / TOTAL, places=9)
            self.assertEqual(roles[name]["requests"], 1)
        self.assertEqual(roles["packet-coder"]["models"], ["claude-haiku-5-5"])
        self.assertEqual(roles["main thread"]["avg_context"], 51000)
        self.assertEqual(rep["price_table"], NEWEST)
        self.assertAlmostEqual(rep["totals"]["usd"], TOTAL, places=9)

    def test_coordination_share_counts_main_and_reviewer_not_coder(self):
        rep = self.build()
        coord = rep["coordination"]
        self.assertEqual(coord["requests"], 2)
        self.assertAlmostEqual(coord["usd"], USD["main thread"] + USD["packet-reviewer"], places=9)
        self.assertAlmostEqual(coord["share"], (USD["main thread"] + USD["packet-reviewer"]) / TOTAL, places=9)

    def test_trigger_kinds_and_first_request_context_count_main_only(self):
        rep = self.build()
        self.assertEqual(rep["trigger_kinds"], [{"kind": "human_prompt", "requests": 1}])
        self.assertEqual(rep["main_thread"]["first_request_context"]["p50"], 51000)
        self.assertEqual(rep["main_thread"]["avg_context"], 51000)

    def test_session_filter_and_empty_window(self):
        rep = self.build(session="S2")
        self.assertEqual(rep["totals"]["requests"], 0)
        self.assertEqual(rep["by_role"], [])
        self.assertEqual(rep["coordination"]["share"], 0.0)

    def test_window_is_since_inclusive_until_exclusive(self):
        db = open_db(self.path)
        try:
            rep = coordination.build(db, Scrubber(use_local=False),
                                     since="2026-10-01T12:01:00.000Z", until="2026-10-01T12:02:00.000Z")
        finally:
            db.close()
        self.assertEqual([r["role"] for r in rep["by_role"]], ["packet-coder"])
        self.assertEqual(rep["trigger_kinds"], [])
        self.assertIsNone(rep["main_thread"]["avg_context"])
        self.assertEqual(rep["coordination"]["requests"], 0)
        self.assertEqual(rep["coordination"]["share"], 0.0)

    def test_cli_subcommand_prints_report_and_reports_bad_database(self):
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = cli.main(["coordination", "--db", str(self.path)])
        self.assertEqual(code, 0)
        self.assertIn("## Coordination share", out.getvalue())
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = cli.main(["coordination", "--db", str(self.path.with_name("missing.sqlite"))])
        self.assertEqual(code, 2)
        self.assertEqual(out.getvalue(), "")
        self.assertIn("status=error", err.getvalue())

    def test_markdown_names_the_roles_and_the_share(self):
        db = open_db(self.path)
        try:
            text = coordination.render(Scrubber(use_local=False).obj(
                coordination.build(db, Scrubber(use_local=False))))
        finally:
            db.close()
        self.assertIn("## Coordination share", text)
        self.assertIn("packet-reviewer", text)
        self.assertIn("human_prompt", text)
        self.assertIn(NEWEST, text)


if __name__ == "__main__":
    unittest.main()
