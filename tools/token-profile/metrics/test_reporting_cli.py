"""Own-session cost applied to the campaign (CM-08k): the apply step, then the `report` subcommand end to end."""

import contextlib
import copy
import io
import json
import shutil
import sqlite3
import tempfile
import unittest
from pathlib import Path

from metrics import cli
from reporting import self_apply
from reporting.inputs import load_inputs

FIXTURES = Path(__file__).resolve().parent / "fixtures" / "inputs"
SELF = {"requests": 3, "unpriced": 0, "total_usd": 5.0, "by_tier": {"opus": 4.0, "haiku": 1.0},
        "by_role": {"coordinator": 5.0}, "sessions": 2}
# The columns self_cost's SELF_SQL reads; s1 is the coordinator session priced in, s2 is not listed.
DB_SCHEMA = """
CREATE TABLE requests (request_id TEXT PRIMARY KEY, session_id TEXT, model TEXT, input_tokens INT,
    output_tokens INT, cache_read INT, cache_write_5m INT, cache_write_1h INT);
CREATE TABLE pr_attribution (request_id TEXT, pr_number INT, campaign TEXT);
CREATE TABLE prs (pr_number INT PRIMARY KEY, campaign TEXT);
"""


def _fixture_db(path):
    """Two Opus requests, one in each session, neither attributed to a campaign."""
    with contextlib.closing(sqlite3.connect(path)) as db:
        db.executescript(DB_SCHEMA)
        db.execute("INSERT INTO requests VALUES ('r1', 's1', 'claude-opus-5-5', 1000000, 0, 0, 0, 0)")
        db.execute("INSERT INTO requests VALUES ('r2', 's2', 'claude-opus-5-5', 1000000, 0, 0, 0, 0)")
        db.commit()


def _schema_db(path):
    """A scratch database built from the profiler's schema.sql, holding one Opus request in session sess-a that no
    campaign claims."""
    schema = (Path(__file__).resolve().parents[1] / "schema.sql").read_text(encoding="utf-8")
    with contextlib.closing(sqlite3.connect(path)) as db:
        db.executescript(schema)
        db.execute("INSERT INTO sessions (session_id, project_dir, first_ts, last_ts, is_coordinator) "
                   "VALUES ('sess-a', 'proj', '2026-10-11T00:00:00Z', '2026-10-11T01:00:00Z', 1)")
        db.execute("INSERT INTO triggers (trigger_id, session_id, ts, kind, rule) "
                   "VALUES ('t-a', 'sess-a', '2026-10-11T00:00:00Z', 'human_prompt', 'R1')")
        db.execute("INSERT INTO requests (request_id, session_id, trigger_id, message_uuid, records_seen, ts, model, "
                   "input_tokens, output_tokens, cache_read, cache_write_5m, cache_write_1h, context_tokens) "
                   "VALUES ('r-a', 'sess-a', 't-a', 'm-a', 1, '2026-10-11T00:00:00Z', 'claude-opus-5-5', "
                   "1000000, 0, 0, 0, 0, 1000000)")
        db.commit()


def _run(argv):
    """(exit status, stdout, stderr) of the CLI run with `argv`."""
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        status = cli.main(argv)
    return status, out.getvalue(), err.getvalue()


class ApplySelfCostTest(unittest.TestCase):
    def test_apply_adds_to_unattributed_and_counts_sessions(self):
        inputs = load_inputs(FIXTURES)
        before = inputs["cost"]["campaigns"]["alpha-old"]["cost"]
        out = self_apply.apply_self_cost(inputs, SELF, campaign="alpha-old")
        cost = out["cost"]["campaigns"]["alpha-old"]["cost"]
        self.assertAlmostEqual(cost["total_usd"], before["total_usd"] + 5.0)
        self.assertAlmostEqual(cost["unattributed_in_campaign_usd"], before["unattributed_in_campaign_usd"] + 5.0)
        self.assertAlmostEqual(cost["by_model_tier"]["opus"], before["by_model_tier"]["opus"] + 4.0)
        self.assertAlmostEqual(cost["by_role"]["coordinator"], before["by_role"]["coordinator"] + 5.0)
        self.assertEqual(cost["self_sessions"], 2)

    def test_absent_campaign_is_created_with_zero_pr_cost(self):
        inputs = load_inputs(FIXTURES)
        out = self_apply.apply_self_cost(inputs, SELF)
        record = out["cost"]["campaigns"]["campaign-metrics"]
        self.assertEqual(record["regime"], "mixed")
        self.assertAlmostEqual(record["cost"]["total_usd"], 5.0)
        self.assertEqual(record["cost"]["self_sessions"], 2)
        self.assertEqual(record["prs"], {"count": 0, "merged": 0, "cost_per_pr": {"mean": None, "p50": None, "p90": None},
                                         "per_pr": []})
        self.assertEqual(record["accepted"], {"packets": 0, "cost_per_accepted_usd": None,
                                              "accepted_gap": "no packet table"})

    def test_apply_does_not_mutate_inputs(self):
        inputs = load_inputs(FIXTURES)
        snapshot = copy.deepcopy(inputs)
        self_apply.apply_self_cost(inputs, SELF)
        self.assertEqual(inputs, snapshot)


class ReportCliTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self.tmp.name)
        self.inputs = self.dir / "inputs"
        shutil.copytree(FIXTURES, self.inputs)
        self.out = self.dir / "out"
        self.db = self.dir / "token-profile.sqlite"
        _fixture_db(self.db)

    def tearDown(self):
        self.tmp.cleanup()

    def test_cli_report_writes_seven_files(self):
        status, out, _ = _run(["report", "--inputs", str(self.inputs), "--out", str(self.out)])
        self.assertEqual(status, 0)
        self.assertEqual(json.loads(out), {"campaigns": 2, "files": 7, "self_sessions": 0})
        for rel in ("data/alpha-old.json", "data/beta-haiku.json", "data/summary.json", "data/metrics.json",
                    "reports/alpha-old.md", "reports/beta-haiku.md", "reports/summary.md"):
            self.assertTrue((self.out / rel).is_file(), rel)

    def test_cli_self_session_prices_only_listed_session(self):
        status, out, _ = _run(["report", "--inputs", str(self.inputs), "--out", str(self.out), "--db", str(self.db),
                               "--self-session", "s1"])
        self.assertEqual(status, 0)
        self.assertEqual(json.loads(out)["self_sessions"], 1)
        # s1's one request is the only one priced; s2's request is not listed, so it is not counted.
        summary = (self.out / "reports" / "summary.md").read_bytes().decode("utf-8")
        self.assertIn("over 1 request in 1 session.", summary)

    def test_cli_own_session_stays_out_of_old_column(self):
        # The own cost makes campaign-metrics, which is mixed: its dollars must not move the old column's total.
        scratch = self.dir / "scratch.sqlite"
        _schema_db(scratch)
        status, _, _ = _run(["report", "--inputs", str(self.inputs), "--out", str(self.out), "--db", str(scratch),
                             "--self-session", "sess-a"])
        self.assertEqual(status, 0)
        summary = (self.out / "reports" / "summary.md").read_bytes().decode("utf-8")
        self.assertIn("| Pure campaigns | 1 |", summary)
        self.assertIn("| Total USD | $100.00 |", summary)
        self.assertIn("- Mixed campaigns are excluded from the cost comparison: campaign-metrics.", summary)

    def test_cli_own_session_leaves_haiku_column_unchanged(self):
        # campaign-metrics has an all-Haiku packets record, yet its own cost keeps it mixed: the Haiku column must not move.
        packets_path = self.inputs / "packets.json"
        packets = json.loads(packets_path.read_text(encoding="utf-8"))
        packets["campaigns"]["campaign-metrics"] = {
            "accepted": {"packets": 1},
            "packets": [{"id": "CM-1", "implementer_sessions": 2, "reviewer_sessions": 1, "fix_commits": 0,
                         "first_pass": True, "accepted": True, "escalated": False, "cycle_hours": 1.0, "retries": []}],
        }
        packets_path.write_text(json.dumps(packets), encoding="utf-8")
        baseline_out = self.dir / "baseline"
        _run(["report", "--inputs", str(self.inputs), "--out", str(baseline_out)])
        scratch = self.dir / "scratch.sqlite"
        _schema_db(scratch)
        _run(["report", "--inputs", str(self.inputs), "--out", str(self.out), "--db", str(scratch),
              "--self-session", "sess-a"])

        def haiku_rows(out):
            summary = (out / "reports" / "summary.md").read_bytes().decode("utf-8")
            return [line for line in summary.splitlines() if line.startswith(("| Pure campaigns |", "| Total USD |"))]

        self.assertEqual(len(haiku_rows(self.out)), 2)
        self.assertEqual(haiku_rows(self.out), haiku_rows(baseline_out))

    def test_cli_self_session_needs_db(self):
        status, _, err = _run(["report", "--inputs", str(self.inputs), "--out", str(self.out), "--self-session", "s1"])
        self.assertEqual(status, 2)
        self.assertIn("status=error self-session-needs-db", err)
        self.assertFalse(self.out.exists())

    def test_cli_missing_inputs_dir_exit_2(self):
        status, _, err = _run(["report", "--inputs", str(self.dir / "nowhere"), "--out", str(self.out)])
        self.assertEqual(status, 2)
        self.assertIn("status=error no inputs found", err)
        self.assertFalse(self.out.exists())

    def test_cli_privacy_refusal_exit_3_and_no_files(self):
        cost_path = self.inputs / "cost.json"
        cost = json.loads(cost_path.read_text(encoding="utf-8"))
        cost["stamp"]["price_source"] = "C:\\Users\\x\\secret"
        cost_path.write_text(json.dumps(cost), encoding="utf-8")
        status, _, err = _run(["report", "--inputs", str(self.inputs), "--out", str(self.out)])
        self.assertEqual(status, 3)
        self.assertIn("status=refused", err)
        self.assertFalse(self.out.exists())


if __name__ == "__main__":
    unittest.main()
