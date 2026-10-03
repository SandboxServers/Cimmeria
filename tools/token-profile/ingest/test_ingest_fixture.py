"""The ingest against the contract's synthetic fixture: every value expected.json pins."""

import json
import sqlite3
import tempfile
import unittest
from pathlib import Path

from .test_support import build_fixtures, pr, run_ingest

AGENT = build_fixtures.AGENT
SESSION = build_fixtures.SESSION


class FixtureIngestTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        tmp = Path(cls.tmp.name)
        cls.root = build_fixtures.build(tmp / "projects")
        cls.expected = json.loads((cls.root / "expected.json").read_text(encoding="utf-8"))
        cls.db_path = tmp / "profile.sqlite"
        prs = [pr(4242, build_fixtures.WORKER_BRANCH, "2026-10-01T11:00:00Z")]
        cls.status_strict = run_ingest(cls.root, tmp / "strict.sqlite", prs=prs)
        cls.status = run_ingest(cls.root, cls.db_path, "--allow-unknown", prs=prs)
        cls.status_again = run_ingest(cls.root, tmp / "strict.sqlite", prs=prs)
        cls.db = sqlite3.connect(cls.db_path)

    @classmethod
    def tearDownClass(cls):
        cls.db.close()
        cls.tmp.cleanup()

    def q(self, sql, *args):
        return self.db.execute(sql, args).fetchall()

    def test_requests_are_exactly_the_expected_set(self):
        got = {r for (r,) in self.q("SELECT request_id FROM requests")}
        self.assertEqual(got, set(self.expected["requests"]))

    def test_token_columns_come_from_the_final_record(self):
        cols = ("input_tokens", "output_tokens", "thinking_tokens", "cache_read", "cache_write_5m", "cache_write_1h")
        for rid, want in self.expected["requests"].items():
            row = self.q(f"SELECT {', '.join(cols)}, records_seen FROM requests WHERE request_id = ?", rid)[0]
            self.assertEqual(dict(zip(cols, row)), {c: want[c] for c in cols}, rid)
            self.assertEqual(row[-1], 2, rid)

    def test_first_record_dedupe_would_fail_this_ingest(self):
        # Every fixture request's first record is a streaming partial with output 3.
        for rid, want in self.expected["requests"].items():
            (out,) = self.q("SELECT output_tokens FROM requests WHERE request_id = ?", rid)[0]
            self.assertEqual(out, want["output_tokens"])
            self.assertNotEqual(out, 3, rid)

    def test_thinking_is_a_subset_of_output(self):
        self.assertEqual(self.q("SELECT COUNT(*) FROM requests WHERE thinking_tokens > output_tokens")[0][0], 0)
        (thinking, output) = self.q("SELECT thinking_tokens, output_tokens FROM requests WHERE request_id = 'req_A'")[0]
        self.assertEqual((thinking, output), (120, 400))
        (thinking,) = self.q("SELECT thinking_tokens FROM requests WHERE request_id = 'req_C'")[0]
        self.assertEqual(thinking, 0)  # older record: no output_tokens_details

    def test_context_is_the_sum_of_input_columns(self):
        (ctx,) = self.q("SELECT context_tokens FROM requests WHERE request_id = 'req_S2'")[0]
        self.assertEqual(ctx, 3 + 61000 + 20000)

    def test_every_request_has_its_expected_trigger(self):
        for rid, want in self.expected["requests"].items():
            kind, rule, tid = self.q("SELECT t.kind, t.rule, t.trigger_id FROM requests r JOIN triggers t"
                                     " USING (trigger_id) WHERE r.request_id = ?", rid)[0]
            self.assertEqual((kind, rule), (want["trigger_kind"], want["rule"]), rid)
            if "trigger_id" in want:
                self.assertEqual(tid, want["trigger_id"], rid)

    def test_unknown_and_mixed_buckets_are_used(self):
        kinds = {k for (k,) in self.q("SELECT kind FROM triggers")}
        self.assertIn("unknown", kinds)
        self.assertIn("mixed", kinds)

    def test_api_errors_are_not_requests(self):
        self.assertEqual(self.q("SELECT COUNT(*) FROM requests WHERE model = '<synthetic>' OR request_id = 'req_ERR'"),
                         [(0,)])

    def test_main_and_subagent_requests_are_told_apart(self):
        for rid, want in self.expected["requests"].items():
            (agent,) = self.q("SELECT agent_id FROM requests WHERE request_id = ?", rid)[0]
            self.assertEqual(agent, want["agent_id"], rid)

    def test_agent_type_comes_from_meta(self):
        row = self.q("SELECT custom_agent_type, agent_type, name, meta_missing, session_id FROM agents"
                     " WHERE agent_id = ?", AGENT)
        self.assertEqual(row, [(self.expected["agents"][AGENT]["custom_agent_type"], "fixture-worker",
                                "fixture-worker", 0, SESSION)])
        (branch,) = self.q("SELECT DISTINCT git_branch FROM requests WHERE agent_id = ?", AGENT)[0]
        self.assertEqual(branch, self.expected["agents"][AGENT]["branch"])

    def test_session_is_a_coordinator(self):
        self.assertEqual(self.q("SELECT is_coordinator FROM sessions WHERE session_id = ?", SESSION), [(1,)])

    def test_fingerprints(self):
        got = dict(self.q("SELECT tool_use_id, fingerprint FROM tool_calls"))
        self.assertEqual(got, self.expected["tool_fingerprints"])

    def test_no_hostile_value_reaches_a_text_column_a_report_reads(self):
        rows = self.q("SELECT fingerprint, mcp_server, tool_name FROM tool_calls UNION ALL"
                      " SELECT source_ref, origin_kind, kind FROM triggers UNION ALL"
                      " SELECT git_branch, cwd_worktree, model FROM requests")
        blob = json.dumps(rows)
        for value in self.expected["hostile"]:
            self.assertNotIn(json.dumps(value)[1:-1], blob, value)

    def test_compaction_is_modelled(self):
        rows = self.q("SELECT trigger, pre_tokens, post_tokens, dropped_tokens, duration_ms, request_before,"
                      " request_after FROM compactions")
        self.assertEqual(len(rows), self.expected["compactions"])
        self.assertEqual(rows[0], ("auto", 65200, 9000, 56200, 30000, "req_U", "req_I"))

    def test_context_exposure_stops_at_the_compaction(self):
        chars, later, exposure = self.q("SELECT result_chars, later_requests, exposure_chars FROM tool_calls"
                                        " WHERE tool_use_id = 'toolu_fixture_bash'")[0]
        # req_B .. req_U follow req_A in the main transcript before the compaction; req_I is after it.
        self.assertEqual(later, 17)
        self.assertEqual(exposure, chars * later)
        self.assertGreater(chars, 5000)

    def test_pr_links_and_cost_state(self):
        self.assertEqual([n for (n,) in self.q("SELECT pr_number FROM pr_links")], self.expected["pr_links"])
        self.assertEqual(self.q("SELECT total_cost_usd FROM cost_states"), [(self.expected["cost_state_total_usd"],)])

    def test_unknown_shape_is_counted_and_fails_the_run(self):
        self.assertEqual(self.q("SELECT shape, count FROM unknown_shapes"),
                         [(s, 1) for s in self.expected["unknown_shapes"]])
        self.assertEqual(self.status, 0)        # --allow-unknown
        self.assertEqual(self.status_strict, 1)
        self.assertEqual(self.status_again, 1)  # still failing on a re-run with nothing new

    def test_run_is_recorded(self):
        row = self.q("SELECT price_table, profiler_commit, status, records_read, unknown_records FROM profiler_runs")
        self.assertEqual(row[0][:3], ("2026-10-03", "test", "ok"))
        self.assertEqual(row[0][4], 1)
        self.assertGreater(row[0][3], 0)

    def test_every_request_is_fully_attributed(self):
        self.assertEqual(self.q("SELECT COUNT(*) FROM attribution_imbalance"), [(0,)])

    def test_fixture_attribution(self):
        got = {rid: (pr_number, method) for rid, pr_number, method in
               self.q("SELECT request_id, pr_number, method FROM pr_attribution")}
        # The worker's own branch is PR 4242's head.
        self.assertEqual(got["req_S1"], (4242, "branch"))
        # Coordinator turns started by that worker: A1.
        for rid in ("req_C", "req_E", "req_K"):
            self.assertEqual(got[rid], (4242, "trigger"), rid)
        # The turn that wrote the pr-link record, and the turn that ran `gh pr checks 4242`: A5.
        for rid in ("req_A", "req_B", "req_G"):
            self.assertEqual(got[rid], (4242, "pr-link"), rid)
        # Human turns on main with no PR activity stay unattributed.
        self.assertEqual(got["req_O"], (None, "unattributed"))


if __name__ == "__main__":
    unittest.main()
