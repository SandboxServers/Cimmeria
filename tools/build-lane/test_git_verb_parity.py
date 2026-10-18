"""Parity between the git-verb .sh twins and their .ps1 specs (AH-11p, D-AH37).

CommonBlockTests (runs everywhere) checks that the C11 block is non-empty and byte-identical in the five
.sh twins. ParityTests runs each verb's .ps1 and .sh on two identical fresh fixtures and compares the exit
code, the first status= token and the first reason= token. It needs Windows, pwsh and Git Bash.
"""

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

ROOT = HERE.parent.parent
VERBS = ["commit-paths", "merge-noff", "branch-at", "push-branch", "rebase-onto"]
BEGIN = "# --- git-verb common (C11) ---"
END = "# --- end common ---"
STATUS = re.compile(r"\bstatus=(\S+)")
REASON = re.compile(r"\breason=([^:\s]*)")


def git(cwd, *args):
    """Run git in a fixture with a fixed identity; returns its stdout, stripped."""
    cmd = ["git", "-c", "user.name=parity", "-c", "user.email=parity@example.invalid",
           "-c", "commit.gpgsign=false", *args]
    return subprocess.run(cmd, cwd=cwd, check=True, capture_output=True, text=True).stdout.strip()


def write(path, text):
    path.write_text(text, encoding="utf-8")


def commit(repo, name, message):
    write(repo / name, message + "\n")
    git(repo, "add", name)
    git(repo, "commit", "-q", "-m", message)


def base(root, branch="main"):
    """A primary checkout on trunk (the main checkout, which the guards refuse), with main and feat at c2,
    `older` at c1 and `topic` one commit past `older`. The case runs in a linked worktree on `branch`,
    because the guards refuse the main checkout."""
    primary = root / "primary"
    primary.mkdir()
    git(primary, "init", "-q", "-b", "trunk")
    commit(primary, "a.txt", "c1")
    git(primary, "branch", "older")
    commit(primary, "b.txt", "c2")
    git(primary, "branch", "main")
    git(primary, "branch", "feat")
    git(primary, "checkout", "-q", "-b", "topic", "older")
    commit(primary, "t.txt", "t1")
    git(primary, "checkout", "-q", "trunk")
    wt = root / "wt"
    git(primary, "worktree", "add", "-q", str(wt), branch)
    return wt


def new_file(root):
    wt = base(root)
    write(wt / "new.txt", "n\n")
    return wt


def dirty(root):
    wt = base(root)
    write(wt / "a.txt", "dirty\n")
    return wt


def on_topic(root):
    return base(root, "topic")


def origin(root):
    """base on feat, with a bare origin that holds main (feat is never pushed)."""
    wt = base(root, "feat")
    git(root, "init", "-q", "--bare", "origin.git")
    git(wt, "remote", "add", "origin", str(root / "origin.git"))
    git(wt, "push", "-q", "origin", "main")
    return wt


FIXTURES = {"base": base, "new": new_file, "dirty": dirty, "on_topic": on_topic, "origin": origin}


def ctx(repo):
    return {"C": str(repo), "head": git(repo, "rev-parse", "HEAD"), "main": git(repo, "rev-parse", "main")}


# (verb, case, fixture, argv builder). Each side gets its own fixture, so the shas differ between sides.
CASES = [
    ("commit-paths", "ok", "new", lambda c: ["-C", c["C"], "-Message", "add new", "-Paths", "new.txt"]),
    ("commit-paths", "rooted path", "new", lambda c: ["-C", c["C"], "-Message", "m", "-Paths", "/x"]),
    ("commit-paths", "dot path", "new", lambda c: ["-C", c["C"], "-Message", "m", "-Paths", "."]),
    ("commit-paths", "blank message", "new", lambda c: ["-C", c["C"], "-Message", " ", "-Paths", "new.txt"]),
    ("commit-paths", "two paths", "new", lambda c: ["-C", c["C"], "-Message", "m", "-Paths", "a.txt", "b.txt"]),
    ("commit-paths", "no message", "new", lambda c: ["-C", c["C"], "-Paths", "new.txt"]),
    ("commit-paths", "expect branch wrong case", "new",
     lambda c: ["-C", c["C"], "-Message", "m", "-Paths", "new.txt", "-ExpectBranch", "Main"]),
    ("commit-paths", "bogus option", "new",
     lambda c: ["-C", c["C"], "-Message", "m", "-Paths", "new.txt", "-Bogus", "x"]),
    # A value token that starts with "-" is taken as the value on both sides: pwsh -File binds "-x" to
    # -Message, and the C11 loop takes the next token verbatim to match it.
    ("commit-paths", "message starts with dash", "new", lambda c: ["-C", c["C"], "-Message", "-x", "-Paths", "new.txt"]),
    ("commit-paths", "message attached form", "new", lambda c: ["-C", c["C"], "-Message:-x", "-Paths", "new.txt"]),
    ("merge-noff", "ok", "base", lambda c: ["-C", c["C"], "-Branch", "topic"]),
    ("merge-noff", "up to date", "base", lambda c: ["-C", c["C"], "-Branch", "older"]),
    ("merge-noff", "no such local branch", "base", lambda c: ["-C", c["C"], "-Branch", "no-such-local-branch"]),
    ("merge-noff", "same branch", "base", lambda c: ["-C", c["C"], "-Branch", "main"]),
    ("merge-noff", "dirty tree", "dirty", lambda c: ["-C", c["C"], "-Branch", "topic"]),
    ("branch-at", "ok", "base", lambda c: ["-C", c["C"], "-Name", "newb", "-Sha", c["main"]]),
    ("branch-at", "short sha", "base", lambda c: ["-C", c["C"], "-Name", "newb", "-Sha", c["main"][:7]]),
    ("branch-at", "HEAD", "base", lambda c: ["-C", c["C"], "-Name", "newb", "-Sha", "HEAD"]),
    ("branch-at", "existing topic in another case", "base",
     lambda c: ["-C", c["C"], "-Name", "Topic", "-Sha", c["main"]]),
    ("branch-at", "refs/heads prefix", "base", lambda c: ["-C", c["C"], "-Name", "refs/heads/x", "-Sha", c["main"]]),
    ("push-branch", "ok", "origin", lambda c: ["-C", c["C"], "-Branch", "feat"]),
    ("push-branch", "main", "origin", lambda c: ["-C", c["C"], "-Branch", "main"]),
    ("push-branch", "force alone", "origin", lambda c: ["-C", c["C"], "-Branch", "feat", "-Force"]),
    ("push-branch", "expect remote sha without force", "origin",
     lambda c: ["-C", c["C"], "-Branch", "feat", "-ExpectRemoteSha", c["main"]]),
    ("push-branch", "plus prefix", "origin", lambda c: ["-C", c["C"], "-Branch", "+x"]),
    ("push-branch", "branch not current", "origin", lambda c: ["-C", c["C"], "-Branch", "topic"]),
    ("rebase-onto", "ok", "on_topic", lambda c: ["-C", c["C"], "-Onto", c["main"]]),
    ("rebase-onto", "short sha", "on_topic", lambda c: ["-C", c["C"], "-Onto", c["main"][:7]]),
    ("rebase-onto", "upstream equal to HEAD", "on_topic",
     lambda c: ["-C", c["C"], "-Onto", c["main"], "-Upstream", c["head"]]),
    ("rebase-onto", "dirty tree", "dirty", lambda c: ["-C", c["C"], "-Onto", c["main"]]),
]


class CommonBlockTests(unittest.TestCase):
    def test_c11_block_is_present_and_identical_in_all_five_twins(self):
        blocks = {}
        for verb in VERBS:
            text = (HERE / f"{verb}.sh").read_bytes().decode("utf-8")
            begin = text.find(BEGIN)
            end = text.find(END)
            self.assertNotEqual(begin, -1, f"{verb}.sh has no C11 begin marker")
            self.assertGreater(end, begin, f"{verb}.sh has no C11 end marker")
            blocks[verb] = text[begin:end + len(END)]
            self.assertTrue(blocks[verb].strip(), f"{verb}.sh has an empty C11 block")
        for verb in VERBS:
            self.assertEqual(blocks[verb], blocks["commit-paths"], f"{verb}.sh C11 block differs from commit-paths.sh")


def signature(rc, out):
    """(exit code, first status= token, first reason= token up to a ':' or space)."""
    m = STATUS.search(out)
    if not m:
        return (rc, None, None)
    r = REASON.search(out)
    return (rc, m.group(1), r.group(1) if r else None)


@unittest.skipUnless(os.name == "nt" and shutil.which("pwsh") and find_bash(),
                     "parity needs Windows, pwsh and Git Bash")
class ParityTests(unittest.TestCase):
    def run_side(self, verb, fixture, argfn, impl, bash):
        root = Path(tempfile.mkdtemp(prefix="parity-"))
        try:
            args = argfn(ctx(FIXTURES[fixture](root)))
            if impl == "ps1":
                cmd = ["pwsh", "-NoProfile", "-NonInteractive", "-File", str(HERE / f"{verb}.ps1"), *args]
            else:
                cmd = [bash, (HERE / f"{verb}.sh").as_posix(), *args]
            p = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=120)
            return signature(p.returncode, p.stdout)
        finally:
            shutil.rmtree(root, ignore_errors=True)

    def test_sh_twins_agree_with_ps1(self):
        bash = find_bash()
        for verb, case, fixture, argfn in CASES:
            with self.subTest(verb=verb, case=case):
                ps1 = self.run_side(verb, fixture, argfn, "ps1", bash)
                sh = self.run_side(verb, fixture, argfn, "sh", bash)
                self.assertEqual(ps1[0], sh[0], f"exit codes: ps1={ps1} sh={sh}")
                if ps1[1] or sh[1]:
                    self.assertEqual(ps1[1], sh[1], f"status: ps1={ps1} sh={sh}")
                    self.assertEqual(ps1[2], sh[2], f"reason: ps1={ps1} sh={sh}")
                else:
                    self.assertEqual(ps1[0], 1, "no status= line on either side, so both exit 1")


if __name__ == "__main__":
    unittest.main()
