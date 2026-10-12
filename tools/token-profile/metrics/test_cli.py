"""The overstep subcommand end to end: a fixture database and ledger in, overstep.json out, counts on stdout."""

import contextlib
import io
import json
import sqlite3
import tempfile
import unittest
from pathlib import Path

from metrics import cli

SCHEMA = Path(__file__).resolve().parents[1] / "schema.sql"
LEDGER = {"schema": "cimmeria-campaign-ledger/1", "campaigns": {"campaign-metrics": {
    "packets": [{"id": "CM-07", "implementer": "packet-coder", "size": "M", "escalated": False, "override": False}],
    "window": {"start": "2026-10-01T00:00:00Z", "end": None}}}}


def _fixture_db(path):
    """One human prompt and one Opus coordinator request that edits a code path."""
    with contextlib.closing(sqlite3.connect(path)) as db:
        db.executescript(SCHEMA.read_text(encoding="utf-8"))
        db.execute("INSERT INTO sessions (session_id, project_dir, first_ts, last_ts) VALUES "
                   "('s1', 'p', '2026-10-11T09:00:00Z', '2026-10-11T10:05:00Z')")
        db.execute("INSERT INTO triggers (trigger_id, session_id, ts, kind, rule) VALUES "
                   "('t1', 's1', '2026-10-11T09:59:00Z', 'human_prompt', 'R1')")
        db.execute("INSERT INTO requests (request_id, session_id, trigger_id, message_uuid, records_seen, ts, model, "
                   "input_tokens, output_tokens, cache_read, cache_write_5m, cache_write_1h, context_tokens) VALUES "
                   "('r1', 's1', 't1', 'u1', 1, '2026-10-11T10:00:00Z', 'claude-opus-5-5', 1000000, 0, 0, 0, 0, 1000000)")
        db.execute("INSERT INTO tool_calls (tool_use_id, request_id, session_id, ts, tool_name, fingerprint) VALUES "
                   "('c1', 'r1', 's1', '2026-10-11T10:00:01Z', 'Edit', 'crates/gameserver/src/lib.rs')")
        db.commit()


def _run(argv):
    """(exit status, stdout, stderr) of the CLI run with `argv`."""
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        status = cli.main(argv)
    return status, out.getvalue(), err.getvalue()


class OverstepCliTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self.tmp.name)
        self.db = self.dir / "token-profile.sqlite"
        self.ledger = self.dir / "ledger.json"
        _fixture_db(self.db)
        self.ledger.write_text(json.dumps(LEDGER), encoding="utf-8")

    def tearDown(self):
        self.tmp.cleanup()

    def test_smoke_writes_overstep_json_and_prints_counts_only(self):
        out = self.dir / "out"
        status, stdout, stderr = _run(["overstep", "--db", str(self.db), "--ledger", str(self.ledger), "--out", str(out)])
        self.assertEqual(status, 0, stderr)
        counts = json.loads(stdout)
        self.assertEqual(counts["hits"], {"O1": 1})
        o1 = counts["by_campaign"]["campaign-metrics"]["rules"]["O1"]
        self.assertEqual(o1["count"], 1)
        self.assertEqual(o1["wasted_usd"], 3.50)
        written = json.loads((out / "overstep.json").read_text(encoding="utf-8"))
        self.assertEqual(written["by_campaign"]["campaign-metrics"]["rules"]["O1"]["count"], 1)

    def test_missing_database_is_an_error_and_is_not_created(self):
        missing = self.dir / "absent.sqlite"
        status, _, stderr = _run(["overstep", "--db", str(missing), "--out", str(self.dir / "out")])
        self.assertEqual(status, 2)
        self.assertEqual(stderr.strip(), "status=error OperationalError")
        self.assertFalse(missing.exists())

    def test_missing_ledger_is_an_error(self):
        status, _, stderr = _run(["overstep", "--db", str(self.db), "--ledger", str(self.dir / "none.json"),
                                  "--out", str(self.dir / "out")])
        self.assertEqual(status, 2)
        self.assertEqual(stderr.strip(), "status=error FileNotFoundError")


if __name__ == "__main__":
    unittest.main()
