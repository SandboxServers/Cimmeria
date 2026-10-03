"""Attribution of in-process teammates and PR authorship: the failure classes TP-05b found in Wave 2.

Each test is shaped on one observed failure, on synthetic sessions:

- a teammate's records carry the coordinator process's branch and cwd, so
  branch matching charged it to the coordinator's PR or left it unattributed;
- a turn that viewed one PR and created another was split between them;
- a teammate's turns before the one that opened its PR stayed unattributed;
- a rebase worker that never touched a PR command stayed unattributed.
"""

import json
import sqlite3
import tempfile
import unittest
from pathlib import Path

from .test_support import build_fixtures as bf
from .test_support import pr, run_ingest, transcript, write_session

S1 = "11111111-0000-4000-8000-000000000001"
T = bf.ts
COORD = "docs/coordinator-ledger"
WORK = "feat/teammate-work"
WT = "C:\\Users\\Steve\\source\\projects\\Cimmeria\\.claude\\worktrees\\tm1"


def usage():
    return bf.usage(1, 10, 0, 100, 10, 0)


def teammate_meta(name):
    return {"agentType": name, "name": name, "taskKind": "in_process_teammate", "requestShape": "background"}


def shell(tid, command):
    return {"type": "tool_use", "id": tid, "name": "Bash", "input": {"command": command}}


class TeammateTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.base = Path(self.tmp.name)
        self.projects = self.base / "projects"
        self.prs = [pr(10, COORD, "2026-09-30T00:00:00Z"), pr(11, WORK, "2026-09-30T00:00:00Z"),
                    pr(12, "feat/other-pr", "2026-09-30T00:00:00Z")]

    def tearDown(self):
        self.tmp.cleanup()

    def coordinator(self):
        m = transcript(S1, branch=COORD)
        m.user_text(T(0), "go", origin={"kind": "human"})
        bf.request(m, T(0, 5), "r_coord", usage())
        return m

    def teammate(self, agent="aT", branch=COORD):
        """A teammate whose records carry the coordinator's branch, as Claude Code writes them."""
        t = transcript(S1, agent_id=agent, branch=branch)
        t.user_text(T(1), "<teammate-message teammate_id=\"team-lead\">work</teammate-message>",
                    origin={"kind": "coordinator"})
        return t

    def ingest(self, agents, *extra):
        write_session(self.projects, S1, self.coordinator(), agents=agents)
        db = self.base / "a.sqlite"
        self.assertEqual(run_ingest(self.projects, db, *extra, prs=self.prs), 0)
        conn = sqlite3.connect(db)
        got = {}
        for rid, n, method, w in conn.execute("SELECT request_id, pr_number, method, weight FROM pr_attribution"):
            got.setdefault(rid, []).append((n, method, round(w, 3)))
        conn.close()
        return {k: sorted(v, key=repr) for k, v in got.items()}

    def test_teammate_git_output_names_its_branch_not_the_coordinators(self):
        t = self.teammate()
        bf.request(t, T(2), "r_t1", usage(), content=[shell("tu_s", f"cd {WT}; git status")])
        t.tool_result(T(2, 5), "tu_s", f"On branch {WORK}\nnothing to commit\n")
        bf.request(t, T(3), "r_t2", usage())
        got = self.ingest([("aT", t, teammate_meta("tm1"))])
        self.assertEqual(got["r_t1"], [(11, "worktree", 1.0)])
        self.assertEqual(got["r_t2"], [(11, "worktree", 1.0)])
        self.assertEqual(got["r_coord"], [(10, "branch", 1.0)])

    def test_teammate_in_a_worktree_the_lane_log_names(self):
        t = self.teammate()
        read = {"type": "tool_use", "id": "tu_r", "name": "Read", "input": {"file_path": WT + "\\src\\a.rs"}}
        bf.request(t, T(2), "r_t1", usage(), content=[read])
        t.tool_result(T(2, 5), "tu_r", "fn main() {}")
        lane = self.base / "jobs.jsonl"
        lane.write_text(json.dumps({"worktree": "tm1", "branch": WORK, "commit": "abcdef1234",
                                    "start": "2026-10-01T07:00:00-0500"}) + "\n", encoding="utf-8")
        got = self.ingest([("aT", t, teammate_meta("tm1"))], "--lane-log", str(lane))
        self.assertEqual(got["r_t1"], [(11, "worktree", 1.0)])

    def test_teammate_with_no_work_evidence_is_not_charged_to_the_coordinators_branch(self):
        t = self.teammate()
        bf.request(t, T(2), "r_t1", usage())
        got = self.ingest([("aT", t, teammate_meta("tm1"))])
        self.assertEqual(got["r_t1"], [(None, "unattributed", 1.0)])

    def test_teammate_record_cwd_of_a_sibling_worktree_is_ignored(self):
        # The process cwd moved to a sibling's worktree: the record says feat/other-pr, the teammate's
        # own git call says its own branch.
        t = self.teammate(branch="feat/other-pr")
        for rec in t.lines:
            rec["cwd"] = WT.replace("tm1", "sibling")
        bf.request(t, T(2), "r_t1", usage(), content=[shell("tu_s", f"git -C {WT} status -sb")])
        t.tool_result(T(2, 5), "tu_s", f"## {WORK}...origin/{WORK}\n")
        for rec in t.lines:
            rec["cwd"] = WT.replace("tm1", "sibling")
        got = self.ingest([("aT", t, teammate_meta("tm1"))])
        self.assertEqual(got["r_t1"], [(11, "worktree", 1.0)])

    def test_rebase_worker_is_placed_by_the_rebase_output(self):
        t = self.teammate()
        bf.request(t, T(2), "r_rb", usage(), content=[shell("tu_rb", f"cd {WT}; git rebase origin/main")])
        t.tool_result(T(2, 5), "tu_rb", f"Successfully rebased and updated refs/heads/{WORK}.\n")
        got = self.ingest([("aT", t, teammate_meta("tm1-rebase"))])
        self.assertEqual(got["r_rb"], [(11, "worktree", 1.0)])

    def test_worktree_subagent_record_branch_of_a_sibling_is_ignored(self):
        # A subagent in its own worktree (not a teammate) whose records carry a parallel agent's branch.
        w = transcript(S1, agent_id="aW", branch="feat/other-pr")
        for rec in w.lines:
            rec["cwd"] = WT
        w.user_text(T(1), "work", origin={"kind": "coordinator"})
        bf.request(w, T(2), "r_w1", usage(), content=[shell("tu_c", "git commit -m x")])
        w.tool_result(T(2, 5), "tu_c", f"[{WORK} 1a2b3c4] x\n 1 file changed\n")
        bf.request(w, T(3), "r_w2", usage())
        for rec in w.lines:
            rec["cwd"] = WT
        got = self.ingest([("aW", w, {"agentType": "rust-gameserver-dev"})])
        self.assertEqual(got["r_w1"], [(11, "worktree", 1.0)])
        self.assertEqual(got["r_w2"], [(11, "worktree", 1.0)])

    def test_a_subagent_record_never_maps_a_worktree_to_a_branch(self):
        # aW's records say worktree tm1 is on feat/other-pr; that is the process checkout's branch,
        # so the teammate that only reads files in tm1 must not be charged to PR 12 through it.
        w = transcript(S1, agent_id="aW", branch="feat/other-pr")
        w.user_text(T(1), "work", origin={"kind": "coordinator"})
        bf.request(w, T(2), "r_w", usage())
        for rec in w.lines:
            rec["cwd"] = WT
        t = self.teammate()
        read = {"type": "tool_use", "id": "tu_r", "name": "Read", "input": {"file_path": WT + "\\src\\a.rs"}}
        bf.request(t, T(3), "r_t", usage(), content=[read])
        t.tool_result(T(3, 5), "tu_r", "fn main() {}")
        got = self.ingest([("aW", w, {"agentType": "rust-gameserver-dev"}), ("aT", t, teammate_meta("tm2"))])
        self.assertEqual(got["r_w"], [(None, "unattributed", 1.0)])
        self.assertEqual(got["r_t"], [(None, "unattributed", 1.0)])

    def test_a_number_in_a_created_prs_title_is_not_the_pr(self):
        t = self.teammate()
        bf.request(t, T(2), "r_c", usage(), content=[
            shell("tu_c", 'gh pr create --title "feat: phase 0, item 12 emitter" --body b')])
        t.tool_result(T(2, 5), "tu_c", "https://github.com/o/r/pull/11\n")
        got = self.ingest([("aT", t, teammate_meta("tm1"))])
        self.assertEqual(got["r_c"], [(11, "pr-link", 1.0)])

    def test_turn_that_views_one_pr_and_creates_another_is_the_created_prs(self):
        t = self.teammate()
        bf.request(t, T(2), "r_view", usage(), content=[shell("tu_v", "gh pr view 12 --json body")])
        t.tool_result(T(2, 5), "tu_v", "{}")
        bf.request(t, T(3), "r_create", usage(), content=[shell("tu_c", "gh pr create --title t --body b")])
        t.tool_result(T(3, 5), "tu_c", "https://github.com/o/r/pull/11\n")
        got = self.ingest([("aT", t, teammate_meta("tm1"))])
        self.assertEqual(got["r_view"], [(11, "pr-link", 1.0)])
        self.assertEqual(got["r_create"], [(11, "pr-link", 1.0)])

    def test_earlier_turns_of_a_teammate_go_to_the_one_pr_it_created(self):
        t = self.teammate()
        bf.request(t, T(2), "r_first_turn", usage())
        t.user_text(T(4), "<task-notification>\n<task-id>bx</task-id>\n<status>completed</status>\n"
                    "</task-notification>", origin={"kind": "task-notification"})
        bf.request(t, T(5), "r_second_turn", usage(), content=[shell("tu_c", "gh pr create --fill")])
        t.tool_result(T(5, 5), "tu_c", "https://github.com/o/r/pull/11\n")
        got = self.ingest([("aT", t, teammate_meta("tm1"))])
        self.assertEqual(got["r_first_turn"], [(11, "pr-link", 1.0)])
        self.assertEqual(got["r_second_turn"], [(11, "pr-link", 1.0)])

    def test_main_session_turn_that_views_one_pr_and_creates_another(self):
        m = transcript(S1)
        m.user_text(T(10), "open the PR", origin={"kind": "human"})
        bf.request(m, T(10, 5), "r_open", usage(), content=[shell("tu_v", "gh pr view 12"),
                                                             shell("tu_c", "gh pr create --fill")])
        m.tool_result(T(10, 6), "tu_v", "title: other")
        m.tool_result(T(10, 7), "tu_c", "https://github.com/o/r/pull/11\n")
        write_session(self.projects, S1, m)
        db = self.base / "c.sqlite"
        self.assertEqual(run_ingest(self.projects, db, prs=self.prs), 0)
        conn = sqlite3.connect(db)
        rows = sorted(conn.execute("SELECT pr_number, method, weight FROM pr_attribution WHERE request_id = 'r_open'"))
        conn.close()
        self.assertEqual(rows, [(11, "pr-link", 1.0)])

    def test_a_reviewer_turn_is_the_reviewed_prs(self):
        t = self.teammate()
        bf.request(t, T(2), "r_review", usage(), content=[shell("tu_d", "gh pr diff 12 | head -400")])
        t.tool_result(T(2, 5), "tu_d", "diff --git a/x b/x")
        got = self.ingest([("aT", t, teammate_meta("reviewer"))])
        self.assertEqual(got["r_review"], [(12, "pr-link", 1.0)])

    def test_a_turn_naming_two_prs_without_creating_either_still_splits(self):
        m = transcript(S1)
        m.user_text(T(10), "check both", origin={"kind": "human"})
        bf.request(m, T(10, 5), "r_both", usage(), content=[shell("tu_a", "gh pr checks 11"),
                                                             shell("tu_b", "gh pr checks 12")])
        write_session(self.projects, S1, m)
        db = self.base / "b.sqlite"
        self.assertEqual(run_ingest(self.projects, db, prs=self.prs), 0)
        conn = sqlite3.connect(db)
        rows = sorted(conn.execute("SELECT pr_number, method, weight FROM pr_attribution WHERE request_id = 'r_both'"))
        conn.close()
        self.assertEqual(rows, [(11, "split", 0.5), (12, "split", 0.5)])


if __name__ == "__main__":
    unittest.main()
