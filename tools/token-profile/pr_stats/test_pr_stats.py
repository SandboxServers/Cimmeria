"""Per-PR stats comments over the synthetic fixture, with a fake `gh` that never touches the network."""

import io
import json
import re
import sqlite3
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest import mock

from report import fixture_db
from report.db import open_db, scope
from report.scrub import Scrubber
from report.sections.prs import pr_records

from . import backfill as bf
from . import block, cli
from .github import MARKER, GitHub, quality

REPO = "SandboxServers/Cimmeria"
PR, OTHER, EMPTY = fixture_db.PR_MAIN, fixture_db.PR_OTHER, 4299
# The plan's key order (issue #957, "Per-PR comment format"). New keys may only be appended.
PLAN_KEYS = ["schema", "pr", "profiler", "claude_code", "price_table", "window", "attribution", "usd_est",
             "by_model", "tokens", "requests", "turns", "human_prompts", "event_triggers", "agents", "context",
             "tools", "quality", "diff"]


class FakeGh:
    """Answers the gh calls github.py makes, and keeps the PRs' comments."""

    def __init__(self, runs=None, timeline=None, comments=None, fail_on=None):
        self.calls = []
        self.runs = runs if runs is not None else [
            {"head_sha": "aaa", "conclusion": "failure"}, {"head_sha": "aaa", "conclusion": "success"},
            {"head_sha": "bbb", "conclusion": "success"}, {"head_sha": "zzz", "conclusion": "failure"}]
        self.timeline = timeline or []
        self.comments = comments or {}
        self.fail_on = fail_on
        self.next_id = 100

    def pr(self, n):
        return {"number": n, "state": "MERGED", "mergedAt": "2026-10-01T13:00:00Z", "headRefName": f"feat/pr-{n}",
                "additions": 120, "deletions": 30, "changedFiles": 4,
                "commits": [{"oid": "aaa"}, {"oid": "bbb"}],
                "reviews": [{"state": "COMMENTED", "commit": {"oid": "aaa"}, "submittedAt": "x"},
                            {"state": "APPROVED", "commit": {"oid": "aaa"}, "submittedAt": "y"},
                            {"state": "PENDING", "commit": {"oid": "bbb"}, "submittedAt": None}]}

    @property
    def writes(self):
        return [c for c in self.calls if "-X" in c[0]]

    def __call__(self, args, stdin=None):
        self.calls.append((list(args), stdin))
        if self.fail_on and self.fail_on(args):
            from .github import GhError
            raise GhError("gh api exited 1: rate limited")
        if args[:2] == ["pr", "view"]:
            return json.dumps(self.pr(int(args[2])))
        if args[0] == "api" and "-X" in args:
            method, path = args[2], args[3]
            body = json.loads(stdin)["body"]
            if method == "POST":
                n = int(path.split("/")[-2])
                self.next_id += 1
                self.comments.setdefault(n, []).append({"id": self.next_id, "body": body})
                return json.dumps({"id": self.next_id})
            cid = int(path.split("/")[-1])
            for cs in self.comments.values():
                for c in cs:
                    if c["id"] == cid:
                        c["body"] = body
            return json.dumps({"id": cid})
        path = args[1]
        if "/actions/runs" in path:
            return json.dumps([{"total_count": len(self.runs), "workflow_runs": self.runs}])
        if "/timeline" in path:
            return json.dumps([self.timeline])
        if "/comments" in path:
            n = int(re.search(r"issues/(\d+)/comments", path).group(1))
            return json.dumps([self.comments.get(n, [])])
        raise AssertionError(f"unexpected gh call {args}")


def run_cli(argv, gh, limiter=None):
    out, err = io.StringIO(), io.StringIO()
    with redirect_stdout(out), redirect_stderr(err):
        code = cli.main(argv, gh_run=gh, limiter=limiter)
    return code, out.getvalue(), err.getvalue()


class Fixture(unittest.TestCase):
    hostile = False

    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.db_path, cls.expected = fixture_db.build(cls.tmp.name, hostile=cls.hostile)
        db = sqlite3.connect(cls.db_path)
        db.execute(fixture_db.PR_INSERT + " VALUES (?, 'docs/empty', NULL, NULL, '2026-10-01T11:00:00.000Z',"
                   " '2026-10-02T00:00:00.000Z', '2026-10-02T00:00:00.000Z', 'MERGED', 1, 1, 1)", (EMPTY,))
        db.commit()
        db.close()

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def argv(self, *args):
        return [*map(str, args), "--db", str(self.db_path), "--repo", REPO]

    def body(self, gh=None, pr=PR):
        code, out, err = run_cli(self.argv(pr), gh or FakeGh())
        self.assertEqual(code, 0, err)
        return out


class BlockTest(Fixture):
    def test_block_has_the_plan_schema(self):
        b = block.parse(self.body())
        self.assertEqual(list(b)[:len(PLAN_KEYS)], PLAN_KEYS)
        self.assertEqual(b["schema"], "cimmeria-pr-stats/1")
        self.assertEqual(b["pr"], PR)
        self.assertEqual(set(b["tokens"]), {"input", "output", "thinking", "cache_read", "cache_write_5m",
                                            "cache_write_1h"})
        self.assertEqual(set(b["attribution"]), {"method", "confidence", "unattributed_share"})
        self.assertEqual(set(b["context"]), {"peak", "p50", "compactions", "idle_gap_write_share"})
        self.assertEqual(set(b["tools"]), {"bash_chars", "read_chars", "top_exposure"})
        self.assertEqual(list(b["quality"])[:4], ["ci_fail_rounds", "review_rounds", "followup_fix_prs", "reverted"])
        self.assertEqual(set(b["diff"]), {"additions", "deletions", "files"})
        self.assertEqual(set(b["window"]), {"first", "merged"})

    def test_comment_opens_with_the_marker_and_round_trips(self):
        body = self.body()
        self.assertTrue(body.startswith(MARKER + "\n"))
        self.assertIn("| est. USD (list) | requests | human prompts | agents | peak ctx | wall clock | CI rounds |",
                      body)
        b = block.parse(body)
        self.assertEqual(block.render(b), body.rstrip("\n") + "\n")
        self.assertIsNone(block.parse("no marker\n```json\n{\"schema\":\"cimmeria-pr-stats/1\"}\n```"))

    def test_database_fields_match_pr_records_and_the_stamp(self):
        b = block.parse(self.body())
        db = open_db(self.db_path)
        scope(db)
        want = pr_records(db, Scrubber(use_local=False), [PR])[0]
        db.close()
        for key in ("usd_est", "by_model", "tokens", "requests", "turns", "human_prompts", "attribution",
                    "context", "agents"):
            self.assertEqual(b[key], json.loads(json.dumps(want[key])), key)
        self.assertEqual(b["profiler"], fixture_db.PROFILER_COMMIT[:12])
        self.assertEqual(b["price_table"], fixture_db.PRICE_VERSION)
        self.assertEqual(b["claude_code"], "2.1.999")
        self.assertGreater(b["usd_est"], 0)

    def test_quality_and_diff_come_from_gh(self):
        b = block.parse(self.body())
        # aaa failed once, bbb passed, zzz is not this PR's commit; reviews: one round on aaa.
        self.assertEqual(b["quality"], {"ci_fail_rounds": 1, "review_rounds": 1, "followup_fix_prs": [],
                                        "reverted": False, "ci_rounds": 2})
        self.assertEqual(b["diff"], {"additions": 120, "deletions": 30, "files": 4})
        self.assertIn("| 1 failed of 2 |", self.body())

    def test_untyped_teammate_name_never_reaches_the_comment(self):
        # Claude Code writes an in-process teammate's name as its meta.agentType.
        copy = Path(self.tmp.name) / "teammate.sqlite"
        copy.write_bytes(Path(self.db_path).read_bytes())
        db = sqlite3.connect(copy)
        db.execute("UPDATE agents SET custom_agent_type = NULL, agent_type = 'secret-teammate',"
                   " name = 'secret-teammate', task_kind = 'in_process_teammate'")
        db.commit()
        db.close()
        code, body, err = run_cli([str(PR), "--db", str(copy)], FakeGh())
        self.assertEqual(code, 0, err)
        self.assertNotIn("secret-teammate", body)
        self.assertIn("teammate", block.parse(body)["agents"])

    def test_a_pr_with_no_attributed_data_posts_nothing(self):
        gh = FakeGh()
        code, out, err = run_cli(self.argv(EMPTY, "--post"), gh)
        self.assertEqual(code, 4)
        self.assertEqual(out, "")
        self.assertIn("status=no-data", err)
        self.assertEqual(gh.calls, [])  # not even a read: the database is asked first

    def test_a_missing_database_is_an_error(self):
        code, _, err = run_cli([str(PR), "--db", str(Path(self.tmp.name) / "nope.sqlite")], FakeGh())
        self.assertEqual(code, 2)
        self.assertIn("status=error", err)

    def test_usage(self):
        with redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit):
                cli.main(["--db", "x"])
            with self.assertRaises(SystemExit):
                cli.main(["1", "--backfill", "--db", "x"])


class IdempotencyTest(Fixture):
    def test_dry_run_writes_nothing(self):
        gh = FakeGh()
        self.body(gh)
        self.assertEqual(gh.writes, [])
        self.assertFalse(any("/comments" in " ".join(c[0]) for c in gh.calls))

    def test_second_post_edits_never_creates(self):
        gh = FakeGh()
        code, _, err = run_cli(self.argv(PR, "--post"), gh)
        self.assertEqual((code, len(gh.comments[PR])), (0, 1), err)
        self.assertIn("status=created", err)

        code, _, err = run_cli(self.argv(PR, "--post"), gh)
        self.assertIn("status=unchanged", err)
        self.assertEqual(len(gh.writes), 1)  # identical body: no PATCH either

        gh.runs.append({"head_sha": "bbb", "conclusion": "timed_out"})
        code, _, err = run_cli(self.argv(PR, "--post"), gh)
        self.assertEqual(code, 0)
        self.assertIn("status=updated", err)
        self.assertEqual(len(gh.comments[PR]), 1)
        self.assertEqual([c[0][2] for c in gh.writes], ["POST", "PATCH"])
        self.assertEqual(block.parse(gh.comments[PR][0]["body"])["quality"]["ci_fail_rounds"], 2)

    def test_existing_duplicates_are_left_and_the_oldest_is_edited(self):
        gh = FakeGh(comments={PR: [{"id": 1, "body": "unrelated"}, {"id": 2, "body": MARKER + "\nold"},
                                   {"id": 3, "body": MARKER + "\nolder bug"}]})
        code, _, err = run_cli(self.argv(PR, "--post"), gh)
        self.assertEqual(code, 0)
        self.assertIn("status=updated comment=2 duplicates=1", err)
        self.assertEqual([c[0][2] for c in gh.writes], ["PATCH"])
        self.assertEqual([c["body"] for c in gh.comments[PR]][0], "unrelated")
        self.assertTrue(gh.comments[PR][1]["body"].startswith(MARKER + "\n**Token profile**"))

    def test_a_marker_quoted_mid_comment_is_not_ours(self):
        gh = FakeGh(comments={PR: [{"id": 7, "body": "see " + MARKER}]})
        run_cli(self.argv(PR, "--post"), gh)
        self.assertEqual([c[0][2] for c in gh.writes], ["POST"])

    def test_gh_failure_is_an_error_and_posts_nothing(self):
        gh = FakeGh(fail_on=lambda a: "/timeline" in " ".join(a))
        code, _, err = run_cli(self.argv(PR, "--post"), gh)
        self.assertEqual(code, 2)
        self.assertIn("status=error", err)
        self.assertEqual(gh.writes, [])


class HostileTest(Fixture):
    """A broken ingest full of private values: the comment carries none of them, or is refused."""

    hostile = True

    def test_no_hostile_value_reaches_the_comment(self):
        gh = FakeGh(timeline=[{"event": "cross-referenced", "created_at": "2026-10-02T00:00:00Z",
                               "source": {"issue": {"number": 77, "title": "fix: leak me@example.com",
                                                    "repository_url": f"https://api.github.com/repos/{REPO}",
                                                    "pull_request": {"merged_at": "2026-10-02T00:00:00Z"}}}}])
        with mock.patch.dict("os.environ", {"TOKEN_PROFILE_DENY": "Steve"}):
            body = self.body(gh)
        for value in self.expected["hostile"] + ["Steve", "example.internal", "secret-project", "me@example.com"]:
            self.assertNotIn(value.lower(), body.lower())
        b = block.parse(body)
        self.assertEqual(b["quality"]["followup_fix_prs"], [77])
        self.assertEqual(b["profiler"], "<invalid>")  # the hostile commit fails field validation

    def test_the_gate_refuses_and_posts_nothing(self):
        gh = FakeGh()
        leaky = block.render
        with mock.patch.object(block, "render", lambda b: leaky(b) + "\nhost 10.1.2.3\n"):
            code, out, err = run_cli(self.argv(PR, "--post"), gh)
        self.assertEqual(code, 3)
        self.assertEqual(out, "")
        self.assertIn("status=refused", err)
        self.assertIn("ipv4", err)
        self.assertNotIn("10.1.2.3", err)  # the gate names the detector, never the value
        self.assertEqual(gh.writes, [])

    def test_deny_words_are_gated(self):
        code, out, err = run_cli(self.argv(PR, "--deny", "Token"), FakeGh())
        # "Token profile" is in every comment's text, which only the gate sees.
        self.assertEqual(code, 3)
        self.assertIn("deny-word", err)


class QualityTest(unittest.TestCase):
    def xref(self, number, title, created, merged="2026-10-02T00:00:00Z", repo=REPO):
        return {"event": "cross-referenced", "created_at": created,
                "source": {"issue": {"number": number, "title": title,
                                     "repository_url": f"https://api.github.com/repos/{repo}",
                                     "pull_request": {"merged_at": merged} if merged is not None else None}}}

    def test_followups_and_reverts(self):
        pr = {"mergedAt": "2026-10-01T00:00:00Z", "commits": [], "reviews": []}
        timeline = [
            self.xref(10, "fix(cell): follow-up", "2026-10-01T05:00:00Z"),
            self.xref(11, "fix: references it before it merged", "2026-09-30T00:00:00Z"),
            self.xref(12, "feat: not a fix", "2026-10-01T05:00:00Z"),
            self.xref(13, "fix: never merged", "2026-10-01T05:00:00Z", merged=None),
            self.xref(14, "fix: another repo", "2026-10-01T05:00:00Z", repo="someone/else"),
            self.xref(1, "fix: itself", "2026-10-01T05:00:00Z"),
            {"event": "labeled"},
        ]
        q = quality(1, REPO, pr, [], timeline)
        self.assertEqual(q["followup_fix_prs"], [10])
        self.assertFalse(q["reverted"])
        q = quality(1, REPO, pr, [], timeline + [self.xref(15, 'Revert "feat: x"', "2026-10-02T00:00:00Z")])
        self.assertTrue(q["reverted"])

    def test_cancelled_runs_are_not_failures(self):
        pr = {"commits": [{"oid": "a"}], "reviews": []}
        q = quality(1, REPO, pr, [{"head_sha": "a", "conclusion": "cancelled"}], [])
        self.assertEqual((q["ci_rounds"], q["ci_fail_rounds"]), (1, 0))

    def test_gh_wrapper_builds_the_calls(self):
        gh = FakeGh()
        GitHub(REPO, gh).quality(5)
        paths = [c[0][1] if c[0][0] == "api" else " ".join(c[0][:3]) for c in gh.calls]
        self.assertEqual(paths, ["pr view 5", f"repos/{REPO}/actions/runs?branch=feat%2Fpr-5&per_page=100",
                                 f"repos/{REPO}/issues/5/timeline?per_page=100"])


class RateTest(unittest.TestCase):
    def test_parse_rate(self):
        self.assertEqual(bf.parse_rate("6"), 6)
        self.assertEqual(bf.parse_rate("6/min"), 6)
        self.assertEqual(bf.parse_rate("120/h"), 2)
        for bad in ("0", "-1", "fast", "6/s"):
            with self.assertRaises(ValueError):
                bf.parse_rate(bad)

    def test_limiter_spaces_calls(self):
        now, slept = [0.0], []

        def sleep(s):
            slept.append(s)
            now[0] += s
        limiter = bf.RateLimiter(6, clock=lambda: now[0], sleep=sleep)
        limiter.wait()
        now[0] += 4
        limiter.wait()
        now[0] += 30
        limiter.wait()
        self.assertEqual(slept, [6.0])  # 10 s apart: 4 s elapsed, so 6 s more; then 30 s is enough


class BackfillTest(Fixture):
    def setUp(self):
        self.state = Path(self.tmp.name) / f"state-{self._testMethodName}.json"
        self.waits = []

    def limiter(self):
        limiter = mock.Mock()
        limiter.wait.side_effect = lambda: self.waits.append(1)
        return limiter

    def backfill(self, gh, *extra):
        return run_cli(self.argv("--backfill", "--since", "2026-09-13", "--state", self.state, *extra), gh,
                       limiter=self.limiter())

    def test_dry_run_is_the_default_and_rate_limits_only_gh_work(self):
        gh = FakeGh()
        code, out, err = self.backfill(gh, "--out", Path(self.tmp.name) / "bodies")
        self.assertEqual(code, 0, err)
        self.assertEqual(out, "")
        self.assertIn(f"pr={PR} status=printed", err)
        self.assertIn(f"pr={OTHER} status=printed", err)
        self.assertIn(f"pr={EMPTY} status=no-data", err)
        self.assertEqual(gh.writes, [])
        self.assertEqual(len(self.waits), 2)  # the no-data PR costs no gh call and no wait
        self.assertEqual(block.parse((Path(self.tmp.name) / "bodies" / f"{PR}.md").read_text())["pr"], PR)

    def test_post_is_resumable_and_idempotent(self):
        gh = FakeGh(fail_on=lambda a: "-X" in a and f"issues/{OTHER}/" in a[3])
        code, _, err = self.backfill(gh, "--post")
        self.assertEqual(code, 2)
        self.assertIn(f"pr={PR} status=created", err)
        self.assertIn(f"pr={OTHER} status=error", err)

        gh.fail_on = None
        self.waits.clear()
        code, _, err = self.backfill(gh, "--post")
        self.assertEqual(code, 0, err)
        self.assertNotIn(f"pr={PR} ", err)  # done last time: not redone
        self.assertIn(f"pr={OTHER} status=created", err)
        self.assertIn("skipped=2", err)  # PR and the no-data PR
        self.assertEqual(len(self.waits), 1)
        self.assertEqual({n: len(c) for n, c in gh.comments.items()}, {PR: 1, OTHER: 1})

        code, _, err = self.backfill(gh, "--post", "--restart")
        self.assertIn("unchanged=2", err)
        self.assertEqual({n: len(c) for n, c in gh.comments.items()}, {PR: 1, OTHER: 1})

    def test_stops_after_errors_in_a_row(self):
        outcomes = iter(["created", "error", "error", "no-data", "error", "error", "error", "created"])
        lines = []
        state = bf.State(self.state)
        counts = bf.backfill(range(1, 9), lambda pr, wait: next(outcomes), self.limiter(), state, "post",
                             lines.append)
        self.assertEqual(counts, {"created": 1, "error": 5, "no-data": 1, "stopped": 1})
        self.assertIn("status=stopped", lines[-1])
        self.assertEqual(bf.State(self.state).data["outcomes"]["post"],
                         {"1": "created", "2": "error", "3": "error", "4": "no-data", "5": "error", "6": "error",
                          "7": "error"})

    def test_a_gh_outage_posts_nothing(self):
        gh = FakeGh(fail_on=lambda a: a[:2] == ["pr", "view"])
        code, _, err = self.backfill(gh, "--post")
        self.assertEqual(code, 2)
        self.assertEqual(gh.writes, [])
        self.assertEqual(json.loads(self.state.read_text())["outcomes"]["post"][str(PR)], "error")

    def test_limit(self):
        code, _, err = self.backfill(FakeGh(), "--limit", "1")
        self.assertIn(f"pr={PR} status=printed", err)
        self.assertNotIn(f"pr={OTHER}", err)


if __name__ == "__main__":
    unittest.main()
