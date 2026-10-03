"""Attribution rules A1-A6 on small synthetic sessions, and the imbalance check."""

import os
import sqlite3
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from . import attribution
from .test_support import pr, run_ingest, transcript, write_session
from .test_support import build_fixtures as bf

S1 = "11111111-0000-4000-8000-000000000001"
S2 = "22222222-0000-4000-8000-000000000002"
S3 = "33333333-0000-4000-8000-000000000003"
T = bf.ts


def usage():
    return bf.usage(1, 10, 0, 100, 10, 0)


def day(d, hour=12):
    return f"2026-10-{d:02d}T{hour:02d}:00:00.000Z"


class AttributionTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.base = Path(self.tmp.name)
        self.projects = self.base / "projects"

    def tearDown(self):
        self.tmp.cleanup()

    def ingest(self, prs, repo=None):
        db = self.base / "a.sqlite"
        status = run_ingest(self.projects, db, prs=prs, repo=repo)
        conn = sqlite3.connect(db)
        rows = {}
        for rid, pr_number, method, weight, conf in conn.execute(
                "SELECT request_id, pr_number, method, weight, confidence FROM pr_attribution"):
            rows.setdefault(rid, []).append((pr_number, method, round(weight, 6), conf))
        conn.close()
        self.assertEqual(status, 0)
        return {k: sorted(v, key=repr) for k, v in rows.items()}

    def test_branch_window_picks_the_pr_when_a_branch_name_is_reused(self):
        m = transcript(S1, branch="feat/reused")
        for rid, when in (("r_old", day(2)), ("r_gap", day(6)), ("r_new", day(11)), ("r_early", day(9, 13))):
            m.user_text(when, "go", origin={"kind": "human"})
            bf.request(m, when, rid, usage())
        write_session(self.projects, S1, m)
        got = self.ingest([pr(50, "feat/reused", day(1), merged=day(3)), pr(51, "feat/reused", day(10))])
        self.assertEqual(got["r_old"], [(50, "branch", 1.0, 1.0)])
        self.assertEqual(got["r_new"], [(51, "branch", 1.0, 1.0)])
        self.assertEqual(got["r_early"], [(51, "branch", 1.0, 1.0)])   # within 24 h before PR 51 opened
        self.assertEqual(got["r_gap"], [(None, "unattributed", 1.0, 0.0)])

    def test_main_never_matches_a_branch(self):
        m = transcript(S1, branch="main")
        m.user_text(T(1), "go", origin={"kind": "human"})
        bf.request(m, T(1, 5), "r_main", usage())
        write_session(self.projects, S1, m)
        got = self.ingest([pr(60, "main", "2026-09-30T00:00:00Z")])
        self.assertEqual(got["r_main"], [(None, "unattributed", 1.0, 0.0)])

    def test_pr_links_for_two_prs_split_the_turn(self):
        m = transcript(S1)
        m.user_text(T(1), "merge both", origin={"kind": "human"})
        bf.request(m, T(1, 5), "r_both", usage())
        for n in (40, 41):
            m.raw({"type": "pr-link", "sessionId": S1, "prNumber": n, "prUrl": "u", "prRepository": "o/r",
                   "timestamp": T(1, 10)})
        m.user_text(T(2), "next", origin={"kind": "human"})
        bf.request(m, T(2, 5), "r_next", usage())
        write_session(self.projects, S1, m)
        got = self.ingest([pr(40, "a", day(1)), pr(41, "b", day(1))])
        self.assertEqual(got["r_both"], [(40, "split", 0.5, 0.6), (41, "split", 0.5, 0.6)])
        self.assertEqual(got["r_next"], [(None, "unattributed", 1.0, 0.0)])

    def test_pr_number_not_in_prs_is_never_a_target(self):
        m = transcript(S1)
        m.user_text(T(1), "x", origin={"kind": "human"})
        bf.request(m, T(1, 5), "r_x", usage(), content=[
            {"type": "tool_use", "id": "tu_gh", "name": "Bash", "input": {"command": "gh pr merge 999 --squash"}}])
        m.tool_result(T(1, 6), "tu_gh", "merged")
        write_session(self.projects, S1, m)
        got = self.ingest([pr(40, "a", day(1))])
        self.assertEqual(got["r_x"], [(None, "unattributed", 1.0, 0.0)])

    def test_gh_pr_create_takes_the_number_from_its_result(self):
        m = transcript(S1)
        m.user_text(T(1), "open it", origin={"kind": "human"})
        bf.request(m, T(1, 5), "r_create", usage(), content=[
            {"type": "tool_use", "id": "tu_c", "name": "Bash", "input": {"command": "gh pr create --title t"}}])
        m.tool_result(T(1, 6), "tu_c", "https://github.com/o/r/pull/77\n")
        write_session(self.projects, S1, m)
        got = self.ingest([pr(77, "x", day(1))])
        self.assertEqual(got["r_create"], [(77, "pr-link", 1.0, 0.6)])

    def test_subagent_inherits_its_parent_turn(self):
        m = transcript(S1)
        m.user_text(T(1), "spawn", origin={"kind": "human"})
        bf.request(m, T(1, 5), "r_spawn", usage(), content=[
            {"type": "tool_use", "id": "tu_agent", "name": "Agent", "input": {"prompt": "p"}}])
        rec = m.tool_result(T(1, 6), "tu_agent", "launched")
        rec["toolUseResult"] = {"agentId": "aSub", "status": "async_launched"}
        m.raw({"type": "pr-link", "sessionId": S1, "prNumber": 20, "prUrl": "u", "prRepository": "o/r",
               "timestamp": T(1, 7)})
        s = transcript(S1, agent_id="aSub", branch="research/no-pr")
        s.user_text(T(2), "do it", origin={"kind": "coordinator"})
        bf.request(s, T(2, 5), "r_sub", usage())
        write_session(self.projects, S1, m, agents=[("aSub", s, None)])
        got = self.ingest([pr(20, "feat/x", day(1))])
        self.assertEqual(got["r_spawn"], [(20, "pr-link", 1.0, 0.6)])
        self.assertEqual(got["r_sub"], [(20, "parent-session", 1.0, 0.7)])

    def test_subagent_meta_missing_is_flagged(self):
        m = transcript(S1)
        m.user_text(T(1), "x", origin={"kind": "human"})
        bf.request(m, T(1, 5), "r1", usage())
        s = transcript(S1, agent_id="aNoMeta")
        s.user_text(T(2), "x", origin={"kind": "coordinator"})
        bf.request(s, T(2, 5), "r2", usage())
        write_session(self.projects, S1, m, agents=[("aNoMeta", s, None)])
        self.ingest([])
        conn = sqlite3.connect(self.base / "a.sqlite")
        self.assertEqual(conn.execute("SELECT meta_missing, agent_type FROM agents").fetchall(), [(1, None)])
        conn.close()

    def test_coordinator_turn_follows_the_worker_not_the_coordinators_branch(self):
        m = transcript(S1, branch="docs/parked")
        m.user_text(T(1), "go", origin={"kind": "human"})
        bf.request(m, T(1, 5), "r_go", usage())
        m.user_text(T(10), "<task-notification>\n<task-id>aW</task-id>\n<status>completed</status>\n"
                    "</task-notification>", origin={"kind": "task-notification"})
        bf.request(m, T(10, 5), "r_done", usage())
        w = transcript(S1, agent_id="aW", branch="feat/worker")
        w.user_text(T(2), "work", origin={"kind": "coordinator"})
        for i in range(3):
            bf.request(w, T(3 + i), f"r_w{i}", usage())
        write_session(self.projects, S1, m, agents=[("aW", w, {"agentType": "worker", "name": "w"})])
        got = self.ingest([pr(70, "feat/worker", day(1)), pr(71, "docs/parked", day(1))])
        self.assertEqual(got["r_go"], [(71, "branch", 1.0, 1.0)])
        self.assertEqual(got["r_done"], [(70, "trigger", 1.0, 0.9)])

    def test_background_shell_completion_follows_its_description(self):
        m = transcript(S1)
        m.user_text(T(1), "spawn", origin={"kind": "human"})
        bf.request(m, T(1, 5), "r_s", usage(), content=[
            {"type": "tool_use", "id": "tu_a", "name": "Agent", "input": {"prompt": "p"}}])
        m.tool_result(T(1, 6), "tu_a", "ok")["toolUseResult"] = {"agentId": "aZ"}
        bf.request(m, T(1, 7), "r_bg", usage(), content=[
            {"type": "tool_use", "id": "tu_bg", "name": "Bash", "input": {
                "command": "gh run watch", "description": "Watch CI for #88", "run_in_background": True}}])
        m.tool_result(T(1, 8), "tu_bg", "started")["toolUseResult"] = {"backgroundTaskId": "bq1", "stdout": ""}
        m.user_text(T(20), "<task-notification>\n<task-id>bq1</task-id>\n<status>completed</status>\n"
                    "</task-notification>", origin={"kind": "task-notification"})
        bf.request(m, T(20, 5), "r_ci", usage())
        write_session(self.projects, S1, m)
        got = self.ingest([pr(88, "feat/ci", day(1))])
        self.assertEqual(got["r_ci"], [(88, "trigger", 1.0, 0.9)])
        # The background call itself is not a pr-link: its #88 came from a description.
        self.assertEqual(got["r_bg"], [(None, "unattributed", 1.0, 0.0)])

    def test_imbalanced_attribution_fails_the_run(self):
        m = transcript(S1)
        m.user_text(T(1), "x", origin={"kind": "human"})
        bf.request(m, T(1, 5), "r1", usage())
        write_session(self.projects, S1, m)
        half = [(None, "unattributed", 0.5)]
        with mock.patch.object(attribution.Attributor, "final", lambda self, req, depth=0: half):
            status = run_ingest(self.projects, self.base / "bad.sqlite", prs=[])
        self.assertEqual(status, 2)


def git(repo, *args, when=None):
    env = dict(os.environ, GIT_AUTHOR_NAME="t", GIT_AUTHOR_EMAIL="t@example.invalid", GIT_COMMITTER_NAME="t",
               GIT_COMMITTER_EMAIL="t@example.invalid")
    if when:
        env.update(GIT_AUTHOR_DATE=when, GIT_COMMITTER_DATE=when)
    out = subprocess.run(["git", "-C", str(repo), *args], env=env, capture_output=True, text=True, check=True)
    return out.stdout.strip()


class AncestryTest(unittest.TestCase):
    """A3: a packet branch with no PR of its own, squash-merged into main through an integration PR."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        base = Path(self.tmp.name)
        self.projects = base / "projects"
        repo = self.repo = base / "repo"
        repo.mkdir()
        git(repo, "init", "-q", "-b", "main")
        git(repo, "commit", "-q", "--allow-empty", "-m", "base", when="2026-10-01T09:00:00Z")
        git(repo, "checkout", "-q", "-b", "integ")
        git(repo, "checkout", "-q", "-b", "packet")
        git(repo, "commit", "-q", "--allow-empty", "-m", "packet work", when="2026-10-01T13:00:00Z")
        git(repo, "checkout", "-q", "main")
        git(repo, "commit", "-q", "--allow-empty", "-m", "other PR", when="2026-10-01T14:00:00Z")
        git(repo, "branch", "stale")                                   # never committed: head is on main
        git(repo, "checkout", "-q", "integ")
        git(repo, "merge", "-q", "--no-ff", "packet", "-m", "Merge branch 'packet' into integ",
            when="2026-10-02T09:00:00Z")
        git(repo, "merge", "-q", "--no-ff", "main", "-m", "Merge branch 'main' into integ",
            when="2026-10-02T10:00:00Z")
        self.integ_head = git(repo, "rev-parse", "HEAD")
        git(repo, "checkout", "-q", "main")
        git(repo, "commit", "-q", "--allow-empty", "-m", "integ (#90)", when="2026-10-03T09:00:00Z")
        self.squash = git(repo, "rev-parse", "HEAD")
        git(repo, "branch", "-D", "-q", "packet")                       # rm-worktree.sh deletes merged packets

    def tearDown(self):
        self.tmp.cleanup()

    def test_packet_commit_reaches_the_integration_pr(self):
        m = transcript(S3)
        m.user_text(T(0), "x", origin={"kind": "human"})
        bf.request(m, T(1), "r_coord", bf.usage(1, 1, 0, 1, 1, 0))
        s = transcript(S3, agent_id="aP", branch="packet")
        s.user_text(T(2), "x", origin={"kind": "coordinator"})
        bf.request(s, T(3), "r_packet", bf.usage(1, 1, 0, 1, 1, 0))
        st = transcript(S3, agent_id="aS", branch="stale")
        st.user_text(T(2), "x", origin={"kind": "coordinator"})
        bf.request(st, T(4), "r_stale", bf.usage(1, 1, 0, 1, 1, 0))
        write_session(self.projects, S3, m, agents=[("aP", s, None), ("aS", st, None)])
        prs = [pr(90, "integ", "2026-10-01T10:00:00Z", merged="2026-10-03T09:00:00Z", head=self.integ_head,
                  merge=self.squash)]
        db = Path(self.tmp.name) / "a.sqlite"
        self.assertEqual(run_ingest(self.projects, db, prs=prs, repo=self.repo), 0)
        conn = sqlite3.connect(db)
        got = dict(((r, (n, m_)) for r, n, m_ in conn.execute(
            "SELECT request_id, pr_number, method FROM pr_attribution")))
        heads = conn.execute("SELECT source FROM branch_heads WHERE branch = 'packet'").fetchall()
        conn.close()
        # The packet branch is gone; its commit is known from the merge subject.
        self.assertEqual(heads, [("merge-subject",)])
        self.assertEqual(got["r_packet"], (90, "ancestry"))
        # A branch whose head main already had before the merge is not that PR's work.
        self.assertEqual(got["r_stale"], (None, "unattributed"))


if __name__ == "__main__":
    unittest.main()
