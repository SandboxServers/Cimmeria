"""agent_stops for transcripts that end after a tool result or on an API error, on the synthetic fixture."""

import json
import shutil
import sqlite3
import tempfile
import unittest
from pathlib import Path

from . import store
from .test_support import PREFIX, build_fixtures, pr, run_ingest

PRS = [pr(4242, build_fixtures.WORKER_BRANCH, "2026-10-01T11:00:00Z")]
LATER = "2099-01-01T00:00:00.000Z"


class StopRowTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.base = Path(cls.tmp.name)
        cls.full = build_fixtures.build(cls.base / "full")
        cls.main_name = f"{build_fixtures.SESSION}.jsonl"
        lines = (cls.full / PREFIX / cls.main_name).read_text(encoding="utf-8").splitlines()
        cls.result_tpl = next(json.loads(line) for line in lines
                              if '"tool_result"' in line and '"toolu_fixture_' in line)
        cls.error_tpl = next(json.loads(line) for line in lines if '"req_ERR"' in line)

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def stop_row(self, name, *extra):
        """The main stop row after ingesting a copy of the fixture whose main transcript gets `extra` records."""
        root = self.base / name
        shutil.copytree(self.full, root)
        target = root / PREFIX / self.main_name
        with open(target, "ab") as fh:
            for rec in extra:
                fh.write(json.dumps(rec).encode("utf-8") + b"\n")
        db = self.base / f"{name}.sqlite"
        run_ingest(root, db, "--allow-unknown", prs=PRS)
        conn = sqlite3.connect(db)
        self.addCleanup(conn.close)
        return conn.execute("SELECT last_result_is_error, ended_after_result, last_tool_background,"
                            " ended_in_api_error, api_error_kind FROM agent_stops"
                            " WHERE agent_id LIKE 'main:%'").fetchone()

    def result(self, is_error):
        rec = json.loads(json.dumps(self.result_tpl))
        rec.update(uuid="stop-result", parentUuid="req_I", timestamp=LATER)
        rec["message"]["content"] = [{"type": "tool_result", "tool_use_id": "toolu_fixture_bg",
                                      "content": "refused", "is_error": is_error}]
        rec["sourceToolUseID"] = "toolu_fixture_bg"
        return rec

    def api_error(self, kind):
        rec = json.loads(json.dumps(self.error_tpl))
        rec.update(uuid="stop-error", timestamp=LATER)
        rec.pop("error", None)
        if kind is not None:
            rec["error"] = kind
        return rec

    def test_a_transcript_ending_on_an_errored_tool_result(self):
        self.assertEqual(self.stop_row("res", self.result(True)), (1, 1, 1, 0, None))

    def test_an_api_error_after_the_result_is_a_stop_by_error(self):
        self.assertEqual(self.stop_row("rate", self.result(True), self.api_error("rate_limit")),
                         (1, 0, 1, 1, "rate_limit"))

    def test_an_api_error_without_an_error_field_is_other(self):
        self.assertEqual(self.stop_row("other", self.api_error(None))[3:], (1, "other"))

    def test_an_error_value_that_is_not_a_plain_name_is_other_and_never_stored(self):
        row = self.stop_row("hostile", self.api_error("Session limit reached; C:\\private\\path"))
        self.assertEqual(row[3:], (1, "other"))


class OlderSchemaTest(unittest.TestCase):
    def test_a_version_5_database_is_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "old.sqlite"
            conn = store.open_db(path)
            conn.execute("UPDATE meta SET value = '5' WHERE key = 'schema_version'")
            conn.commit()
            conn.close()
            with self.assertRaisesRegex(SystemExit, "schema version 5"):
                store.open_db(path)
