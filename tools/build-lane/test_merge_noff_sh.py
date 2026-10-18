"""Integration tests for merge-noff.sh, the bash twin of merge-noff.ps1, in throwaway git repositories.

Run from tools/build-lane: python -m unittest test_merge_noff_sh -v (needs Git Bash on Windows).
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

SCRIPT = HERE / "merge-noff.sh"
BASH = find_bash()
SHA = re.compile(r"^[0-9a-f]{40}$")


def write(path: Path, text: str) -> None:
    """Write one line of text with a \\n ending, without newline translation."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text + "\n", encoding="utf-8", newline="")


@unittest.skipUnless(BASH, "no bash: Git for Windows missing or LANE_TESTS_NO_BASH=1")
class MergeNoffShTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="merge-noff-sh-"))
        empty = self.tmp / "empty.gitconfig"
        empty.write_text("", encoding="utf-8")
        self.env = dict(os.environ, GIT_CONFIG_GLOBAL=str(empty), GIT_CONFIG_SYSTEM=str(empty))
        self.side = 0
        self.main = self.tmp / "main"
        self.wt = self.tmp / "wt"
        self.src = self.tmp / "src"
        # the same fixture as test-merge-noff.ps1's setup: main, feat/target (wt), feat/src (src)
        self.main.mkdir()
        self.git(self.main, "init", "-q", "-b", "main")
        self.git(self.main, "config", "user.email", "test@example.invalid")
        self.git(self.main, "config", "user.name", "test")
        write(self.main / "a.txt", "base")
        write(self.main / "b.txt", "base")
        self.git(self.main, "add", "--", "a.txt", "b.txt")
        self.git(self.main, "commit", "-q", "-m", "init")
        self.init_sha = self.git(self.main, "rev-parse", "HEAD")
        self.git(self.main, "worktree", "add", "-q", "-b", "feat/target", self.wt)
        self.git(self.main, "branch", "feat/src")
        self.git(self.main, "branch", "feat/old")
        self.git(self.main, "branch", "feat/conf")
        self.git(self.main, "worktree", "add", "-q", self.src, "feat/src")
        self.commit_file(self.src, "b.txt", "from src")
        self.commit_file(self.wt, "a.txt", "from target")

    def tearDown(self) -> None:
        shutil.rmtree(self.tmp, ignore_errors=True)

    def git(self, cwd: Path, *args: str | Path, check: bool = True) -> str:
        argv = [a.as_posix() if isinstance(a, Path) else a for a in args]
        r = subprocess.run(["git", "-C", cwd.as_posix(), *argv], env=self.env, stdout=subprocess.PIPE,
                           stderr=subprocess.STDOUT, text=True, encoding="utf-8", errors="replace", check=check)
        return r.stdout.strip()

    def run_sh(self, *argv: str, env: dict[str, str] | None = None) -> tuple[int, str]:
        r = subprocess.run([BASH, SCRIPT.as_posix(), *argv], env=env or self.env,
                           stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                           text=True, encoding="utf-8", errors="replace")
        return r.returncode, r.stdout

    def merge(self, wt: Path, branch: str, expect: str = "", env: dict[str, str] | None = None) -> tuple[int, str]:
        # -Branch:<value> keeps a value that starts with `-` (such as --no-ff) from being a usage error
        argv = ["-C", wt.as_posix(), f"-Branch:{branch}"]
        if expect:
            argv.append(f"-ExpectBranch:{expect}")
        return self.run_sh(*argv, env=env)

    def commit_file(self, cwd: Path, name: str, text: str) -> str:
        write(cwd / name, text)
        self.git(cwd, "add", "--", name)
        self.git(cwd, "commit", "-q", "-m", f"{name} {text}")
        return self.git(cwd, "rev-parse", "HEAD")

    def new_side(self, branch: str, base: str, name: str, text: str) -> Path:
        self.side += 1
        d = self.tmp / f"side{self.side}"
        self.git(self.main, "worktree", "add", "-q", "-b", branch, d, base)
        self.commit_file(d, name, text)
        return d

    def head(self, cwd: Path) -> str:
        return self.git(cwd, "rev-parse", "HEAD")

    def parents(self, cwd: Path) -> list[str]:
        return self.git(cwd, "rev-list", "--parents", "-n", "1", "HEAD").split()

    def make_ff(self) -> str:
        ff = self.tmp / "ff"
        self.git(self.main, "worktree", "add", "-q", "-b", "feat/ff", ff, self.head(self.wt))
        return self.commit_file(ff, "ff.txt", "ff")

    # --- refusals -----------------------------------------------------------------------

    def test_refusals(self) -> None:
        before = self.head(self.wt)
        for bad in ("--no-ff", "-x", "no-such", "feat/target"):
            code, out = self.merge(self.wt, bad)
            self.assertTrue(code == 2 and "status=refused" in out, f"refused branch '{bad}' (exit {code})")
        code, out = self.merge(self.main, "feat/src")
        self.assertTrue(code == 2 and "main-checkout" in out, "refuses the main checkout")
        write(self.wt / "junk.txt", "untracked")
        code, out = self.merge(self.wt, "feat/src")
        self.assertTrue(code == 2 and "dirty-tree" in out, "refuses a dirty tree, untracked files included")
        (self.wt / "junk.txt").unlink()
        code, out = self.merge(self.wt, "feat/src", "some/other-branch")
        self.assertTrue(code == 2 and "branch-mismatch" in out, "-ExpectBranch that differs from HEAD is refused")
        code, out = self.merge(self.wt, "feat/src", "FEAT/TARGET")
        self.assertTrue(code == 2 and "branch-mismatch" in out, "-ExpectBranch is compared case-sensitively")
        # a bare second token must be a parameter error: as a positional it would become -Branch
        code, out = self.run_sh("-C", self.wt.as_posix(), "feat/src")
        self.assertTrue(code != 0 and "status=ok" not in out,
                        f"a positional branch token is a parameter error, not a merge (exit {code})")
        self.assertEqual(self.head(self.wt), before, "the refusals moved nothing")
        # extra guard proof: a tag with no branch of that name is no-such-local-branch
        self.git(self.main, "tag", "tagonly", self.init_sha)
        code, out = self.merge(self.wt, "tagonly")
        self.assertTrue(code == 2 and "no-such-local-branch: tagonly" in out,
                        f"a tag with no branch of that name is refused (exit {code})")

    # --- clean --no-ff merge -----------------------------------------------------------

    def test_clean_no_ff_merge(self) -> None:
        code, out = self.merge(self.wt, "feat/src", "feat/target")
        parents = self.parents(self.wt)
        self.assertTrue(code == 0 and re.search(r"status=ok merge=[0-9a-f]{40} branch=feat/src", out),
                        f"merges with --no-ff (exit {code}): {out.strip()}")
        self.assertEqual(len(parents), 3, "the merge commit has two parents")
        self.assertIn(f"merge={self.head(self.wt)}", out, "the printed merge SHA is HEAD")

    # --- already contained: no new commit ----------------------------------------------

    def test_already_contained(self) -> None:
        before = self.head(self.wt)
        code, out = self.merge(self.wt, "feat/old")
        self.assertTrue(code == 0 and "status=up-to-date" in out, "an already-merged branch reports up-to-date")
        self.assertEqual(self.head(self.wt), before, "up-to-date moved nothing")

    # --- a tag with the branch's name does not win: the branch is what is merged -------

    def test_tag_named_like_branch(self) -> None:
        self.commit_file(self.src, "s2.txt", "s2")
        self.git(self.src, "tag", "t1")
        s3 = self.commit_file(self.src, "s3.txt", "s3")
        self.git(self.src, "branch", "t1")
        code, out = self.merge(self.wt, "t1")
        parents = self.parents(self.wt)
        self.assertTrue(code == 0 and "status=ok" in out,
                        f"merges the branch t1, not the tag (exit {code}): {out.strip()}")
        self.assertTrue(len(parents) == 3 and parents[2] == s3,
                        f"the second parent is the branch tip: {' '.join(parents)}")
        self.assertTrue((self.wt / "s3.txt").exists(), "the branch-only commit s3 is in the worktree")

    # --- --no-ff: a target that could fast-forward still gets a merge commit -----------

    def test_no_ff_when_fast_forwardable(self) -> None:
        ff_tip = self.make_ff()
        code, out = self.merge(self.wt, "feat/ff")
        parents = self.parents(self.wt)
        self.assertTrue(code == 0 and len(parents) == 3 and parents[2] == ff_tip,
                        f"a fast-forwardable merge still has two parents (exit {code}): {' '.join(parents)}")

    # --- the merge subject names the branch, not refs/heads/<branch> -------------------

    def test_merge_subject(self) -> None:
        self.make_ff()
        self.merge(self.wt, "feat/ff")
        subject = self.git(self.wt, "log", "-1", "--format=%s")
        self.assertEqual(subject, "Merge branch 'feat/ff' into feat/target", f"the merge subject is: {subject}")

    # --- the strategy is pinned: pull.twohead=ours must not drop the branch -------------

    def test_pull_twohead_does_not_drop_branch(self) -> None:
        th = self.tmp / "th"
        self.git(self.main, "worktree", "add", "-q", "-b", "feat/th", th, self.init_sha)
        self.commit_file(th, "th.txt", "th")
        self.commit_file(self.wt, "a.txt", "diverge for th")
        self.git(self.main, "config", "pull.twohead", "ours")
        code, out = self.merge(self.wt, "feat/th")
        self.git(self.main, "config", "--unset", "pull.twohead")
        self.assertTrue(code == 0 and (self.wt / "th.txt").exists(),
                        f"a configured pull.twohead=ours does not drop the branch's changes (exit {code}): {out.strip()}")

    # --- guards that nothing else covers -----------------------------------------------

    def test_guards_nothing_else_covers(self) -> None:
        before = self.head(self.wt)
        self.git(self.main, "branch", "feat/guard", self.init_sha)
        sub = self.main / "sub"
        sub.mkdir()
        main_before = self.head(self.main)
        code, out = self.merge(sub, "feat/guard")
        self.assertTrue(code == 2 and "not-the-worktree-root" in out,
                        f"refuses a subdirectory of the main checkout (exit {code}): {out.strip()}")
        self.assertEqual(self.head(self.main), main_before, "the main checkout did not move")
        self.git(self.wt, "checkout", "-q", "--detach")
        code, out = self.merge(self.wt, "feat/old")
        self.assertTrue(code == 2 and "detached-head" in out, f"refuses a detached HEAD (exit {code}): {out.strip()}")
        self.git(self.wt, "checkout", "-q", "feat/target")
        self.assertEqual(self.head(self.wt), before, "the guard refusals moved nothing")

    # --- GIT_OBJECT_DIRECTORY / GIT_DIR must not redirect the merge --------------------

    def test_env_redirects_do_not_redirect_merge(self) -> None:
        main_before = self.head(self.main)
        envwt = self.tmp / "envwt"
        self.git(self.main, "worktree", "add", "-q", "-b", "feat/env", envwt, self.init_sha)
        self.commit_file(envwt, "env.txt", "env")
        objdir = self.tmp / "odd-objects"
        objdir.mkdir()
        env = dict(self.env, GIT_OBJECT_DIRECTORY=objdir.as_posix(), GIT_DIR=(self.main / ".git").as_posix())
        code, out = self.merge(self.wt, "feat/env", env=env)
        self.assertTrue(code == 0 and "status=ok" in out and (self.wt / "env.txt").exists(),
                        f"merges with GIT_OBJECT_DIRECTORY and GIT_DIR set (exit {code}): {out.strip()}")
        self.assertEqual(self.head(self.main), main_before, "a GIT_DIR pointing at the main checkout did not move main")
        self.assertTrue(SHA.match(self.git(self.wt, "rev-parse", "--verify", "HEAD^{commit}")),
                        "the worktree branch tip is a readable commit")

    # --- a merge already in progress is the caller's: refused, never aborted -----------

    def test_merge_in_progress_is_refused(self) -> None:
        self.new_side("side/open", self.init_sha, "open.txt", "open")
        self.new_side("side/other", self.init_sha, "other.txt", "other")
        self.git(self.wt, "merge", "-q", "--no-commit", "--no-ff", "-s", "ours", "side/open", check=False)
        open_head = self.git(self.wt, "rev-parse", "-q", "--verify", "MERGE_HEAD", check=False)
        code, out = self.merge(self.wt, "side/other")
        self.assertTrue(code == 2 and "operation-in-progress: MERGE_HEAD" in out,
                        f"a merge in progress is refused (exit {code}): {out.strip()}")
        self.assertEqual(self.git(self.wt, "rev-parse", "-q", "--verify", "MERGE_HEAD", check=False), open_head,
                         "the caller's merge is still open")
        self.git(self.wt, "merge", "--abort", check=False)

    # --- branch.<current>.mergeOptions does not reach the merge ------------------------

    def test_branch_merge_options_do_not_reach_merge(self) -> None:
        # `-s ours` would drop the branch, `--no-commit` would leave the merge open, `-Xours` would
        # resolve the conflict silently, and a branch named `mo=x` needs the override another way.
        self.new_side("side/mo1", self.init_sha, "mo1.txt", "mo1")
        self.git(self.main, "config", "branch.feat/target.mergeoptions", "-s ours")
        code, out = self.merge(self.wt, "side/mo1")
        self.assertTrue(code == 0 and (self.wt / "mo1.txt").exists(),
                        f"mergeoptions '-s ours' does not drop the branch (exit {code}): {out.strip()}")
        self.new_side("side/mo2", self.init_sha, "mo2.txt", "mo2")
        self.git(self.main, "config", "branch.feat/target.mergeoptions", "--no-commit")
        code, out = self.merge(self.wt, "side/mo2")
        parents = self.parents(self.wt)
        left_open = self.git(self.wt, "rev-parse", "-q", "--verify", "MERGE_HEAD", check=False) != ""
        self.assertTrue(code == 0 and len(parents) == 3 and not left_open,
                        f"mergeoptions '--no-commit' does not leave the merge open (exit {code}): {out.strip()}")
        if left_open:
            self.git(self.wt, "merge", "--abort", check=False)
        self.new_side("side/mo3", self.init_sha, "a.txt", "mo3 side of a")
        self.git(self.main, "config", "branch.feat/target.mergeoptions", "-Xours")
        before = self.head(self.wt)
        code, out = self.merge(self.wt, "side/mo3")
        self.assertTrue(code == 1 and 'reason=merge-aborted files="a.txt"' in out and self.head(self.wt) == before,
                        f"mergeoptions '-Xours' does not resolve the conflict silently (exit {code}): {out.strip()}")
        self.git(self.main, "config", "--unset", "branch.feat/target.mergeoptions")
        eq_wt = self.tmp / "eq"
        self.git(self.main, "worktree", "add", "-q", "-b", "mo=x", eq_wt, self.init_sha)
        self.new_side("side/mo4", self.init_sha, "mo4.txt", "mo4")
        self.git(self.main, "config", "branch.mo=x.mergeoptions", "-s ours")
        code, out = self.merge(eq_wt, "side/mo4")
        self.assertTrue(code == 0 and (eq_wt / "mo4.txt").exists(),
                        f"mergeoptions of a branch named 'mo=x' is overridden too (exit {code}): {out.strip()}")

    # --- a conflicted file name cannot plant a status= token in the failed line --------

    def test_conflicted_name_with_equals_is_encoded(self) -> None:
        inj_name = "q status=ok.txt"
        inj = self.tmp / "inj"
        self.git(self.main, "worktree", "add", "-q", "-b", "feat/inj", inj, self.init_sha)
        self.commit_file(inj, inj_name, "target")
        self.new_side("side/inj", self.init_sha, inj_name, "side")
        code, out = self.merge(inj, "side/inj")
        self.assertTrue(code == 1 and 'files="q status%3Dok.txt"' in out and out.count("status=") == 1,
                        f"a file name holding status= is encoded (exit {code}): {out.strip()}")

    # --- conflict: aborted, worktree back to its pre-merge state -----------------------

    def test_conflict_is_aborted(self) -> None:
        conf = self.tmp / "conf"
        self.git(self.main, "worktree", "add", "-q", conf, "feat/conf")
        self.commit_file(conf, "a.txt", "conflicting")
        self.commit_file(self.wt, "a.txt", "other side")
        before = self.head(self.wt)
        code, out = self.merge(self.wt, "feat/conf")
        self.assertTrue(code == 1 and re.search(
            r'status=failed reason=merge-aborted files="a.txt" detail="CONFLICT \(content\): Merge conflict in a\.txt"',
            out), f"a conflict exits 1, names the file and carries git's CONFLICT line (exit {code}): {out.strip()}")
        self.assertEqual(self.head(self.wt), before, "the aborted merge left HEAD where it was")
        self.assertEqual(self.git(self.wt, "rev-parse", "-q", "--verify", "MERGE_HEAD", check=False), "",
                         "no merge is left in progress")
        self.assertEqual(self.git(self.wt, "status", "--porcelain"), "", "the tree is clean after the abort")


if __name__ == "__main__":
    unittest.main()
