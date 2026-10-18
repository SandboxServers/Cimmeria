"""Integration tests for commit-paths.sh, the bash twin of test-commit-paths.ps1 (its sections up to the trailer tests).

Each test builds a throwaway main checkout and one linked worktree, with no network, and runs the
script through Git Bash (or PATH bash elsewhere).
"""

from __future__ import annotations

import os
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from test_lane_stats import find_bash  # noqa: E402  Git Bash on Windows, PATH bash elsewhere

SCRIPT = HERE / "commit-paths.sh"
BASH = find_bash()


def write(path: Path, text: str) -> None:
    """Writes text with LF endings, whatever the platform."""
    with open(path, "w", encoding="utf-8", newline="") as f:
        f.write(text)


@unittest.skipUnless(BASH, "no bash: Git for Windows missing or LANE_TESTS_NO_BASH=1")
class CommitPathsShTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="commit-paths-sh-"))
        # Isolate git from the developer's global and system config.
        empty = self.tmp / "empty.gitconfig"
        empty.write_text("", encoding="utf-8")
        self.env = dict(os.environ, GIT_CONFIG_GLOBAL=str(empty), GIT_CONFIG_SYSTEM=str(empty))
        self.main = self.tmp / "main"
        self.wt = self.tmp / "wt"
        self.main.mkdir()
        self.git(self.main, "init", "-q", "-b", "main")
        self.git(self.main, "config", "user.email", "test@example.invalid")
        self.git(self.main, "config", "user.name", "test")
        self.git(self.main, "config", "core.whitespace", "cr-at-eol")
        write(self.main / "a.txt", "one\n")
        write(self.main / "b.txt", "one\n")
        self.git(self.main, "add", "--", "a.txt", "b.txt")
        self.git(self.main, "commit", "-q", "-m", "init")
        self.git(self.main, "worktree", "add", "-q", "-b", "feat/x", str(self.wt))
        (self.wt / "sub").mkdir()
        write(self.wt / "sub" / "c.txt", "x\n")

    def tearDown(self) -> None:
        shutil.rmtree(self.tmp, ignore_errors=True)

    def git(self, cwd: Path, *args: str) -> str:
        r = subprocess.run(
            ["git", "-C", str(cwd), *args],
            env=self.env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            errors="replace",
            check=True,
        )
        return r.stdout

    def head(self) -> str:
        return self.git(self.wt, "rev-parse", "HEAD").strip()

    def staged(self) -> list[str]:
        return [line for line in self.git(self.wt, "diff", "--cached", "--name-only").splitlines() if line]

    def run_sh(self, *argv: str, env: dict[str, str] | None = None) -> tuple[int, str]:
        r = subprocess.run(
            [BASH, SCRIPT.as_posix(), *argv],
            env=env or self.env,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            encoding="utf-8",
            errors="replace",
        )
        return r.returncode, r.stdout

    def commit(
        self, wt: Path, paths: list[str], trailers: list[str] | None = None, expect: str = ""
    ) -> tuple[int, str]:
        """Runs the script the way ops does: one comma list for -Paths, one ';;' list for -Trailers."""
        argv = ["-C", wt.as_posix(), "-Message", "subject line", "-Paths:" + ",".join(paths)]
        if trailers:
            argv += ["-Trailers", ";;".join(trailers)]
        if expect:
            argv += ["-ExpectBranch", expect]
        return self.run_sh(*argv)

    def test_refusals_leave_head_where_it_was(self) -> None:
        before = self.head()
        write(self.wt / "a.txt", "two\n")
        for bad in [".", "-A", "--all", "*.txt", "sub", ":/a.txt", "a.txt/", "sub/..", "a.txt,"]:
            code, out = self.commit(self.wt, [bad])
            self.assertTrue(code == 2 and "status=refused" in out, f"refused path '{bad}' (exit {code})")

        # A rooted path must be refused, not joined under -C.
        rooted = [
            str(self.wt),
            str(self.wt / "sub"),
            str(self.wt / "a.txt"),
            self.wt.as_posix(),
            "/a.txt",
            "//server/share/a.txt",
            "C:a.txt",
        ]
        for abs_path in rooted:
            code, out = self.commit(self.wt, [abs_path])
            self.assertTrue(
                code == 2 and "rooted-path" in out,
                f"refused rooted path '{abs_path}' (exit {code}): {out.strip()}",
            )
        self.assertEqual(self.head(), before, "no refusal moved HEAD")
        self.assertEqual(self.staged(), [], "no refusal staged anything")

        # A space-separated list must not commit only its first path.
        code, out = self.run_sh("-C", self.wt.as_posix(), "-Message", "subject line", "-Paths", "a.txt", "sub/c.txt")
        self.assertTrue(
            code != 0 and "status=ok" not in out,
            f"-Paths a b (space separated) is an error, not a partial commit (exit {code})",
        )
        self.assertEqual(self.head(), before, "the space-separated list did not commit")
        self.assertEqual(self.staged(), [], "the space-separated list staged nothing")

        code, out = self.commit(self.main, ["a.txt"])
        self.assertTrue(code == 2 and "main-checkout" in out, "refuses the main checkout")
        code, out = self.commit(self.wt / "sub", ["c.txt"])
        self.assertTrue(code == 2 and "not-the-worktree-root" in out, "refuses a subdirectory of the worktree")

        write(self.wt / "b.txt", "two\n")
        self.git(self.wt, "add", "--", "b.txt")
        code, out = self.commit(self.wt, ["a.txt"])
        self.assertTrue(code == 2 and "other-staged-files: b.txt" in out, "refuses when another file is staged")
        self.git(self.wt, "restore", "--staged", "--", "b.txt")
        self.assertEqual(self.head(), before, "the other-staged refusal did not commit")

        # A named path that is already staged is refused too, and stays staged.
        write(self.wt / "a.txt", "bad   \n")
        self.git(self.wt, "add", "--", "a.txt")
        code, out = self.commit(self.wt, ["a.txt"])
        self.assertTrue(
            code == 2 and "other-staged-files: a.txt" in out,
            f"refuses a named path that is already staged (exit {code}): {out.strip()}",
        )
        self.assertEqual(
            ",".join(self.staged()), "a.txt", "the earlier staging of the named path is left as found"
        )
        self.git(self.wt, "restore", "--staged", "--", "a.txt")
        write(self.wt / "a.txt", "two\n")

        self.git(self.wt, "checkout", "-q", "--detach")
        code, out = self.commit(self.wt, ["a.txt"])
        self.assertTrue(code == 2 and "detached-head" in out, "refuses a detached HEAD")
        self.git(self.wt, "checkout", "-q", "feat/x")

    def test_happy_path_stages_and_commits_only_the_named_files(self) -> None:
        write(self.wt / "a.txt", "two\n")
        write(self.wt / "b.txt", "two\n")
        code, out = self.commit(self.wt, ["a.txt"], ["Co-Authored-By: Test <test@example.invalid>"])
        self.assertTrue(
            code == 0 and re.search(r"status=ok sha=[0-9a-f]{40} branch=feat/x", out),
            f"commits the named path (exit {code}): {out}",
        )
        files = [f for f in self.git(self.wt, "show", "--name-only", "--format=", "HEAD").splitlines() if f]
        self.assertTrue(files == ["a.txt"], "the commit holds only a.txt")
        status = self.git(self.wt, "status", "--porcelain")
        self.assertTrue(" M b.txt" in status, "the unnamed b.txt is left unstaged and modified")
        msg = self.git(self.wt, "log", "-1", "--format=%B")
        self.assertTrue(
            "subject line" in msg and "\n\nCo-Authored-By: Test" in msg,
            "the message has the subject and a trailer block",
        )

    def test_several_paths_and_trailers_survive(self) -> None:
        write(self.wt / "b.txt", "two\n")
        before = self.head()
        code, out = self.commit(self.wt, ["b.txt"], [], "other/branch")
        self.assertTrue(code == 2 and "branch-mismatch" in out, "-ExpectBranch that differs from HEAD is refused")
        self.assertEqual(self.head(), before, "the branch-mismatch refusal did not commit")

        trailers = ["Co-Authored-By: Test <test@example.invalid>", "Claude-Session: https://example.invalid/s/1"]
        code, out = self.commit(self.wt, ["b.txt", "sub/c.txt"], trailers, "feat/x")
        self.assertTrue(
            code == 0 and re.search(r"status=ok sha=[0-9a-f]{40} branch=feat/x", out),
            f"commits two paths with -ExpectBranch (exit {code}): {out}",
        )
        files = sorted(f for f in self.git(self.wt, "show", "--name-only", "--format=", "HEAD").splitlines() if f)
        self.assertTrue(",".join(files) == "b.txt,sub/c.txt", f"the commit holds both named files: {','.join(files)}")
        msg = self.git(self.wt, "log", "-1", "--format=%B")
        self.assertTrue(
            "\n\nCo-Authored-By: Test <test@example.invalid>\nClaude-Session: https://example.invalid/s/1" in msg,
            "both trailers are separate lines in one trailer block",
        )

    def test_whitespace_only_message_is_refused(self) -> None:
        # Not in the .ps1 sections above: it proves the empty-message guard on its own.
        before = self.head()
        write(self.wt / "a.txt", "two\n")
        code, out = self.run_sh("-C", self.wt.as_posix(), "-Message", "   ", "-Paths:a.txt")
        self.assertTrue(code == 2 and "status=refused reason=empty-message" in out, f"refuses a blank message (exit {code})")
        self.assertEqual(self.head(), before, "the empty-message refusal did not commit")
        self.assertEqual(self.staged(), [], "the empty-message refusal staged nothing")


if __name__ == "__main__":
    unittest.main()
