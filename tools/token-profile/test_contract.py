"""Tests for the token profiler's data contract.

They check the contract, not an ingest: schema.sql loads and its constraints
reject the mistakes the issue made, and the synthetic fixture can tell a right
ingest from a wrong one. TP-01a's ingest tests run against the same fixture.

    python -m unittest discover -s tools/token-profile -p "test_*.py"
"""

import json
import re
import sqlite3
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE / "fixtures"))
import build_fixtures  # noqa: E402


def trigger_rules(doc):
    """Map each rule id in transcript-format.md's trigger table to its kind."""
    table = doc.split("## Trigger classification", 1)[1].split("\n## ", 1)[0]
    rules = {}
    for line in table.splitlines():
        m = re.match(r"\| (R\d+) \|", line)
        if m:
            rules[m.group(1)] = re.search(r"`([a-z_]+)`", line.rstrip(" |").rsplit("|", 1)[1]).group(1)
    return rules


def load_schema():
    db = sqlite3.connect(":memory:")
    db.executescript((HERE / "schema.sql").read_text(encoding="utf-8"))
    return db


def seed_request(db, request_id="r1", output=100, thinking=50, inp=1, read=2, w5=3, w1h=4, context=None,
                 trigger="t1"):
    db.execute("INSERT OR IGNORE INTO sessions (session_id, project_dir, first_ts, last_ts) VALUES ('s', 'p', 't', 't')")
    db.execute("INSERT OR IGNORE INTO triggers (trigger_id, session_id, ts, kind, rule)"
               " VALUES ('t1', 's', 't', 'human_prompt', 'R13')")
    db.execute(
        "INSERT INTO requests (request_id, session_id, trigger_id, message_uuid, records_seen, ts, model,"
        " input_tokens, output_tokens, thinking_tokens, cache_read, cache_write_5m, cache_write_1h, context_tokens)"
        " VALUES (?, 's', ?, 'u', 2, 't', 'm', ?, ?, ?, ?, ?, ?, ?)",
        (request_id, trigger, inp, output, thinking, read, w5, w1h,
         inp + read + w5 + w1h if context is None else context),
    )


class SchemaTest(unittest.TestCase):
    def test_schema_loads_with_version(self):
        db = load_schema()
        self.assertEqual(db.execute("SELECT value FROM meta WHERE key = 'schema_version'").fetchone()[0], "2")

    def test_thinking_is_a_subset_of_output(self):
        db = load_schema()
        seed_request(db, "ok", output=100, thinking=100)
        with self.assertRaises(sqlite3.IntegrityError):
            seed_request(db, "bad", output=100, thinking=101)

    def test_context_is_the_sum_of_input_columns(self):
        db = load_schema()
        with self.assertRaises(sqlite3.IntegrityError):
            seed_request(db, "bad", context=999)

    def test_unattributed_rows_have_no_pr_and_attributed_rows_do(self):
        db = load_schema()
        seed_request(db)
        db.execute("INSERT INTO prs (pr_number, head_branch, created_at, state) VALUES (7, 'b', 't', 'MERGED')")
        db.execute("INSERT INTO pr_attribution VALUES ('r1', 7, 'branch', 0.5, 1.0)")
        db.execute("INSERT INTO pr_attribution VALUES ('r1', NULL, 'unattributed', 0.5, 0.0)")
        with self.assertRaises(sqlite3.IntegrityError):
            db.execute("INSERT INTO pr_attribution VALUES ('r1', NULL, 'branch', 0.1, 1.0)")
        with self.assertRaises(sqlite3.IntegrityError):
            db.execute("INSERT INTO pr_attribution VALUES ('r1', 7, 'unattributed', 0.1, 0.0)")

    def test_every_request_has_a_trigger(self):
        db = load_schema()
        with self.assertRaises(sqlite3.IntegrityError):
            seed_request(db, trigger=None)

    def test_imbalanced_attribution_is_reported(self):
        db = load_schema()
        seed_request(db, "missing")
        seed_request(db, "partial")
        seed_request(db, "over")
        seed_request(db, "ok")
        db.execute("INSERT INTO prs (pr_number, head_branch, created_at, state) VALUES (7, 'b', 't', 'MERGED')")
        db.execute("INSERT INTO prs (pr_number, head_branch, created_at, state) VALUES (8, 'c', 't', 'MERGED')")
        db.execute("INSERT INTO pr_attribution VALUES ('partial', 7, 'branch', 0.5, 1.0)")
        db.execute("INSERT INTO pr_attribution VALUES ('over', 7, 'split', 0.7, 0.6)")
        db.execute("INSERT INTO pr_attribution VALUES ('over', 8, 'split', 0.7, 0.6)")
        db.execute("INSERT INTO pr_attribution VALUES ('ok', 7, 'split', 0.5, 0.6)")
        db.execute("INSERT INTO pr_attribution VALUES ('ok', NULL, 'unattributed', 0.5, 0.0)")
        bad = {row[0] for row in db.execute("SELECT request_id FROM attribution_imbalance")}
        self.assertEqual(bad, {"missing", "partial", "over"})

    def test_one_unattributed_row_per_request(self):
        db = load_schema()
        seed_request(db)
        db.execute("INSERT INTO pr_attribution VALUES ('r1', NULL, 'unattributed', 0.5, 0.0)")
        with self.assertRaises(sqlite3.IntegrityError):
            db.execute("INSERT INTO pr_attribution VALUES ('r1', NULL, 'unattributed', 0.5, 0.0)")

    def test_trigger_kinds_match_the_classification_table(self):
        db = load_schema()
        sql = db.execute("SELECT sql FROM sqlite_master WHERE name = 'triggers'").fetchone()[0]
        schema_kinds = set(re.findall(r"'([a-z_]+)'", sql.split("kind", 1)[1].split("origin_kind", 1)[0]))
        doc = (HERE / "transcript-format.md").read_text(encoding="utf-8")
        self.assertEqual(schema_kinds, set(trigger_rules(doc).values()))


class FixtureTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.root = build_fixtures.build(cls.tmp.name)
        cls.expected = json.loads((cls.root / "expected.json").read_text(encoding="utf-8"))
        cls.records = []
        for path in sorted(cls.root.rglob("*.jsonl")):
            for line in path.read_text(encoding="utf-8").splitlines():
                cls.records.append(json.loads(line))

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def assistant_records(self, request_id):
        return [r for r in self.records if r.get("type") == "assistant" and r.get("requestId") == request_id]

    def test_first_record_dedupe_gives_the_wrong_answer(self):
        # Every request has a streaming partial first, so an ingest that keeps
        # the first record cannot match expected.json.
        for rid, want in self.expected["requests"].items():
            recs = self.assistant_records(rid)
            self.assertGreaterEqual(len(recs), 2, rid)
            self.assertEqual(recs[-1]["message"]["usage"]["output_tokens"], want["output_tokens"], rid)
            self.assertNotEqual(recs[0]["message"]["usage"]["output_tokens"], want["output_tokens"], rid)

    def test_thinking_never_exceeds_output(self):
        for rid, want in self.expected["requests"].items():
            self.assertLessEqual(want["thinking_tokens"], want["output_tokens"], rid)
        self.assertTrue(any(w["thinking_tokens"] for w in self.expected["requests"].values()))

    def test_every_trigger_rule_is_covered(self):
        doc = (HERE / "transcript-format.md").read_text(encoding="utf-8")
        covered = {(w["rule"], w["trigger_kind"]) for w in self.expected["requests"].values()}
        self.assertEqual(covered, set(trigger_rules(doc).items()))

    def test_api_errors_are_not_requests(self):
        errors = [r for r in self.records if r.get("isApiErrorMessage")]
        self.assertTrue(errors)
        for r in errors:
            self.assertEqual(r["message"]["model"], "<synthetic>")
            self.assertNotIn(r["requestId"], self.expected["requests"])
        self.assertEqual(self.expected["requests"]["req_R"]["trigger_id"], "req_R:retry")

    def test_fingerprints_keep_no_hostile_value(self):
        for fingerprint in self.expected["tool_fingerprints"].values():
            for value in self.expected["hostile"]:
                self.assertNotIn(value, fingerprint or "")

    def test_hostile_values_are_planted(self):
        # The privacy tests in TP-01b are only meaningful if the input carries
        # the values they must keep out of reports.
        blob = "\n".join(json.dumps(r) for r in self.records)
        for value in self.expected["hostile"]:
            self.assertIn(json.dumps(value)[1:-1], blob, value)

    def test_unknown_shape_is_present_and_undocumented(self):
        doc = (HERE / "transcript-format.md").read_text(encoding="utf-8")
        for shape in self.expected["unknown_shapes"]:
            self.assertTrue(any(r.get("type") == shape for r in self.records))
            self.assertNotIn(shape, doc)

    def test_every_other_record_type_is_documented(self):
        doc = (HERE / "transcript-format.md").read_text(encoding="utf-8")
        unknown = set(self.expected["unknown_shapes"])
        for r in self.records:
            t = r["type"]
            if t in unknown:
                continue
            self.assertIn(f"`{t}`", doc, t)
            if t == "system":
                self.assertIn(f"`{r['subtype']}`", doc)
            if t == "attachment":
                self.assertIn(f"`{r['attachment']['type']}`", doc)

    def test_subagent_has_meta(self):
        metas = list(self.root.rglob("*.meta.json"))
        self.assertEqual(len(metas), 1)
        meta = json.loads(metas[0].read_text(encoding="utf-8"))
        agent = next(iter(self.expected["agents"].values()))
        self.assertEqual(meta["customAgentType"], agent["custom_agent_type"])


if __name__ == "__main__":
    unittest.main()
