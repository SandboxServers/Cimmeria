"""End to end: a broken ingest full of private values still produces a clean report, or none.

fixture_db.build(hostile=True) puts whole commands, absolute paths, URL
credentials, tokens and private addresses into every free-text column a
report could show. The report must contain none of fixtures' hostile values
and none of the deny words, and must say how many values it rejected.
"""

import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from . import cli, fixture_db, generate, render
from .scrub import PrivacyError

USERNAME = "Steve"  # the user name planted in the fixture's paths


class PrivacyTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.db_path, cls.expected = fixture_db.build(cls.tmp.name, hostile=True)
        cls.md, cls.js = generate.generate(cls.db_path, deny=[USERNAME], use_local_deny=False)

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def test_no_hostile_value_reaches_either_output(self):
        for text in (self.md, self.js):
            for value in self.expected["hostile"] + [USERNAME, "example.internal", "secret-project"]:
                self.assertNotIn(value, text)
                self.assertNotIn(value.lower(), text.lower())

    def test_rejected_values_are_counted_in_the_stamp(self):
        rejected = json.loads(self.js)["stamp"]["rejected"]
        for kind in ("fingerprint", "agent_type", "cc_version", "profiler_commit", "tool_name"):
            self.assertGreater(rejected.get(kind, 0), 0, kind)
        self.assertIn("Values rejected by the privacy filter", self.md)

    def test_the_report_is_still_useful(self):
        r = json.loads(self.js)
        self.assertEqual(r["tokens"]["totals"]["requests"], len(self.expected["requests"]))
        self.assertGreater(r["cost"]["totals"]["usd"], 0)

    def test_local_names_are_denied_by_default(self):
        with mock.patch.dict("os.environ", {"USERNAME": USERNAME, "TOKEN_PROFILE_DENY": "fixture-worker"}):
            md, js = generate.generate(self.db_path)
        for text in (md, js):
            self.assertNotIn(USERNAME, text)
            self.assertNotIn("fixture-worker", text)


class GateTest(unittest.TestCase):
    """The last layer: anything that slips past validation and redaction stops the write."""

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.tmp = Path(tmp.name)
        self.db_path, _ = fixture_db.build(self.tmp)

    def leak(self, value):
        real = render.markdown
        return mock.patch.object(render, "markdown", lambda report: real(report) + value)

    def test_gate_refuses_a_leaky_render(self):
        for value in ("postgres://u:hunter2@10.0.0.5/db", "C:\\Users\\Steve\\x", "ghp_abcdefghijklmnop1234"):
            with self.subTest(value=value), self.leak(value), self.assertRaises(PrivacyError) as cm:
                generate.generate(self.db_path, use_local_deny=False)
            self.assertNotIn("hunter2", str(cm.exception))

    def test_cli_writes_nothing_when_refused(self):
        out = self.tmp / "out"
        with self.leak("10.0.0.5"), mock.patch("sys.stderr"):
            self.assertEqual(cli.main(["--db", str(self.db_path), "--out", str(out)]), 3)
        self.assertFalse(out.exists())

    def test_cli_writes_markdown_and_json(self):
        out = self.tmp / "out"
        with mock.patch("sys.stdout"):
            self.assertEqual(cli.main(["--db", str(self.db_path), "--out", str(out)]), 0)
        self.assertTrue((out / "token-report.md").read_text(encoding="utf-8").startswith("# Token profile report"))
        self.assertEqual(json.loads((out / "token-report.json").read_text(encoding="utf-8"))["schema"],
                         "cimmeria-token-report/1")

    def test_cli_reports_a_missing_database(self):
        with mock.patch("sys.stderr"):
            self.assertEqual(cli.main(["--db", str(self.tmp / "missing.sqlite"), "--out", str(self.tmp)]), 2)


if __name__ == "__main__":
    unittest.main()
