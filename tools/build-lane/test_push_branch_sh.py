"""Tests for push-branch.sh, run through Git Bash, in a throwaway repository with a local bare origin.

Ported from test-push-branch.ps1: one method per section, one assertion per Check. No network.
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

SCRIPT = HERE / "push-branch.sh"
BASH = find_bash()


@unittest.skipUnless(BASH, "no bash: Git for Windows missing or LANE_TESTS_NO_BASH=1")
class PushBranchShTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="push-branch-sh-"))
        empty = self.tmp / "empty.gitconfig"
        empty.write_text("", encoding="utf-8")
        self.env = dict(os.environ, GIT_CONFIG_GLOBAL=str(empty), GIT_CONFIG_SYSTEM=str(empty))
        self.bare = self.tmp / "origin.git"
        self.main = self.tmp / "main"
        self.wt = self.tmp / "wt"
        self.git(self.tmp, "init", "-q", "--bare", "-b", "main", str(self.bare))
        self.main.mkdir()
        self.git(self.main, "init", "-q", "-b", "main")
        self.git(self.main, "config", "user.email", "test@example.invalid")
        self.git(self.main, "config", "user.name", "test")
        self.write(self.main / "a.txt", "one")
        self.git(self.main, "add", "--", "a.txt")
        self.git(self.main, "commit", "-q", "-m", "init")
        self.git(self.main, "remote", "add", "origin", str(self.bare))
        self.git(self.main, "push", "-q", "-u", "origin", "main")
        self.git(self.main, "worktree", "add", "-q", "-b", "feat/x", str(self.wt))
        self.write(self.wt / "b.txt", "two")
        self.git(self.wt, "add", "--", "b.txt")
        self.git(self.wt, "commit", "-q", "-m", "feat one")

    def tearDown(self) -> None:
        shutil.rmtree(self.tmp, ignore_errors=True)

    def git(self, cwd: Path, *args: str, env: dict[str, str] | None = None) -> str:
        r = subprocess.run(["git", "-C", str(cwd), *args], env=env or self.env,
                           stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                           text=True, encoding="utf-8", errors="replace")
        if r.returncode != 0:
            raise AssertionError(f"git {' '.join(args)} failed: {r.stderr.strip()}")
        return r.stdout.strip()

    @staticmethod
    def write(path: Path, text: str) -> None:
        with open(path, "w", encoding="utf-8", newline="") as f:
            f.write(text + "\n")

    def remote_ref(self, ref: str) -> str | None:
        """The sha of a ref in the bare origin, or None when the ref does not exist."""
        r = subprocess.run(["git", "-C", str(self.bare), "rev-parse", "--verify", "--quiet", ref],
                           env=self.env, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                           text=True, encoding="utf-8", errors="replace")
        return r.stdout.strip() if r.returncode == 0 else None

    def run_sh(self, *argv: str, env: dict[str, str] | None = None) -> tuple[int, str]:
        r = subprocess.run([BASH, SCRIPT.as_posix(), *argv], env=env or self.env,
                           stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                           text=True, encoding="utf-8", errors="replace")
        return r.returncode, r.stdout

    def push(self, wt: Path, branch: str, *extra: str, env: dict[str, str] | None = None) -> tuple[int, str]:
        return self.run_sh("-C", wt.as_posix(), "-Branch", branch, *extra, env=env)

    def push_plain(self) -> str:
        """Publish feat/x as it is; returns the pushed sha."""
        code, out = self.push(self.wt, "feat/x")
        assert code == 0, out
        return self.git(self.wt, "rev-parse", "HEAD")

    def amend(self, message: str) -> str:
        self.git(self.wt, "commit", "-q", "--amend", "-m", message)
        return self.git(self.wt, "rev-parse", "HEAD")

    # --- refusals push nothing --------------------------------------------------------

    def test_refusals_push_nothing(self) -> None:
        code, out = self.push(self.main, "main")
        self.assertTrue(code == 2 and "main-checkout" in out, "refuses the main checkout")
        code, out = self.push(self.wt / "nope", "feat/x")
        self.assertTrue(code == 2 and "not-a-directory" in out, "refuses a path that is not a directory")
        code, out = self.push(self.wt, "main")
        self.assertTrue(code == 2 and "protected-branch: main" in out, "refuses to push main")
        code, out = self.push(self.wt, "master")
        self.assertTrue(code == 2 and "protected-branch: master" in out, "refuses to push master")
        code, out = self.push(self.wt, "other")
        self.assertTrue(code == 2 and "not-current-branch" in out, "refuses a branch that is not the current one")
        code, out = self.push(self.wt, "bad..name")
        self.assertTrue(code == 2 and "bad-branch-name" in out, "refuses an invalid branch name")
        self.assertIsNone(self.remote_ref("refs/heads/feat/x"), "no refusal created the remote branch")
        self.git(self.wt, "checkout", "-q", "--detach")
        code, out = self.push(self.wt, "feat/x")
        self.assertTrue(code == 2 and "detached-head" in out, "refuses a detached HEAD")
        self.git(self.wt, "checkout", "-q", "feat/x")

    # --- a plain push publishes the branch and sets its upstream ----------------------

    def test_plain_push_publishes_branch(self) -> None:
        code, out = self.push(self.wt, "feat/x")
        head = self.git(self.wt, "rev-parse", "HEAD")
        self.assertTrue(code == 0 and f"status=ok branch=feat/x sha={head}" in out,
                        f"plain push ok (exit {code}): {out.strip()}")
        self.assertEqual(self.remote_ref("refs/heads/feat/x"), head, "the remote branch is at HEAD")
        self.assertEqual(self.git(self.wt, "rev-parse", "--abbrev-ref", "@{u}"), "origin/feat/x",
                         "the upstream is origin/feat/x")

    # --- a rewritten branch needs -Force; a plain push is rejected -------------------

    def test_rewritten_branch_needs_force(self) -> None:
        head = self.push_plain()
        self.amend("feat one rewritten")
        code, out = self.push(self.wt, "feat/x")
        self.assertTrue(code == 1 and re.search(r'status=failed reason=git-push branch=feat/x detail="! \[rejected\][^"]*"', out),
                        f"a plain push of a rewritten branch fails and says why (exit {code}): {out.strip()}")
        self.assertEqual(self.remote_ref("refs/heads/feat/x"), head, "the failed plain push left the remote unchanged")

    # -Force needs the remote sha the caller last saw: a bare lease is defeated by any fetch.

    def test_force_needs_expect_remote_sha(self) -> None:
        head = self.push_plain()
        self.amend("feat one rewritten")
        code, out = self.push(self.wt, "feat/x", "-Force")
        self.assertTrue(code == 2 and "force-needs-expect-remote-sha" in out,
                        f"-Force without -ExpectRemoteSha is refused (exit {code}): {out.strip()}")
        code, out = self.push(self.wt, "feat/x", "-Force", "-ExpectRemoteSha", head.upper())
        self.assertTrue(code == 2 and "force-needs-expect-remote-sha" in out,
                        "-ExpectRemoteSha must be 40 lowercase hex digits")
        code, out = self.push(self.wt, "feat/x", "-ExpectRemoteSha", head)
        self.assertTrue(code == 2 and "expect-remote-sha-needs-force" in out,
                        "-ExpectRemoteSha without -Force is refused")
        self.assertEqual(self.remote_ref("refs/heads/feat/x"), head, "the -Force refusals left the remote unchanged")

    def test_force_with_current_sha_pushes_rewritten(self) -> None:
        head = self.push_plain()
        rewritten = self.amend("feat one rewritten")
        code, out = self.push(self.wt, "feat/x", "-Force", "-ExpectRemoteSha", head)
        self.assertTrue(code == 0 and f"sha={rewritten}" in out,
                        f"-Force with the current remote sha pushes the rewritten branch (exit {code})")
        self.assertEqual(self.remote_ref("refs/heads/feat/x"), rewritten, "the remote branch is at the rewritten HEAD")

    # --- the lease holds: someone else's push to the remote is not overwritten ---------
    # A second clone advances origin/feat/x after our last push. The lease still expects our tip,
    # so -Force must be rejected; a bare --force would overwrite it and this check would fail.

    def test_lease_holds_against_other_push(self) -> None:
        self.push_plain()
        rewritten = self.amend("feat one rewritten")
        code, _ = self.push(self.wt, "feat/x", "-Force", "-ExpectRemoteSha", self.remote_ref("refs/heads/feat/x") or "")
        self.assertEqual(code, 0, "setup: the informed force publishes the rewritten branch")
        other = self.tmp / "other"
        self.git(self.tmp, "clone", "-q", str(self.bare), str(other))
        self.git(other, "config", "user.email", "test@example.invalid")
        self.git(other, "config", "user.name", "test")
        self.git(other, "checkout", "-q", "feat/x")
        self.write(other / "c.txt", "someone else")
        self.git(other, "add", "--", "c.txt")
        self.git(other, "commit", "-q", "-m", "remote advances")
        self.git(other, "push", "-q", "origin", "feat/x")
        theirs = self.git(other, "rev-parse", "HEAD")
        self.amend("feat one rewritten again")
        code, out = self.push(self.wt, "feat/x", "-Force", "-ExpectRemoteSha", rewritten)
        self.assertTrue(code == 1 and "status=failed reason=git-push" in out,
                        f"-Force is rejected when the remote moved (exit {code}): {out.strip()}")
        self.assertEqual(self.remote_ref("refs/heads/feat/x"), theirs, "the rejected -Force left the other push in place")
        # A fetch refreshes origin/feat/x, which a bare --force-with-lease then trusts and overwrites.
        # The explicit expected sha is the one the caller saw before the remote moved, so it still fails.
        self.git(self.wt, "fetch", "-q", "origin")
        self.assertEqual(self.git(self.wt, "rev-parse", "origin/feat/x"), theirs,
                         "the fetch refreshed the remote-tracking ref")
        code, out = self.push(self.wt, "feat/x", "-Force", "-ExpectRemoteSha", rewritten)
        self.assertTrue(code == 1 and "status=failed reason=git-push" in out,
                        f"-Force after a fetch is still rejected (exit {code}): {out.strip()}")
        self.assertEqual(self.remote_ref("refs/heads/feat/x"), theirs, "the fetch-then-force left the other push in place")
        code, out = self.push(self.wt, "feat/x", "-Force", "-ExpectRemoteSha", theirs)
        self.assertTrue(code == 0 and "status=ok" in out,
                        f"-Force with the sha the caller really saw succeeds (exit {code})")
        self.assertEqual(self.remote_ref("refs/heads/feat/x"), self.git(self.wt, "rev-parse", "HEAD"),
                         "the remote is at the local tip after the informed force")

    # --- the push runs in the repository named by -C, whatever GIT_DIR says ----------------
    # `other` holds feat/x at a commit the remote does not have, so a push from it would be a
    # rejected non-fast-forward; the script has to clear GIT_DIR and push from the worktree.

    def test_git_dir_is_cleared(self) -> None:
        self.push_plain()
        other = self.tmp / "other"
        self.git(self.tmp, "clone", "-q", str(self.bare), str(other))
        self.git(other, "config", "user.email", "test@example.invalid")
        self.git(other, "config", "user.name", "test")
        self.git(other, "checkout", "-q", "feat/x")
        self.write(other / "o.txt", "other local")
        self.git(other, "add", "--", "o.txt")
        self.git(other, "commit", "-q", "-m", "other local")
        self.write(self.wt / "c.txt", "three")
        self.git(self.wt, "add", "--", "c.txt")
        self.git(self.wt, "commit", "-q", "-m", "feat two")
        env = dict(self.env, GIT_DIR=str(other / ".git"))
        code, out = self.push(self.wt, "feat/x", env=env)
        self.assertTrue(code == 0 and "status=ok" in out,
                        f"pushes the worktree's branch with GIT_DIR set to another repository (exit {code}): {out.strip()}")
        self.assertEqual(self.remote_ref("refs/heads/feat/x"), self.git(self.wt, "rev-parse", "HEAD"),
                         "the remote holds the worktree's HEAD, not the other clone's commit")

    # --- -C must be the worktree root, not a subdirectory of the main checkout ---------------

    def test_c_must_be_worktree_root(self) -> None:
        main_sub = self.main / "sub"
        main_sub.mkdir()
        code, out = self.push(main_sub, "main")
        self.assertTrue(code == 2 and "not-the-worktree-root" in out,
                        f"refuses a subdirectory of the main checkout (exit {code}): {out.strip()}")
        code, _ = self.push(self.wt / "sub-none", "feat/x")
        self.assertEqual(code, 2, "refuses a path below the worktree that is not a directory")

    # --- the full refspec is pushed: a tag named like the branch must not make the push ambiguous ----

    def test_tag_named_like_branch(self) -> None:
        wt_amb = self.tmp / "wtamb"
        self.git(self.main, "worktree", "add", "-q", "-b", "amb", str(wt_amb), "main")
        self.write(wt_amb / "amb.txt", "amb")
        self.git(wt_amb, "add", "--", "amb.txt")
        self.git(wt_amb, "commit", "-q", "-m", "amb commit")
        self.git(wt_amb, "tag", "amb", "main")
        code, out = self.push(wt_amb, "amb")
        self.assertTrue(code == 0 and "status=ok" in out,
                        f"pushes branch amb although a tag amb exists (exit {code}): {out.strip()}")
        self.assertEqual(self.git(self.bare, "rev-parse", "refs/heads/amb"), self.git(wt_amb, "rev-parse", "HEAD"),
                         "the remote branch amb is at the branch tip, not the tag")

    # --- the branch name compare is case-sensitive -----------------------------------------

    def test_branch_name_compare_is_case_sensitive(self) -> None:
        code, out = self.push(self.wt, "FEAT/X")
        self.assertTrue(code == 2 and "not-current-branch" in out,
                        f"refuses -Branch FEAT/X for the branch feat/x (exit {code})")

    # --- a name that starts with + is refused: pushed raw it is a force refspec ------------
    # Remote `victim` holds a commit that local branch `+victim` does not contain. Pushing the raw
    # name `+victim` would force-push `victim` over it and bypass the main/master check.

    def test_plus_names_are_refused(self) -> None:
        victim = self.git(self.bare, "rev-parse", "refs/heads/main")
        self.git(self.bare, "branch", "victim", "main")
        wt_plus = self.tmp / "wtplus"
        self.git(self.main, "worktree", "add", "-q", "-b", "+victim", str(wt_plus), "main")
        self.write(wt_plus / "v.txt", "v")
        self.git(wt_plus, "add", "--", "v.txt")
        self.git(wt_plus, "commit", "-q", "-m", "victim rewrite")
        for plus in ("+victim", "+main"):
            if plus == "+main":
                self.git(wt_plus, "checkout", "-q", "-b", "+main")
            code, out = self.push(wt_plus, plus)
            self.assertTrue(code == 2 and "bad-branch-name" in out,
                            f"refuses the branch '{plus}' (exit {code}): {out.strip()}")
            code, out = self.push(wt_plus, plus, "-Force")
            self.assertTrue(code == 2 and "bad-branch-name" in out,
                            f"refuses the branch '{plus}' with -Force too (exit {code})")
        self.assertEqual(self.git(self.bare, "rev-parse", "refs/heads/victim"), victim,
                         "the remote victim branch was not overwritten")
        self.assertIsNone(self.remote_ref("refs/heads/+victim"), "no +victim branch was created on the remote")


if __name__ == "__main__":
    unittest.main()
