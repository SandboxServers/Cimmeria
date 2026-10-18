"""Hard-case integration tests for commit-paths.sh, the bash twin of test-commit-paths.ps1 (its sections from the whitespace tests on).

Each test builds a fresh throwaway main checkout and one linked worktree, with no network, and runs the
script through Git Bash (or PATH bash elsewhere). Methods do not depend on each other. Each Check of the
.ps1 section is one assertion whose message is the Check's text.
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
from test_lane_stats import find_bash  # noqa: E402  Git Bash on Windows, PATH bash elsewhere

SCRIPT = HERE / "commit-paths.sh"
BASH = find_bash()


def write(path: Path, text: str) -> None:
    """Writes text with LF endings, whatever the platform."""
    with open(path, "w", encoding="utf-8", newline="") as f:
        f.write(text)


def make_junction(link: Path, target: Path) -> None:
    """Makes a junction on Windows and a symlink elsewhere, as the test-commit-paths.ps1 fixture does."""
    if os.name == "nt":
        subprocess.run(["cmd", "/c", "mklink", "/J", str(link), str(target)], check=True, capture_output=True)
    else:
        os.symlink(target, link)


def remove_junction(link: Path) -> None:
    """Removes the link alone, so rmtree never follows it into the target."""
    if os.name == "nt":
        os.rmdir(link)
    else:
        os.unlink(link)


@unittest.skipUnless(BASH, "no bash: Git for Windows missing or LANE_TESTS_NO_BASH=1")
class CommitPathsShMoreTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="commit-paths-sh-more-"))
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

    def git(self, cwd: Path, *args: str, env: dict[str, str] | None = None) -> str:
        r = subprocess.run(
            ["git", "-C", str(cwd), *args],
            env=env or self.env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            errors="replace",
            check=True,
        )
        return r.stdout

    def git_quiet(self, cwd: Path, *args: str) -> int:
        """Runs git and returns its exit code, for steps the fixture expects to fail (a conflict)."""
        return subprocess.run(
            ["git", "-C", str(cwd), *args], env=self.env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
        ).returncode

    def head(self) -> str:
        return self.git(self.wt, "rev-parse", "HEAD").strip()

    def staged(self) -> list[str]:
        return [line for line in self.git(self.wt, "diff", "--cached", "--name-only").splitlines() if line]

    def shown(self) -> list[str]:
        return [x for x in self.git(self.wt, "show", "--name-status", "--format=", "HEAD").splitlines() if x]

    def git_dir(self) -> Path:
        return Path(self.git(self.wt, "rev-parse", "--path-format=absolute", "--absolute-git-dir").strip())

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
        self, wt: Path, paths: list[str], expect: str = "", env: dict[str, str] | None = None
    ) -> tuple[int, str]:
        """Runs the script the way ops does: one comma list for -Paths."""
        argv = ["-C", wt.as_posix(), "-Message", "subject line", "-Paths:" + ",".join(paths)]
        if expect:
            argv += ["-ExpectBranch", expect]
        return self.run_sh(*argv, env=env)

    def test_whitespace_error_stops_the_commit_and_stages_nothing(self) -> None:
        write(self.wt / "a.txt", "bad   \n")
        before = self.head()
        code, out = self.commit(self.wt, ["a.txt"])
        self.assertTrue(code == 2 and "diff-check" in out, f"refuses a whitespace error (exit {code}): {out}")
        self.assertEqual(self.head(), before, "the whitespace refusal did not commit")
        self.assertEqual(self.staged(), [], "the whitespace refusal left nothing staged")

    def test_failing_hook_leaves_nothing_staged(self) -> None:
        write(self.wt / "a.txt", "fine\n")
        before = self.head()
        common = self.git(self.wt, "rev-parse", "--path-format=absolute", "--git-common-dir").strip()
        hook_dir = Path(common) / "hooks"
        hook_dir.mkdir(exist_ok=True)
        hook = hook_dir / "pre-commit"
        write(hook, "#!/bin/sh\nexit 1\n")
        os.chmod(hook, 0o755)
        try:
            code, out = self.commit(self.wt, ["a.txt"])
        finally:
            hook.unlink()
        self.assertTrue(
            code == 1 and "status=failed reason=git-commit" in out,
            f"a failing hook is a failed commit (exit {code}): {out}",
        )
        self.assertEqual(self.head(), before, "the hook failure did not commit")
        self.assertEqual(self.staged(), [], "the hook failure left nothing staged")

    def test_rename_staged_by_someone_else_is_not_swept_in(self) -> None:
        write(self.wt / "r.txt", "rename me, a file long enough to pair\n")
        self.git(self.wt, "add", "--", "r.txt")
        self.git(self.wt, "commit", "-q", "-m", "add r")
        before = self.head()
        self.git(self.wt, "mv", "r.txt", "r2.txt")
        code, out = self.commit(self.wt, ["r2.txt"])
        self.assertTrue(
            code == 2 and "other-staged-files: r.txt" in out,
            f"refuses a staged rename whose source is unnamed (exit {code}): {out}",
        )
        self.assertEqual(self.head(), before, "the rename refusal did not commit")

    def test_deleted_file_can_be_committed_by_name(self) -> None:
        (self.wt / "a.txt").unlink()
        code, out = self.commit(self.wt, ["a.txt"])
        self.assertTrue(code == 0 and "status=ok" in out, f"commits the deletion of a named file (exit {code}): {out}")
        self.assertEqual(self.shown(), ["D\ta.txt"], "the commit is the deletion of a.txt")

    def test_deleted_directory_must_not_stage_every_deletion_beneath_it(self) -> None:
        (self.wt / "d").mkdir()
        write(self.wt / "d" / "x.txt", "x\n")
        write(self.wt / "d" / "y.txt", "y\n")
        self.git(self.wt, "add", "--", "d")
        self.git(self.wt, "commit", "-q", "-m", "add d")
        before = self.head()
        (self.wt / "d" / "x.txt").unlink()
        (self.wt / "d" / "y.txt").unlink()
        (self.wt / "d").rmdir()
        code, out = self.commit(self.wt, ["d"])
        self.assertTrue(code == 2 and "widened-pathspec" in out, f"refuses a deleted directory name (exit {code}): {out}")
        self.assertEqual(self.head(), before, "the deleted-directory refusal did not commit")
        self.assertEqual(self.staged(), [], "the deleted-directory refusal left nothing staged")
        code, out = self.commit(self.wt, ["d/x.txt"])
        self.assertTrue(
            code == 0 and self.shown() == ["D\td/x.txt"],
            f"a deleted file inside the directory can still be named (exit {code}): {out}",
        )

    def test_junction_inside_worktree_must_not_carry_a_file_from_outside(self) -> None:
        outside = self.tmp / "outside"
        outside.mkdir()
        write(outside / "secret.txt", "outside the worktree\n")
        junction = self.wt / "jd"
        make_junction(junction, outside)
        try:
            before = self.head()
            code, out = self.commit(self.wt, ["jd/secret.txt"])
        finally:
            remove_junction(junction)
        self.assertTrue(code == 2 and "reparse-point" in out, f"refuses a path below a junction (exit {code}): {out}")
        self.assertEqual(self.head(), before, "the junction refusal did not commit")
        self.assertEqual(self.staged(), [], "the junction refusal left nothing staged")

    def test_git_dir_and_git_work_tree_in_the_environment_cannot_retarget_the_commit(self) -> None:
        write(self.wt / "a.txt", "env change\n")
        main_tip = self.git(self.main, "rev-parse", "refs/heads/main").strip()
        env = dict(self.env, GIT_DIR=str(self.main / ".git"), GIT_WORK_TREE=str(self.wt))
        code, out = self.commit(self.wt, ["a.txt"], env=env)
        self.assertEqual(
            self.git(self.main, "rev-parse", "refs/heads/main").strip(),
            main_tip,
            "a GIT_DIR pointing at the main checkout did not move main",
        )
        self.assertTrue(
            code == 0 and "branch=feat/x" in out,
            f"the commit still lands on the worktree's branch (exit {code}): {out}",
        )

    def test_branch_compare_is_case_sensitive(self) -> None:
        write(self.wt / "a.txt", "case change\n")
        before = self.head()
        code, out = self.commit(self.wt, ["a.txt"], expect="FEAT/X")
        self.assertTrue(code == 2 and "branch-mismatch" in out, f"-ExpectBranch FEAT/X does not match feat/x (exit {code})")
        self.assertEqual(self.head(), before, "the case mismatch did not commit")

    def test_whitespace_is_refused_only_where_the_repo_sets_a_policy(self) -> None:
        self.git(self.main, "config", "--unset", "core.whitespace")
        # A whitespace attribute is a policy too: that file stays a refusal.
        write(self.wt / ".gitattributes", "*.ws whitespace=trailing-space\n")
        write(self.wt / "w.ws", "attr   \n")
        before = self.head()
        code, out = self.commit(self.wt, ["w.ws"])
        (self.wt / ".gitattributes").unlink()
        self.assertTrue(
            code == 2 and "diff-check" in out,
            f"a whitespace attribute keeps the whitespace refusal (exit {code}): {out}",
        )
        self.assertTrue(
            self.head() == before and self.staged() == [],
            "the attribute refusal committed and staged nothing",
        )
        (self.wt / "w.ws").unlink()
        write(self.wt / "a.txt", "trailing   \n")
        code, out = self.commit(self.wt, ["a.txt"])
        self.assertTrue(
            code == 0 and "status=ok" in out and "warning" in out,
            f"without core.whitespace a whitespace error is a warning, not a refusal (exit {code}): {out}",
        )

    def test_git_object_directory_in_the_environment_does_not_strand_the_commit(self) -> None:
        write(self.wt / "a.txt", "object dir\n")
        odd = self.tmp / "odd-objects"
        odd.mkdir()
        env = dict(self.env, GIT_OBJECT_DIRECTORY=str(odd))
        code, out = self.commit(self.wt, ["a.txt"], env=env)
        tip_ok = self.git_quiet(self.wt, "rev-parse", "--verify", "--quiet", "HEAD^{commit}") == 0
        self.assertTrue(
            code == 0 and tip_ok,
            f"a GIT_OBJECT_DIRECTORY in the environment does not strand the commit (exit {code}): {out}",
        )

    def test_merge_in_progress_is_not_concluded_by_a_path_commit(self) -> None:
        # `merge --no-commit` leaves MERGE_HEAD with an index equal to HEAD.
        side = self.git(self.wt, "commit-tree", "HEAD^{tree}", "-p", "HEAD", "-m", "side").strip()
        self.git_quiet(self.wt, "merge", "-q", "--no-commit", "--no-ff", "-s", "ours", side)
        self.assertEqual(self.git_quiet(self.wt, "rev-parse", "-q", "--verify", "MERGE_HEAD"), 0, "setup: merge open")
        write(self.wt / "a.txt", "edited during a merge\n")
        before = self.head()
        code, out = self.commit(self.wt, ["a.txt"])
        merge_open = self.git_quiet(self.wt, "rev-parse", "-q", "--verify", "MERGE_HEAD") == 0
        self.git_quiet(self.wt, "merge", "--abort")
        self.assertTrue(code == 2 and "merge-in-progress" in out, f"refuses while a merge is in progress (exit {code}): {out}")
        self.assertTrue(
            self.head() == before and merge_open,
            "the merge-in-progress refusal committed nothing and left the merge open",
        )

    def test_conflicted_cherry_pick_or_revert_resolved_to_head_is_still_refused(self) -> None:
        # Resolving to HEAD leaves an index equal to HEAD, so only the *_HEAD pseudo-ref shows the operation.
        write(self.wt / "a.txt", "pick side\n")
        self.git(self.wt, "commit", "-q", "-am", "to pick")
        pick = self.head()
        self.git(self.wt, "reset", "-q", "--hard", "HEAD~1")
        write(self.wt / "a.txt", "head side\n")
        self.git(self.wt, "commit", "-q", "-am", "head side")
        self.git_quiet(self.wt, "cherry-pick", pick)
        self.git(self.wt, "checkout", "-q", "--ours", "--", "a.txt")
        self.git(self.wt, "add", "--", "a.txt")
        git_dir = self.git_dir()
        self.assertTrue((git_dir / "CHERRY_PICK_HEAD").exists(), "setup: the cherry-pick is open with an index equal to HEAD")
        write(self.wt / "a.txt", "edited during a cherry-pick\n")
        before = self.head()
        code, out = self.commit(self.wt, ["a.txt"])
        self.assertTrue(
            code == 2 and "merge-in-progress: CHERRY_PICK_HEAD" in out,
            f"refuses while a cherry-pick is in progress (exit {code}): {out}",
        )
        self.assertTrue(
            self.head() == before and (git_dir / "CHERRY_PICK_HEAD").exists(),
            "the cherry-pick refusal committed nothing and left the cherry-pick open",
        )
        self.git(self.wt, "checkout", "-q", "--", "a.txt")
        self.git_quiet(self.wt, "cherry-pick", "--abort")

        write(self.wt / "a.txt", "revert one\n")
        self.git(self.wt, "commit", "-q", "-am", "to revert")
        to_revert = self.head()
        write(self.wt / "a.txt", "revert two\n")
        self.git(self.wt, "commit", "-q", "-am", "after revert target")
        self.git_quiet(self.wt, "revert", "--no-edit", to_revert)
        self.git(self.wt, "checkout", "-q", "--ours", "--", "a.txt")
        self.git(self.wt, "add", "--", "a.txt")
        self.assertTrue((git_dir / "REVERT_HEAD").exists(), "setup: the revert is open with an index equal to HEAD")
        write(self.wt / "a.txt", "edited during a revert\n")
        before = self.head()
        code, out = self.commit(self.wt, ["a.txt"])
        self.assertTrue(
            code == 2 and "merge-in-progress: REVERT_HEAD" in out,
            f"refuses while a revert is in progress (exit {code}): {out}",
        )
        self.assertTrue(
            self.head() == before and (git_dir / "REVERT_HEAD").exists(),
            "the revert refusal committed nothing and left the revert open",
        )
        self.git(self.wt, "checkout", "-q", "--", "a.txt")
        self.git_quiet(self.wt, "revert", "--abort")


if __name__ == "__main__":
    unittest.main()
