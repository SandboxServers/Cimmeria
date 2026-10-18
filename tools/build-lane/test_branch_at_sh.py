"""Tests for branch-at.sh in a throwaway repository with a linked worktree, the bash twin of
test-branch-at.ps1. Each method starts from a fresh fixture; no network, no cargo.

    python -m unittest discover -s tools/build-lane -p "test_branch_at_sh*.py" -v
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

SCRIPT = HERE / "branch-at.sh"
BASH = find_bash()


@unittest.skipUnless(BASH, "no bash: Git for Windows missing or LANE_TESTS_NO_BASH=1")
class BranchAtShTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="branch-at-sh-"))
        # Isolate git from the developer's global and system config (commit.gpgsign, core.hooksPath,
        # rebase.updateRefs ...): both point at an empty file inside the throwaway directory.
        empty = self.tmp / "empty.gitconfig"
        empty.write_bytes(b"")
        self.env = dict(os.environ, GIT_CONFIG_GLOBAL=str(empty), GIT_CONFIG_SYSTEM=str(empty))

    def tearDown(self) -> None:
        shutil.rmtree(self.tmp, ignore_errors=True)

    def run_sh(self, *argv: str, env: dict[str, str] | None = None) -> tuple[int, str]:
        r = subprocess.run([BASH, SCRIPT.as_posix(), *argv], env=env or self.env,
                           stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                           text=True, encoding="utf-8", errors="replace")
        return r.returncode, r.stdout

    def branch_at(self, repo: Path, name: str, sha: str) -> tuple[int, str]:
        return self.run_sh("-C", repo.as_posix(), "-Name", name, "-Sha", sha)

    def git(self, repo: Path, *args: str) -> str:
        r = subprocess.run(["git", "-C", repo.as_posix(), *args], env=self.env,
                           stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                           text=True, encoding="utf-8", errors="replace")
        self.assertEqual(r.returncode, 0, f"git {' '.join(args)}: {r.stdout}")
        return r.stdout

    def git_rc(self, repo: Path, *args: str) -> int:
        return subprocess.run(["git", "-C", repo.as_posix(), *args], env=self.env,
                              stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode

    def build_fixture(self) -> None:
        # The same repository as test-branch-at.ps1: main with two commits, a linked worktree on
        # feat/wt, a remote named origin and a tag v9 on the first commit.
        self.main = self.tmp / "main"
        self.wt = self.tmp / "wt"
        self.main.mkdir()
        self.git(self.main, "init", "-q", "-b", "main")
        self.git(self.main, "config", "user.email", "test@example.invalid")
        self.git(self.main, "config", "user.name", "test")
        (self.main / "a.txt").write_bytes(b"one\n")
        self.git(self.main, "add", "--", "a.txt")
        self.git(self.main, "commit", "-q", "-m", "first")
        self.first = self.git(self.main, "rev-parse", "HEAD").strip()
        (self.main / "a.txt").write_bytes(b"two\n")
        self.git(self.main, "commit", "-q", "-am", "second")
        self.second = self.git(self.main, "rev-parse", "HEAD").strip()
        self.git(self.main, "worktree", "add", "-q", "-b", "feat/wt", self.wt.as_posix())
        self.git(self.main, "remote", "add", "origin", (self.tmp / "origin.git").as_posix())
        self.git(self.main, "tag", "v9", self.first)

    def expect(self, msg: str, code: int, want_code: int, pattern: str, out: str) -> None:
        ok = code == want_code and re.search(pattern, out) is not None
        self.assertTrue(ok, f"{msg} (exit {code}): {out.strip()}")

    def test_refusals(self) -> None:
        self.build_fixture()
        main, first, second = self.main, self.first, self.second
        refs_before = self.git(main, "for-each-ref", "--format=%(refname) %(objectname)")
        code, out = self.branch_at(main, "feat/a", first.upper())
        self.expect("refuses an upper-case SHA", code, 2, "sha-not-full-40-hex", out)
        code, out = self.branch_at(main, "feat/a", first[:12])
        self.expect("refuses a short SHA", code, 2, "sha-not-full-40-hex", out)
        code, out = self.branch_at(main, "feat/a", "0" * 40)
        self.expect("refuses a SHA that names no commit", code, 2, "no-such-commit", out)
        code, out = self.branch_at(main, "--force", first)
        self.expect("refuses a name that starts with a dash", code, 2, "bad-branch-name", out)
        for plus in ("+x", "+main"):
            code, out = self.branch_at(main, plus, first)
            self.expect(f"refuses the name '{plus}' (a leading + is a force refspec when pushed)",
                        code, 2, "bad-branch-name", out)
        for hex_name in ("abc1234", "a" * 40, second[:7]):
            code, out = self.branch_at(main, hex_name, first)
            self.expect(f"refuses the hash-like name '{hex_name}'", code, 2, "looks like a commit hash", out)
        code, out = self.branch_at(main, "v9", second)
        self.expect("refuses a name equal to an existing tag", code, 2, "existing tag", out)
        code, out = self.branch_at(main, "bad..name", first)
        self.expect("refuses an invalid branch name", code, 2, "bad-branch-name", out)
        for amb in ("@", "HEAD", "FETCH_HEAD", "origin/main", "origin", "tags/x", "heads/x",
                    "remotes/x", "refs/heads/zz"):
            code, out = self.branch_at(main, amb, first)
            self.expect(f"refuses the ambiguous name '{amb}' (exit {code})", code, 2, "status=refused", out)
        for pseudo in ("REBASE_HEAD", "AUTO_MERGE", "REVERT_HEAD", "BISECT_HEAD", "head", "Orig_Head",
                       "Rebase_Head", "Auto_Merge", "Revert_Head"):
            code, out = self.branch_at(main, pseudo, first)
            self.expect(f"refuses the pseudo-ref shaped name '{pseudo}' (exit {code})",
                        code, 2, "ambiguous-branch-name", out)
        code, out = self.branch_at(main, "feat/wt", second)
        self.expect("refuses to overwrite an existing branch", code, 2, "branch-exists", out)
        code, out = self.branch_at(self.wt, "feat/wt", second)
        self.expect("refuses an existing branch from a worktree too", code, 2, "branch-exists", out)
        refs_after = self.git(main, "for-each-ref", "--format=%(refname) %(objectname)")
        self.assertEqual(refs_after, refs_before, "the refusals changed no ref")

    def test_creation(self) -> None:
        self.build_fixture()
        main, first = self.main, self.first
        code, out = self.branch_at(self.wt, "feat/at-first", first)
        self.expect(f"creates a branch at the SHA (exit {code})", code, 0,
                    f"status=ok branch=feat/at-first sha={first}", out)
        self.assertEqual(self.git(main, "rev-parse", "refs/heads/feat/at-first").strip(), first,
                         "the new ref points at the SHA")
        self.assertEqual(self.git(main, "rev-parse", "--abbrev-ref", "HEAD").strip(), "main",
                         "the checkout of main did not move")
        self.assertEqual(self.git(self.wt, "rev-parse", "--abbrev-ref", "HEAD").strip(), "feat/wt",
                         "the worktree did not move")
        code, out = self.branch_at(main, "originals/x", first)
        self.assertEqual(code, 0, "a name that only starts with a remote name as a prefix is allowed")
        for ok in ("topic", "Zed", "topic_x", "v1"):
            code, out = self.branch_at(main, ok, first)
            self.expect(f"an ordinary name '{ok}' stays valid (exit {code}): {out.strip()}", code, 0,
                        f"status=ok branch={re.escape(ok)} ", out)

    def test_case_variant_refused_once_packed(self) -> None:
        # topic exists before the refusals. On NTFS a loose refs/heads/Topic would shadow the
        # packed refs/heads/topic and silently re-point it.
        self.build_fixture()
        main, second, first = self.main, self.second, self.first
        self.git(main, "branch", "topic", first)
        self.git(main, "pack-refs", "--all")
        topic_before = self.git(main, "rev-parse", "refs/heads/topic").strip()
        for variant in ("Topic", "tOPIC"):
            code, out = self.branch_at(main, variant, second)
            self.expect(f"refuses '{variant}' while topic exists (exit {code})", code, 2, "branch-exists", out)
        for df in ("topic/sub", "Topic/sub"):
            code, out = self.branch_at(main, df, second)
            self.expect(f"refuses '{df}' under the existing branch topic (exit {code})", code, 2,
                        "branch-exists", out)
        code, out = self.branch_at(main, "feat", second)
        self.expect(f"refuses 'feat', a path prefix of the existing feat/wt (exit {code})", code, 2,
                    "branch-exists", out)
        self.assertTrue(self.git(main, "rev-parse", "refs/heads/topic").strip() == topic_before
                        and self.git(main, "rev-parse", "topic").strip() == topic_before,
                        "topic did not move")
        self.assertEqual(self.git(main, "for-each-ref", "refs/heads/Topic", "refs/heads/TOPIC"), "",
                         "no case-variant ref was created")

    def test_failed_git_branch_reports_detail(self) -> None:
        # A failed `git branch` reports git's last line.
        self.build_fixture()
        main, first = self.main, self.first
        git_dir = Path(self.git(main, "rev-parse", "--absolute-git-dir").strip())
        (git_dir / "refs" / "heads").mkdir(parents=True, exist_ok=True)
        (git_dir / "refs" / "heads" / "lockme.lock").write_bytes(b"")
        code, out = self.branch_at(main, "lockme", first)
        self.expect(f"a failed git branch carries git's lock error as detail (exit {code}): {out.strip()}",
                    code, 1, r'status=failed reason=git-branch detail="[^"]*cannot lock', out)
        (git_dir / "refs" / "heads" / "lockme.lock").unlink()

    def test_name_that_already_resolves_is_refused(self) -> None:
        # git's rev lookup tries refs/<name> before refs/heads/<name>: with a stash present a branch
        # `stash` would never be what `stash` names.
        self.build_fixture()
        main, first, second = self.main, self.first, self.second
        (main / "a.txt").write_bytes(b"stashed edit\n")
        self.git(main, "stash", "-q")
        self.git(main, "update-ref", "refs/bisect/good-x", first)
        for shadowed in ("stash", "bisect/good-x"):
            code, out = self.branch_at(main, shadowed, second)
            self.expect(f"refuses '{shadowed}', which refs/{shadowed} already answers (exit {code}): {out.strip()}",
                        code, 2, r"ambiguous-branch-name: \S+ already resolves", out)
            self.assertEqual(self.git(main, "for-each-ref", f"refs/heads/{shadowed}"), "",
                             f"no branch {shadowed} was created")
        self.git(main, "stash", "drop", "-q")
        self.git(main, "update-ref", "-d", "refs/bisect/good-x")

    def test_git_dir_in_environment_does_not_move_branch(self) -> None:
        # GIT_DIR in the environment must not move the branch into another repository.
        self.build_fixture()
        main, first = self.main, self.first
        other = self.tmp / "other"
        self.git(self.tmp, "clone", "-q", main.as_posix(), other.as_posix())
        env = dict(self.env, GIT_DIR=(other / ".git").as_posix())
        code, out = self.run_sh("-C", main.as_posix(), "-Name", "env/probe", "-Sha", first, env=env)
        self.assertEqual(code, 0, f"creates the branch with GIT_DIR set (exit {code}): {out.strip()}")
        self.assertEqual(self.git_rc(main, "rev-parse", "--verify", "--quiet", "refs/heads/env/probe"), 0,
                         "the branch is in the repository named by -C")
        self.assertNotEqual(self.git_rc(other, "rev-parse", "--verify", "--quiet", "refs/heads/env/probe"), 0,
                            "the repository GIT_DIR named got no branch")


if __name__ == "__main__":
    unittest.main()
