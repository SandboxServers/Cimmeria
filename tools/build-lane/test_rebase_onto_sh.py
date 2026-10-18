"""Integration tests for rebase-onto.sh, the bash twin of rebase-onto.ps1.

Each test builds a throwaway main checkout plus linked worktrees (the "remote" is never needed),
runs the script under Git Bash and asserts the same Checks as test-rebase-onto.ps1, one per assertion.
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

SCRIPT = HERE / "rebase-onto.sh"
BASH = find_bash()


def matches(pattern: str, text: str) -> bool:
    """PowerShell's -match: a case-insensitive regex search."""
    return re.search(pattern, text, re.IGNORECASE) is not None


@unittest.skipUnless(BASH, "no bash: Git for Windows missing or LANE_TESTS_NO_BASH=1")
class RebaseOntoShTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="rebase-onto-sh-"))
        empty = self.tmp / "empty.gitconfig"
        empty.write_text("", encoding="utf-8")
        self.env = dict(os.environ, GIT_CONFIG_GLOBAL=str(empty), GIT_CONFIG_SYSTEM=str(empty))
        self.main = self.tmp / "main"
        self.wt = self.tmp / "wt"

    def tearDown(self) -> None:
        shutil.rmtree(self.tmp, ignore_errors=True)

    def run_sh(self, *argv: str, env: dict[str, str] | None = None) -> tuple[int, str]:
        r = subprocess.run([BASH, SCRIPT.as_posix(), *argv], env=env or self.env,
                           stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                           text=True, encoding="utf-8", errors="replace")
        return r.returncode, r.stdout

    def rebase(self, wt: Path, onto: str, upstream: str = "", env: dict[str, str] | None = None) -> tuple[int, str]:
        argv = ["-C", wt.as_posix(), "-Onto", onto]
        if upstream:
            argv += ["-Upstream", upstream]
        return self.run_sh(*argv, env=env)

    def git(self, cwd: Path, *args: str, check: bool = True) -> str:
        r = subprocess.run(["git", "-C", str(cwd), *args], env=self.env,
                           stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                           text=True, encoding="utf-8", errors="replace", check=check)
        return r.stdout.strip()

    def head(self, d: Path) -> str:
        return self.git(d, "rev-parse", "HEAD")

    def commit(self, d: Path, name: str, msg: str) -> str:
        """Writes a file whose content is msg, commits it and returns the new HEAD."""
        (d / name).write_text(msg, encoding="utf-8", newline="")
        self.git(d, "add", "--", name)
        self.git(d, "commit", "-q", "-m", msg)
        return self.head(d)

    def make_main(self) -> None:
        self.main.mkdir()
        self.git(self.main, "init", "-q", "-b", "main")
        self.git(self.main, "config", "user.email", "test@example.invalid")
        self.git(self.main, "config", "user.name", "test")
        self.commit(self.main, "a.txt", "one")

    def base(self) -> str:
        """Main with a worktree on feat/x (one commit), then main advances. Returns main's tip."""
        self.make_main()
        self.git(self.main, "worktree", "add", "-q", "-b", "feat/x", str(self.wt))
        self.commit(self.wt, "b.txt", "feat one")
        return self.commit(self.main, "c.txt", "main advances")

    # --- refusals leave the branch alone ---------------------------------------------

    def test_refusals_leave_branch_alone(self) -> None:
        main_tip = self.base()
        code, out = self.rebase(self.main, main_tip)
        self.assertTrue(code == 2 and matches("main-checkout", out), "refuses the main checkout")
        code, out = self.rebase(self.wt, "0" * 40)
        self.assertTrue(code == 2 and matches("not-a-commit", out), "refuses an -Onto that names no commit")
        for loose in ("main", "HEAD", main_tip[:12], main_tip.upper()):
            code, out = self.rebase(self.wt, loose)
            self.assertTrue(code == 2 and matches("sha-not-full-40-hex", out),
                            f"refuses -Onto '{loose}', which is not a full 40-hex SHA")
            code, out = self.rebase(self.wt, main_tip, loose)
            self.assertTrue(code == 2 and matches("sha-not-full-40-hex", out),
                            f"refuses -Upstream '{loose}', which is not a full 40-hex SHA")
        (self.wt / "dirty.txt").write_text("untracked", encoding="utf-8", newline="")
        before = self.head(self.wt)
        code, out = self.rebase(self.wt, main_tip)
        self.assertTrue(code == 2 and matches("dirty-tree", out), "refuses a dirty tree")
        self.assertTrue(self.head(self.wt) == before, "the dirty refusal did not move HEAD")
        (self.wt / "dirty.txt").unlink()

    # --- a clean rebase replays the branch onto the new tip --------------------------

    def test_clean_rebase_replays_onto_new_tip(self) -> None:
        main_tip = self.base()
        code, out = self.rebase(self.wt, main_tip)
        tip = self.head(self.wt)
        self.assertTrue(code == 0 and matches(f"status=ok tip={tip} branch=feat/x", out),
                        f"rebases onto main (exit {code}): {out.strip()}")
        self.assertTrue(self.git(self.wt, "merge-base", "HEAD", "main") == main_tip, "HEAD now contains main")
        self.assertTrue((self.wt / "b.txt").exists(), "the branch commit survived the rebase")
        self.assertTrue(self.git(self.wt, "status", "--porcelain") == "", "the tree is clean after the rebase")
        code, out = self.rebase(self.wt, main_tip)
        self.assertTrue(code == 0 and matches("status=up-to-date", out),
                        f"a second rebase onto main is up to date (exit {code})")

    # --- -Upstream must be an ancestor of HEAD ----------------------------------------

    def test_upstream_must_be_ancestor(self) -> None:
        self.base()
        new_main = self.commit(self.main, "d.txt", "main d")
        code, out = self.rebase(self.wt, new_main, new_main)
        self.assertTrue(code == 2 and matches("upstream-not-ancestor", out),
                        "refuses an -Upstream that HEAD does not contain")

    # --- a conflict is aborted and leaves the worktree as it was ---------------------

    def test_conflict_is_aborted(self) -> None:
        self.base()
        new_main = self.commit(self.main, "d.txt", "main d")
        self.commit(self.wt, "d.txt", "feat d")
        before = self.head(self.wt)
        code, out = self.rebase(self.wt, new_main)
        gitdir = Path(self.git(self.wt, "rev-parse", "--absolute-git-dir"))
        self.assertTrue(
            code == 1 and matches(r'status=failed reason=rebase-aborted branch=feat/x detail="[^"]*(?:CONFLICT|could not apply)', out),
            f"a conflicting rebase fails, aborts and says why (exit {code}): {out.strip()}")
        self.assertTrue(self.head(self.wt) == before, "the conflict left HEAD where it was")
        self.assertTrue(self.git(self.wt, "status", "--porcelain") == "", "the conflict left the tree clean")
        self.assertFalse((gitdir / "rebase-merge").exists() or (gitdir / "rebase-apply").exists(),
                         "no rebase is left in progress")

    # --- a rebase left by someone else is refused, not aborted -------------------------

    def test_stale_rebase_directory_is_refused(self) -> None:
        # An interrupted rebase, then `checkout -f` back onto the branch and a new commit: HEAD is
        # attached and the tree clean, but the stale rebase-merge directory remains.
        self.base()
        new_main = self.commit(self.main, "d.txt", "main d")
        self.commit(self.wt, "d.txt", "feat d")
        before = self.head(self.wt)
        self.git(self.wt, "rebase", new_main, check=False)
        gitdir = Path(self.git(self.wt, "rev-parse", "--absolute-git-dir"))
        self.assertTrue((gitdir / "rebase-merge").exists(), "setup: a hand-run rebase stopped on its conflict")
        self.git(self.wt, "checkout", "-q", "-f", "feat/x")
        (self.wt / "precious.txt").write_text("precious work", encoding="utf-8", newline="")
        precious = self.commit(self.wt, "precious.txt", "precious work")
        code, out = self.rebase(self.wt, new_main, self.git(self.wt, "rev-parse", "HEAD~2"))
        self.assertTrue(code == 2 and matches("operation-in-progress: rebase-merge", out),
                        f"a stale rebase directory is refused (exit {code}): {out.strip()}")
        self.assertTrue(self.git(self.wt, "rev-parse", "refs/heads/feat/x") == precious
                        and (self.wt / "precious.txt").exists(),
                        "the stale-rebase refusal kept the branch and its newest commit")
        self.git(self.wt, "rebase", "--quit", check=False)
        self.git(self.wt, "merge", "-q", "--no-commit", "--no-ff", "-s", "ours", new_main, check=False)
        code, out = self.rebase(self.wt, new_main)
        self.assertTrue(code == 2 and matches("operation-in-progress: MERGE_HEAD", out),
                        f"a merge in progress is refused (exit {code}): {out.strip()}")
        self.git(self.wt, "merge", "--abort", check=False)
        self.git(self.wt, "reset", "-q", "--hard", before)

    # --- guards that nothing else covers ------------------------------------------------

    def test_guards_nothing_else_covers(self) -> None:
        main_tip = self.base()
        sub = self.main / "sub"
        sub.mkdir()
        main_head = self.head(self.main)
        code, out = self.rebase(sub, main_tip)
        self.assertTrue(code == 2 and matches("not-the-worktree-root", out),
                        f"refuses a subdirectory of the main checkout (exit {code}): {out.strip()}")
        self.assertTrue(self.head(self.main) == main_head, "the main checkout did not move")
        before = self.head(self.wt)
        self.git(self.wt, "checkout", "-q", "--detach")
        code, out = self.rebase(self.wt, main_tip)
        self.assertTrue(code == 2 and matches("detached-head", out),
                        f"refuses a detached HEAD (exit {code}): {out.strip()}")
        self.git(self.wt, "checkout", "-q", "feat/x")
        self.assertTrue(self.head(self.wt) == before, "the guard refusals moved nothing")

    # --- GIT_DIR and GIT_OBJECT_DIRECTORY in the environment must not retarget the rebase ---

    def test_git_dir_env_does_not_retarget(self) -> None:
        self.make_main()
        wt_e = self.tmp / "wte"
        e_base = self.head(self.main)
        self.git(self.main, "worktree", "add", "-q", "-b", "feat/e", str(wt_e), e_base)
        self.commit(wt_e, "e1.txt", "e1")
        e_onto = self.commit(self.main, "e0.txt", "main e0")
        obj_dir = self.tmp / "odd-objects"
        obj_dir.mkdir()
        env = dict(self.env, GIT_DIR=(self.main / ".git").as_posix(), GIT_OBJECT_DIRECTORY=obj_dir.as_posix())
        code, out = self.rebase(wt_e, e_onto, env=env)
        self.assertTrue(code == 0 and matches("status=ok", out) and matches("branch=feat/e", out),
                        f"rebases with GIT_DIR and GIT_OBJECT_DIRECTORY set (exit {code}): {out.strip()}")
        self.assertTrue(self.head(self.main) == e_onto, "main did not move")
        self.assertTrue(self.git(wt_e, "merge-base", "HEAD", e_onto) == e_onto, "feat/e now sits on the new tip")

    # --- the -Upstream (--onto) path: success, stacked branches, and no hidden reset ----

    def test_upstream_path_rebases_stacked_branches(self) -> None:
        self.make_main()
        base = self.head(self.main)
        wt_y = self.tmp / "wty"
        wt_z = self.tmp / "wtz"
        self.git(self.main, "worktree", "add", "-q", "-b", "feat/y", str(wt_y), base)
        y_tip = self.commit(wt_y, "y.txt", "y1")
        self.git(self.main, "worktree", "add", "-q", "-b", "feat/z", str(wt_z), y_tip)
        self.commit(wt_z, "z.txt", "z1")
        new_tip = self.commit(self.main, "e.txt", "main e")
        # The parent branch is rebased first; its tip changes, so the child still sits on the old y1.
        code, out = self.rebase(wt_y, new_tip, base)
        self.assertTrue(code == 0 and matches("status=ok", out),
                        f"-Upstream rebases the parent branch (exit {code}): {out.strip()}")
        new_y = self.head(wt_y)
        self.assertTrue(self.git(wt_y, "rev-parse", "HEAD~1") == new_tip,
                        "the parent branch now sits on the new main tip")
        # The child replays only z1 on top of the rewritten parent, not the old y1 again.
        code, out = self.rebase(wt_z, new_y, y_tip)
        self.assertTrue(code == 0 and matches("status=ok", out),
                        f"-Upstream rebases a stacked branch (exit {code}): {out.strip()}")
        self.assertTrue(self.git(wt_z, "rev-parse", "HEAD~1") == new_y,
                        "the stacked branch sits directly on the new parent tip")
        subjects = self.git(wt_z, "log", "--format=%s", f"{new_tip}..HEAD").splitlines()
        self.assertTrue(",".join(subjects) == "z1,y1",
                        f"the stacked branch holds z1 and one y1 only: {','.join(subjects)}")
        # -Upstream equal to HEAD would replay nothing and move the branch to -Onto: a hidden reset.
        before = self.head(wt_z)
        code, out = self.rebase(wt_z, base, before)
        self.assertTrue(code == 2 and matches("nothing-to-replay", out), "-Upstream equal to HEAD is refused")
        self.assertTrue(self.head(wt_z) == before, "the nothing-to-replay refusal did not move HEAD")
        # An -Onto already inside HEAD with an -Upstream above it would drop the commits between them.
        code, out = self.rebase(wt_z, base, new_y)
        self.assertTrue(code == 2 and matches("would-drop-commits", out),
                        f"-Onto below -Upstream is refused (exit {code}): {out.strip()}")
        self.assertTrue(self.head(wt_z) == before, "the would-drop-commits refusal did not move HEAD")
        self.assertTrue((wt_z / "y.txt").exists(), "the commits that would have been dropped are still there")

    # --- a merge commit on the branch survives the rebase (--rebase-merges) -----------

    def test_merge_commit_survives(self) -> None:
        self.make_main()
        m_base = self.head(self.main)
        wt_m = self.tmp / "wtm"
        wt_s = self.tmp / "wts"
        self.git(self.main, "worktree", "add", "-q", "-b", "feat/m", str(wt_m), m_base)
        self.commit(wt_m, "m1.txt", "m1")
        self.git(self.main, "worktree", "add", "-q", "-b", "side", str(wt_s), m_base)
        self.commit(wt_s, "s1.txt", "s1")
        self.git(wt_m, "merge", "-q", "--no-ff", "-m", "merge side", "side")
        m_tip = self.commit(self.main, "f.txt", "main f")
        self.assertTrue(len(self.git(wt_m, "rev-list", "--merges", f"{m_base}..HEAD").splitlines()) == 1,
                        "setup: the branch holds one merge commit")
        code, out = self.rebase(wt_m, m_tip, m_base)
        self.assertTrue(code == 0 and matches("status=ok", out),
                        f"a branch with a merge commit rebases (exit {code}): {out.strip()}")
        self.assertTrue(len(self.git(wt_m, "rev-list", "--merges", f"{m_tip}..HEAD").splitlines()) == 1,
                        "the merge commit survived the rebase")

    # --- rebase.updateRefs=true must not move other local branches -------------------

    def test_update_refs_config_is_ignored(self) -> None:
        self.make_main()
        m_tip = self.head(self.main)
        wt_u = self.tmp / "wtu"
        self.git(self.main, "worktree", "add", "-q", "-b", "feat/u", str(wt_u), m_tip)
        self.commit(wt_u, "u1.txt", "u1")
        self.git(wt_u, "branch", "inner")
        inner = self.git(self.main, "rev-parse", "refs/heads/inner")
        self.commit(wt_u, "u2.txt", "u2")
        g_tip = self.commit(self.main, "g.txt", "main g")
        self.git(self.main, "config", "rebase.updateRefs", "true")
        # A tag with the branch's name makes `rev-parse --abbrev-ref HEAD` answer `heads/feat/u`.
        self.git(self.main, "tag", "feat/u", m_tip)
        code, out = self.rebase(wt_u, g_tip)
        self.assertTrue(code == 0 and matches("status=ok", out),
                        f"rebases with rebase.updateRefs=true set (exit {code}): {out.strip()}")
        self.assertTrue(matches(r"branch=feat/u(?:\s|$)", out),
                        f"the branch is named plainly although a tag has its name: {out.strip()}")
        self.assertTrue(self.git(wt_u, "merge-base", "HEAD", g_tip) == g_tip,
                        "setup: the branch now sits on the new main tip")
        self.assertTrue(self.git(self.main, "rev-parse", "refs/heads/inner") == inner,
                        "the unrelated branch inside the range was not moved")


if __name__ == "__main__":
    unittest.main()
