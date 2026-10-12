"""The GitHub collector on a four-PR fixture: CI verdicts, escaped defects, privacy of the output.

No test touches the network: every gh call goes to replay(fixtures/) or to a small fake.
"""

import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

from ingest.campaigns import Campaigns

from . import github as gh
from .cli import main

FIXTURES = Path(__file__).resolve().parent / "fixtures"
CAMPAIGN_OF = {101: "lab-record", 102: "lab-record", 103: "docs", 104: "pr-104"}.get


class GithubTests(unittest.TestCase):
    def setUp(self):
        self.run = gh.replay(FIXTURES)
        self.prs = gh.collect_prs(self.run, "2026-09-21")
        self.by_number = {p["number"]: p for p in self.prs}

    def ci(self, number):
        pr = self.by_number[number]
        commits = gh.head_commits(self.run, pr)
        results = {c["sha"]: gh.ci_result(self.run, c["sha"]) for c in commits}
        return gh.ci_summary(pr, commits, results)

    def test_green_first_try_is_first_attempt_green(self):
        self.assertEqual(self.ci(101), {"first_attempt": True, "overall": True, "runs": 1})

    def test_red_then_green_takes_the_earliest_gated_commit_for_first_attempt(self):
        # Reverting "earliest gated commit" to "last commit" makes first_attempt True here.
        self.assertEqual(self.ci(102), {"first_attempt": False, "overall": True, "runs": 2})

    def test_no_gate_is_none_not_a_failure(self):
        self.assertEqual(self.ci(103), {"first_attempt": None, "overall": None, "runs": 0})

    def test_failed_gate_names_the_failed_job_only(self):
        self.assertEqual(gh.ci_result(self.run, "b101"), {"gate": "failure", "failed_jobs": ["build"]})

    def test_fix_pr_is_an_escaped_defect_of_the_campaign_owning_pr_101(self):
        defects = gh.escaped_defects(self.prs, gh.collect_issues(self.run, "2026-09-21"), CAMPAIGN_OF)
        self.assertEqual(
            defects["lab-record"],
            [{"kind": "pr", "number": 104, "cites": 101, "days_after_merge": 2.9},
             {"kind": "issue", "number": 900, "cites": 101, "days_after_merge": 3.9}],
        )
        self.assertNotIn("pr-104", defects, "a fix of its own campaign's PR is not an escaped defect")

    def test_fix_in_the_same_campaign_is_not_an_escaped_defect(self):
        same = lambda n: "lab-record"  # noqa: E731
        self.assertEqual(gh.escaped_defects(self.prs, [], same), {})

    def test_revert_quoting_a_campaign_pr_title_is_an_escaped_defect(self):
        prs = [
            {"number": 1, "title": "feat(x): one", "body": "", "mergedAt": "2026-10-01T10:00:00Z"},
            {"number": 2, "title": 'Revert "feat(x): one"', "body": "", "mergedAt": "2026-10-02T10:00:00Z"},
        ]
        defects = gh.escaped_defects(prs, [], lambda n: "x" if n == 1 else "y")
        self.assertEqual(defects, {"x": [{"kind": "revert", "number": 2, "cites": 1, "days_after_merge": 1.0}]})

    def test_branch_names_are_dropped_from_the_pr_records(self):
        for pr in self.prs:
            self.assertNotIn("headRefName", pr)

    def test_code_pr_is_true_when_a_crates_tools_db_or_ci_path_changed(self):
        self.assertTrue(gh.code_pr(self.run, 101))
        self.assertFalse(gh.code_pr(self.run, 103))

    def test_campaign_tags_come_from_head_branch_then_title_then_base(self):
        campaigns = Campaigns({"lab-record": {"prefixes": ["lab/"]}})
        tagged = {p["number"]: p["campaign"] for p in gh.collect_prs(self.run, "2026-09-21", campaigns)}
        self.assertEqual(tagged, {101: "lab-record", 102: "lab-record", 103: None, 104: None})

    def test_commits_are_capped_at_one_hundred(self):
        commits = [{"sha": f"s{i}", "commit": {"committer": {"date": "2026-10-01T00:00:00Z"}}} for i in range(150)]
        run = lambda args: [commits]  # noqa: E731
        self.assertEqual(len(gh.head_commits(run, {"number": 1})), gh.MAX_COMMITS)

    def test_window_end_is_searched_and_drops_items_created_at_or_after_it(self):
        searches = []

        def run(args):
            searches.append(args)
            if args[:2] == ["pr", "list"]:
                return [{"number": 1, "createdAt": "2026-10-12T00:00:00Z"},
                        {"number": 2, "createdAt": "2026-10-13T00:00:00Z"}]
            return []

        prs = gh.collect_prs(run, "2026-09-21", None, "2026-10-13T00:00:00Z")
        self.assertEqual([p["number"] for p in prs], [1])
        self.assertIn("created:>=2026-09-21 created:<2026-10-13", searches[0])

    def test_a_retryable_error_waits_and_is_retried(self):
        calls = []

        def run(args):
            calls.append(args)
            if len(calls) < 3:
                raise gh.GhError("limited", retryable=True)
            return []

        with mock.patch.object(gh.time, "sleep") as sleep:
            self.assertEqual(gh.polite(run)(["api", "x"]), [])
        waits = [c.args[0] for c in sleep.call_args_list if c.args[0] != gh.PAUSE_S]
        self.assertEqual(waits, list(gh.RETRY_WAITS_S[:2]))

    def test_a_non_retryable_error_raises_at_once(self):
        def run(args):
            raise gh.GhError("bad")

        with mock.patch.object(gh.time, "sleep") as sleep, self.assertRaises(gh.GhError):
            gh.polite(run)(["api", "x"])
        self.assertEqual([c.args[0] for c in sleep.call_args_list], [gh.PAUSE_S])

    def test_a_secondary_limit_is_retryable_when_its_status_is_past_the_truncated_message(self):
        result = SimpleNamespace(returncode=1, stdout="", stderr="x" * 400 + " (HTTP 403)")
        with mock.patch.object(gh.subprocess, "run", return_value=result), self.assertRaises(gh.GhError) as ctx:
            gh.run_gh(["api", "x"])
        self.assertTrue(ctx.exception.retryable)

    def test_a_cached_api_answer_is_fetched_once(self):
        calls = []

        def run(args):
            calls.append(args)
            return []

        stats = {"gh_calls": 0, "cache_hits": 0}
        with tempfile.TemporaryDirectory() as tmp, mock.patch.object(gh.time, "sleep"):
            call = gh.polite(run, cache_dir=tmp, stats=stats)
            call(["api", "x"])
            call(["api", "x"])
        self.assertEqual(len(calls), 1)
        self.assertEqual(stats, {"gh_calls": 1, "cache_hits": 1})

    def test_cancelled_and_skipped_gates_are_none_and_a_timed_out_gate_is_a_failure(self):
        def gate(conclusion):
            return lambda args: [{"check_runs": [{"name": gh.GATE, "conclusion": conclusion}]}]

        for conclusion in ("cancelled", "skipped", None):
            with self.subTest(conclusion=conclusion):
                self.assertEqual(gh.ci_result(gate(conclusion), "x")["gate"], "none")
        self.assertEqual(gh.ci_result(gate("timed_out"), "x")["gate"], "failure")

    def test_a_cancelled_first_push_is_not_a_failed_first_attempt(self):
        commits = [{"sha": "A", "committed": "2026-10-01T00:00:00Z"}, {"sha": "B", "committed": "2026-10-01T01:00:00Z"}]
        results = {"A": {"gate": "none", "failed_jobs": []}, "B": {"gate": "success", "failed_jobs": []}}
        self.assertEqual(gh.ci_summary({"mergedAt": None}, commits, results),
                         {"first_attempt": True, "overall": True, "runs": 1})

    def test_untagged_prs_are_never_an_escaped_defect(self):
        prs = [{"number": 1, "title": "feat: a", "body": "", "mergedAt": "2026-10-01T00:00:00Z"},
               {"number": 2, "title": "fix(x): y (#1)", "body": "", "mergedAt": "2026-10-02T00:00:00Z"}]
        self.assertEqual(gh.escaped_defects(prs, [], lambda n: None), {})


class CliSmokeTest(unittest.TestCase):
    def test_replay_writes_github_json_and_a_count_summary(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                code = main(["github", "--repo", ".", "--out", tmp, "--from-json", str(FIXTURES)])
            record = json.loads((Path(tmp) / "github.json").read_text(encoding="utf-8"))
        self.assertEqual(code, 0)
        self.assertEqual(json.loads(out.getvalue()), {
            "since": "2026-09-21", "prs_collected": 4, "merged": 4, "ci_checked": 3, "untagged_prs": 4,
            "escaped_defects": {}, "gh_calls": 0, "cache_hits": 0,
        })
        self.assertEqual([r["ci"]["runs"] for r in record["prs"]], [1, 2, 0, 1])

    def test_campaigns_file_tags_the_prs_it_names(self):
        with tempfile.TemporaryDirectory() as tmp:
            rules = Path(tmp) / "campaigns.json"
            rules.write_text(json.dumps({"lab-record": {"prefixes": ["lab/"]}}), encoding="utf-8")
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                main(["github", "--repo", ".", "--out", tmp, "--from-json", str(FIXTURES), "--campaigns", str(rules)])
        self.assertEqual(json.loads(out.getvalue())["escaped_defects"], {"lab-record": 1})

    def test_ledger_pr_map_tags_the_prs_it_names(self):
        with tempfile.TemporaryDirectory() as tmp:
            ledger = Path(tmp) / "ledger.json"
            ledger.write_text(json.dumps({"pr_map": {"101": "lab-record"}}), encoding="utf-8")
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                main(["github", "--repo", ".", "--out", tmp, "--from-json", str(FIXTURES), "--ledger", str(ledger)])
            record = json.loads((Path(tmp) / "github.json").read_text(encoding="utf-8"))
        self.assertEqual(json.loads(out.getvalue())["escaped_defects"], {"lab-record": 1})
        self.assertNotIn("headRefName", json.dumps(record))


if __name__ == "__main__":
    unittest.main()
