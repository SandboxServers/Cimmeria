"""The work-branch extractors and campaign tagging, on synthetic text and PR rows."""

import sqlite3
import unittest
from pathlib import Path

from . import campaigns as cmod
from . import workbranch as wb

HERE = Path(__file__).resolve().parent


class ExtractorTest(unittest.TestCase):
    def test_branch_from_each_git_output_shape(self):
        cases = {
            "On branch feat/a\nnothing to commit\n": "feat/a",
            "## feat/b...origin/feat/b [behind 1]\n M x\n": "feat/b",
            "## feat/c\n": "feat/c",
            "## feat/c-2...origin/feat/c-2\n": "feat/c-2",
            "[feat/d 1a2b3c4] fix: thing\n 1 file changed\n": "feat/d",
            "Switched to a new branch 'feat/e'\n": "feat/e",
            "Successfully rebased and updated refs/heads/feat/f.\n": "feat/f",
            "To github.com:o/r.git\n + 1234567...89abcde feat/g -> feat/g (forced update)\n": "feat/g",
            "To github.com:o/r.git\n * [new branch]      feat/h -> feat/h\n": "feat/h",
            "On branch main\n": "main",
        }
        for text, want in cases.items():
            self.assertEqual(wb.branch_seen(text), want, text)

    def test_ambiguous_or_heading_output_names_no_branch(self):
        self.assertIsNone(wb.branch_seen("On branch feat/a\n[feat/b 1234567] x\n"))
        self.assertIsNone(wb.branch_seen("## Purpose\n## Decisions\n"))
        self.assertIsNone(wb.branch_seen("HEAD detached at 1234567\n"))
        # git fetch prints ref updates too, under `From`; they name other people's branches.
        self.assertIsNone(wb.branch_seen("From github.com:o/r\n   1234567..89abcde  main  -> origin/main\n"))
        self.assertEqual(wb.branch_seen("From github.com:o/r\n   1234567..89abcde  main  -> origin/main\n"
                                        "To github.com:o/r\n   89abcde..1234567  feat/x -> feat/x\n"), "feat/x")

    def test_worktree_reference_in_paths_and_commands(self):
        self.assertEqual(wb.worktree_ref({"command": "cd C:\\r\\.claude\\worktrees\\tp10; git status"}),
                         ".claude/worktrees/tp10")
        self.assertEqual(wb.worktree_ref({"file_path": ".claude/worktrees/tp10/a.py"}), ".claude/worktrees/tp10")
        self.assertIsNone(wb.worktree_ref({"command": "diff .claude/worktrees/a/x .claude/worktrees/b/x"}))
        self.assertIsNone(wb.worktree_ref({"command": "git status"}))

    def test_worktree_list_pairs(self):
        plain = "C:/r  c402ad3ca [main]\nC:/r/.claude/worktrees/tp05b  7a1bdde66 [feat/t]\n"
        porcelain = "worktree C:/r/.claude/worktrees/x\nHEAD 7a1bdde66aaaa\nbranch refs/heads/feat/x\n\n"
        self.assertEqual(wb.worktree_list(plain), [(".claude/worktrees/tp05b", "feat/t")])
        self.assertEqual(wb.worktree_list(porcelain), [(".claude/worktrees/x", "feat/x")])


class CampaignTest(unittest.TestCase):
    RULES = {"_doc": "x", "harset": {"branches": ["content/harset-rebuild"], "prefixes": ["harset/"]},
             "tokens": {"issues": [957]}, "craft": {"prefixes": ["craft/", "craft/cr1"]}}

    def tag(self, rows):
        db = sqlite3.connect(":memory:")
        db.execute("CREATE TABLE prs (pr_number INTEGER, head_branch TEXT, title TEXT, base_branch TEXT,"
                   " campaign TEXT)")
        db.executemany("INSERT INTO prs (pr_number, head_branch, title, base_branch) VALUES (?, ?, ?, ?)", rows)
        cmod.tag_prs(db, cmod.Campaigns(self.RULES))
        return dict(db.execute("SELECT pr_number, campaign FROM prs"))

    def test_branch_title_and_base_rules(self):
        got = self.tag([
            (1, "content/harset-rebuild", "Harset", "main"),
            (2, "harset/h07", "packet", "content/harset-rebuild"),
            (3, "feat/x", "feat(token-profile): TP-10 (#957)", "main"),
            (4, "feat/integ", "integration", "main"),
            (5, "feat/packet", "packet", "feat/integ"),
            (6, "fix/y", "fix: unrelated (#12)", "main"),
            (7, "craft/cr13-close", "close", "main"),
        ])
        self.assertEqual(got, {1: "harset", 2: "harset", 3: "tokens", 4: "pr-4", 5: "pr-4", 6: None, 7: "craft"})

    def test_every_configured_campaign_has_a_ledger_folder(self):
        rules = cmod.Campaigns.load()
        analysis = HERE.parent.parent.parent / "docs" / "analysis"
        for name in rules.names:
            self.assertTrue((analysis / name).is_dir(), name)


if __name__ == "__main__":
    unittest.main()
