"""Tests for rm-worktree.sh --merged-into, each run under real bash against a temporary
repository (a bare origin, a main clone and three packet worktrees). The gh and psql
stubs are shell shims first on PATH. The same cases and expected lines as the PowerShell
twin, test-rm-worktree.ps1.

    python -m unittest discover -s tools/build-lane -p "test_*.py"
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from test_lane_stats import find_bash  # noqa: E402
from test_rebase_pr import Repo, sh  # noqa: E402

GH_SHIM = (
    'case "$1 $2" in\n'
    '  "pr list") echo null ;;\n'
    '  "pr view") echo "$GH_STUB_STATE" ;;\n'
    '  *) exit 2 ;;\n'
    'esac\n'
)


@unittest.skipUnless(find_bash(), "no bash")
class RmMergedIntoTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="rm-merged-into-"))
        self.repo = Repo(self.tmp)
        main = self.repo.main
        self.repo.commit(main, {".gitignore": ".claude/worktrees/\n", "README.md": "base\n"}, "base")
        self.repo.push_main()
        # a, b and c each commit their own file; a and c are merged into the integration branch.
        for n in "abc":
            sh("git", "worktree", "add", "-q", "-b", f"zz-s5/{n}", f".claude/worktrees/zz-s5-{n}",
               "origin/main", cwd=main)
            self.repo.commit(self.wt(n), {f"{n}.txt": f"{n}\n"}, f"packet {n}")
        sh("git", "checkout", "-q", "-b", "zz-s5/int", "origin/main", cwd=main)
        sh("git", "merge", "-q", "--no-ff", "-m", "integration", "zz-s5/a", "zz-s5/c", cwd=main)
        sh("git", "push", "-q", "origin", "HEAD:refs/pull/7/head", cwd=main)
        sh("git", "checkout", "-q", "main", cwd=main)
        self.bin = self.tmp / "bin"
        self.bin.mkdir()
        self.shim("gh", GH_SHIM)
        self.shim("psql", "exit 0\n")

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def shim(self, name: str, body: str) -> None:
        path = self.bin / name
        path.write_text(f"#!/bin/sh\n{body}", encoding="utf-8", newline="\n")
        path.chmod(0o755)

    def wt(self, n: str) -> Path:
        return self.repo.main / ".claude" / "worktrees" / f"zz-s5-{n}"

    def branches(self, name: str) -> str:
        return sh("git", "branch", "--list", name, cwd=self.repo.main)

    def rm(self, *args: str, state: str = "MERGED"):
        env = dict(os.environ)
        env["PATH"] = self.bin.as_posix() + os.pathsep + os.environ["PATH"]
        env.update(PSQL=(self.bin / "psql").as_posix(), LANE_ROOT=(self.tmp / "lane").as_posix(),
                   CIMMERIA_TARGET_ROOT="", GH_STUB_STATE=state)
        return subprocess.run([find_bash(), (HERE / "rm-worktree.sh").as_posix(), *args],
                              cwd=self.repo.main, env=env, capture_output=True, text=True)

    def test_merged_into_retires_a_packet_merged_into_the_pr_head(self):
        p = self.rm("--merged-into", "7", "zz-s5-a")
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertIn("retire zz-s5-a (zz-s5/a, MERGED_INTO)", p.stdout)
        self.assertFalse(self.wt("a").exists())
        self.assertEqual(self.branches("zz-s5/a"), "")

    def test_without_the_option_the_same_packet_is_skipped(self):
        p = self.rm("zz-s5-c")
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertIn("skip zz-s5-c: branch zz-s5/c is NO_PR", p.stdout)
        self.assertTrue(self.wt("c").exists())

    def test_a_branch_past_the_pr_head_is_skipped(self):
        p = self.rm("--merged-into", "7", "zz-s5-b")
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertIn("skip zz-s5-b: branch zz-s5/b is NO_PR", p.stdout)
        self.assertTrue(self.wt("b").exists())

    def test_uncommitted_changes_still_block_the_retire(self):
        (self.wt("c") / "scratch.txt").write_text("x\n", encoding="utf-8")
        p = self.rm("--merged-into", "7", "zz-s5-c")
        self.assertIn("skip zz-s5-c: uncommitted changes", p.stdout)
        self.assertTrue(self.wt("c").exists())
        (self.wt("c") / "scratch.txt").unlink()
        p = self.rm("--merged-into", "7", "zz-s5-c")
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertIn("retire zz-s5-c (zz-s5/c, MERGED_INTO)", p.stdout)
        self.assertFalse(self.wt("c").exists())

    def test_a_pr_that_is_not_merged_stops_the_run(self):
        p = self.rm("--merged-into", "7", "zz-s5-b", state="OPEN")
        self.assertEqual(p.returncode, 1)
        self.assertIn("not merged", p.stderr)
        self.assertTrue(self.wt("b").exists())

    def test_usage_errors_exit_2(self):
        cases = [
            (("--merged-into",), "--merged-into needs a PR number"),
            (("--merged-into", "x", "zz-s5-b"), "--merged-into needs a PR number, got x"),
            (("--merged-into", "7", "--merged"), "does not combine with --merged"),
            (("--merged-into", "7"), "usage: rm-worktree.sh"),
        ]
        for args, text in cases:
            with self.subTest(args=args):
                p = self.rm(*args)
                self.assertEqual(p.returncode, 2, p.stderr)
                self.assertIn(text, p.stderr)

    def test_dry_run_of_a_skip_removes_nothing(self):
        p = self.rm("--dry-run", "--merged-into", "7", "zz-s5-b")
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertIn("dry run: would retire 0, skip 1", p.stdout)
        self.assertTrue(self.wt("b").exists())


if __name__ == "__main__":
    unittest.main()
