"""Tests for rebase_pr.py (rebase-pr.sh), each in a throwaway repository with a bare origin.

    python -m unittest discover -s tools/build-lane -p "test_*.py"
"""

from __future__ import annotations

import contextlib
import io
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import rebase_pr  # noqa: E402
from test_lane_stats import find_bash  # noqa: E402

GEN_LINE = "Tests: <!-- gen:tests-total -->{n}<!-- /gen:tests-total --> {tail}\n"
TABLE = "<!-- gen:tests-by-crate -->\n\n| crate | tests |\n|---|---|\n| a | {n} |\n\n<!-- /gen:tests-by-crate -->\n"


def sh(*args: str, cwd: Path) -> str:
    p = subprocess.run(args, cwd=cwd, capture_output=True, text=True)
    if p.returncode != 0:
        raise AssertionError(f"{args} failed: {p.stderr}")
    return p.stdout.strip()


class Repo:
    """origin.git (bare), main/ (clone on main) and a feature worktree feat/."""

    def __init__(self, root: Path):
        self.root = root
        self.origin = root / "origin.git"
        self.main = root / "main"
        sh("git", "init", "-q", "--bare", "-b", "main", str(self.origin), cwd=root)
        sh("git", "clone", "-q", str(self.origin), str(self.main), cwd=root)
        for k, v in (("user.name", "t"), ("user.email", "t@example.invalid"), ("core.autocrlf", "false"),
                     ("commit.gpgsign", "false"), ("init.defaultBranch", "main")):
            sh("git", "config", k, v, cwd=self.main)
        sh("git", "checkout", "-q", "-b", "main", cwd=self.main)

    def commit(self, wt: Path, files: dict[str, str], msg: str = "c") -> str:
        for name, text in files.items():
            p = wt / name
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_bytes(text.encode())
        sh("git", "add", "-A", cwd=wt)
        sh("git", "commit", "-q", "-m", msg, cwd=wt)
        return sh("git", "rev-parse", "HEAD", cwd=wt)

    def push_main(self) -> None:
        sh("git", "push", "-q", "origin", "main", cwd=self.main)

    def feature(self, name: str = "feat") -> Path:
        wt = self.root / name
        sh("git", "worktree", "add", "-q", "-b", name, str(wt), "origin/main", cwd=self.main)
        return wt


class RebaseCase(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="test-rpr-"))
        self.repo = Repo(self.tmp)
        self.repo.commit(self.repo.main, {
            "README.md": "# Demo\n" + GEN_LINE.format(n=100, tail="total.") + "\n" + TABLE.format(n=100) + "\nEnd.\n",
            "src.rs": "fn a() {}\nfn b() {}\n",
            "Cargo.lock": "# lock\nversion = 3\n",
        })
        self.repo.push_main()
        self.wt = self.repo.feature()

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def run_tool(self, *extra: str, target: str = "feat") -> tuple[int, str]:
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            code = rebase_pr.main(["--repo", str(self.repo.main), *extra, target])
        return code, buf.getvalue()

    def main_moves(self, files: dict[str, str]) -> str:
        sha = self.repo.commit(self.repo.main, files, "main")
        self.repo.push_main()
        return sha

    def head(self, ref: str = "HEAD") -> str:
        return sh("git", "rev-parse", ref, cwd=self.wt)

    def assert_on_main(self):
        main = sh("git", "rev-parse", "origin/main", cwd=self.repo.main)
        self.assertEqual(subprocess.run(["git", "merge-base", "--is-ancestor", main, "HEAD"], cwd=self.wt).returncode, 0)

    def assert_no_scratch_left(self):
        listed = sh("git", "worktree", "list", "--porcelain", cwd=self.repo.main)
        self.assertNotIn("rebase-pr-", listed)


class CleanRebaseTests(RebaseCase):
    def test_clean_rebase_is_silent_and_moves_the_checked_out_branch(self):
        self.repo.commit(self.wt, {"new.rs": "fn n() {}\n"}, "feature")
        self.main_moves({"other.rs": "fn o() {}\n"})
        config = sh("git", "config", "--local", "--list", cwd=self.repo.main)
        code, out = self.run_tool()
        self.assertEqual((code, out), (0, ""))
        self.assertEqual(sh("git", "config", "--local", "--list", cwd=self.repo.main), config,
                         "the sparse scratch worktree must not change shared config")
        self.assert_on_main()
        self.assertTrue((self.wt / "other.rs").exists(), "the worktree follows the rebased branch")
        self.assertEqual(sh("git", "status", "--porcelain", cwd=self.wt), "")
        self.assert_no_scratch_left()

    def test_other_worktree_entries_are_not_pruned(self):
        """A prune deletes every entry whose path this git cannot resolve; on 2026-10-03
        live worktrees lost their entries that way. Only the scratch entry may go."""
        stale = self.repo.main / ".git" / "worktrees" / "elsewhere"
        stale.mkdir(parents=True)
        (stale / "gitdir").write_text("/no/such/place/.git\n")
        (stale / "commondir").write_text("../..\n")
        (stale / "HEAD").write_text("ref: refs/heads/main\n")
        self.repo.commit(self.wt, {"new.rs": "fn n() {}\n"}, "feature")
        self.main_moves({"other.rs": "fn o() {}\n"})
        self.assertEqual(self.run_tool(), (0, ""))
        self.assertTrue(stale.exists())
        entries = sorted(p.name for p in stale.parent.iterdir())
        self.assertEqual(entries, ["elsewhere", "feat"], "the scratch entry is removed, nothing else")

    def test_up_to_date_branch_is_left_alone(self):
        self.repo.commit(self.wt, {"new.rs": "x\n"})
        before = self.head()
        code, out = self.run_tool("-v")
        self.assertEqual(code, 0)
        self.assertIn("status=up-to-date", out)
        self.assertEqual(self.head(), before)

    def test_branch_without_a_worktree_is_moved_by_ref(self):
        self.repo.commit(self.wt, {"new.rs": "x\n"})
        sh("git", "worktree", "remove", str(self.wt), cwd=self.repo.main)
        self.main_moves({"other.rs": "y\n"})
        code, _ = self.run_tool()
        self.assertEqual(code, 0)
        main = sh("git", "rev-parse", "origin/main", cwd=self.repo.main)
        self.assertEqual(sh("git", "merge-base", main, "feat", cwd=self.repo.main), main)

    def test_push_uses_a_lease_and_refuses_a_diverged_origin(self):
        self.repo.commit(self.wt, {"new.rs": "x\n"})
        sh("git", "push", "-q", "-u", "origin", "feat", cwd=self.wt)
        self.main_moves({"other.rs": "y\n"})
        code, out = self.run_tool("--push", "--json", "-v")
        self.assertEqual(code, 0, out)
        self.assertTrue(json.loads(out)["pushed"])
        self.assertEqual(sh("git", "ls-remote", str(self.repo.origin), "refs/heads/feat", cwd=self.tmp).split()[0], self.head())

        # Someone else pushes to the branch: a --push run must not overwrite it.
        other = self.tmp / "other"
        sh("git", "clone", "-q", "-b", "feat", str(self.repo.origin), str(other), cwd=self.tmp)
        sh("git", "-c", "user.name=o", "-c", "user.email=o@example.invalid", "commit", "-q", "--allow-empty", "-m", "theirs", cwd=other)
        sh("git", "push", "-q", "origin", "feat", cwd=other)
        self.main_moves({"third.rs": "z\n"})
        before = self.head()
        code, out = self.run_tool("--push")
        self.assertEqual(code, 2, out)
        self.assertIn("status=diverged", out)
        self.assertEqual(self.head(), before)


class MechanicalConflictTests(RebaseCase):
    def test_generated_block_conflict_takes_mains_side(self):
        readme = (self.wt / "README.md").read_text()
        self.repo.commit(self.wt, {"README.md": readme.replace("100<!-- /gen:tests-total --> total.",
                                                               "100<!-- /gen:tests-total --> in all.")})
        self.main_moves({"README.md": readme.replace("100", "120")})
        code, out = self.run_tool()
        self.assertEqual(code, 0, out)
        self.assertIn("mechanical=README.md", out)
        text = (self.wt / "README.md").read_text()
        self.assertIn("-->120<!-- /gen:tests-total --> in all.", text, "main's number, the branch's prose")
        self.assertIn("| a | 120 |", text)
        self.assertNotIn("<<<<<<<", text)
        self.assert_on_main()

    def test_crate_graph_block_conflict_takes_mains_side(self):
        base = "# Crates\n<!-- crate-graph:begin -->\ngraph v1\n<!-- crate-graph:end -->\nRows.\n"
        self.main_moves({"crates.md": base})
        sh("git", "fetch", "-q", "origin", cwd=self.wt)
        sh("git", "reset", "-q", "--hard", "origin/main", cwd=self.wt)
        self.repo.commit(self.wt, {"crates.md": base.replace("graph v1", "graph v1 plus my crate").replace("Rows.", "Rows and mine.")})
        self.main_moves({"crates.md": base.replace("graph v1", "graph v2")})
        code, out = self.run_tool()
        self.assertEqual(code, 0, out)
        text = (self.wt / "crates.md").read_text()
        self.assertIn("graph v2\n", text)
        self.assertIn("Rows and mine.", text)

    def test_lockfile_conflict_is_regenerated_in_a_full_checkout(self):
        self.repo.commit(self.wt, {"Cargo.lock": "# lock\nversion = 3\nbranch-dep\n"})
        self.main_moves({"Cargo.lock": "# lock\nversion = 3\nmain-dep\n"})
        cmd = f'"{sys.executable}" -c "open(\'Cargo.lock\',\'a\').write(\'regenerated\\n\')"'
        os.environ["REBASE_PR_LOCKFILE_CMD"] = cmd
        try:
            code, out = self.run_tool()
        finally:
            del os.environ["REBASE_PR_LOCKFILE_CMD"]
        self.assertEqual(code, 0, out)
        self.assertIn("mechanical=Cargo.lock", out)
        self.assertEqual((self.wt / "Cargo.lock").read_text(), "# lock\nversion = 3\nmain-dep\nregenerated\n")


class UnionMergeTests(RebaseCase):
    """docs/readme.md is `merge=union` in this repo and holds generated blocks."""

    def setUp(self):
        super().setUp()
        self.main_moves({".gitattributes": "README.md merge=union\n"})
        sh("git", "fetch", "-q", "origin", cwd=self.wt)
        sh("git", "reset", "-q", "--hard", "origin/main", cwd=self.wt)
        self.readme = (self.wt / "README.md").read_text()

    def test_generated_line_is_not_duplicated_by_union(self):
        self.repo.commit(self.wt, {"README.md": self.readme.replace(" total.", " in all.")})
        self.main_moves({"README.md": self.readme.replace("100", "120")})
        code, out = self.run_tool()
        self.assertEqual(code, 0, out)
        text = (self.wt / "README.md").read_text()
        self.assertEqual(text.count("<!-- gen:tests-total -->"), 1, text)
        self.assertIn("-->120<!-- /gen:tests-total --> in all.", text)

    def test_union_still_keeps_both_appended_rows(self):
        self.repo.commit(self.wt, {"README.md": self.readme + "branch row\n"})
        self.main_moves({"README.md": self.readme.replace("100", "120") + "main row\n"})
        code, out = self.run_tool()
        self.assertEqual(code, 0, out)
        text = (self.wt / "README.md").read_text()
        self.assertIn("main row\n", text)
        self.assertIn("branch row\n", text)
        self.assertIn("| a | 120 |", text)


class SemanticConflictTests(RebaseCase):
    def snapshot(self):
        files = {p.relative_to(self.wt).as_posix(): (p.read_bytes(), p.stat().st_mtime_ns)
                 for p in self.wt.rglob("*") if p.is_file() and ".git" not in p.parts}
        return self.head(), sh("git", "status", "--porcelain", cwd=self.wt), files

    def test_semantic_conflict_aborts_and_leaves_the_branch_untouched(self):
        readme = (self.wt / "README.md").read_text()
        self.repo.commit(self.wt, {"src.rs": "fn a() { branch }\nfn b() {}\n",
                                   "README.md": readme.replace(" total.", " in all.")}, "feature")
        self.main_moves({"src.rs": "fn a() { main }\nfn b() {}\n", "README.md": readme.replace(">100<", ">120<")})
        before = self.snapshot()
        code, out = self.run_tool()
        self.assertEqual(code, 1, out)
        summary = dict(line.split("=", 1) for line in out.strip().splitlines())
        self.assertEqual(summary["status"], "conflict")
        self.assertEqual(summary["semantic"], "src.rs")
        self.assertEqual(summary["mechanical"], "README.md", "the generated block was resolvable")
        self.assertIn("src.rs", summary["preview"].split(","))
        self.assertEqual(summary["stopped_at"], before[0])
        self.assertEqual(self.snapshot(), before, "same HEAD, status, file contents and mtimes")
        self.assertFalse(os.path.exists(sh("git", "rev-parse", "--path-format=absolute", "--git-path", "rebase-merge", cwd=self.wt)))
        self.assert_no_scratch_left()

    def test_prose_conflict_next_to_a_generated_block_is_semantic(self):
        readme = (self.wt / "README.md").read_text()
        self.repo.commit(self.wt, {"README.md": readme.replace(" total.", " in all.")})
        self.main_moves({"README.md": readme.replace(" total.", " altogether.")})
        code, out = self.run_tool("--json")
        self.assertEqual(code, 1)
        self.assertEqual(json.loads(out)["semantic"], ["README.md"])

    def test_dirty_worktree_is_refused(self):
        self.repo.commit(self.wt, {"new.rs": "x\n"})
        self.main_moves({"other.rs": "y\n"})
        (self.wt / "src.rs").write_text("edited\n")
        before = self.head()
        code, out = self.run_tool()
        self.assertEqual(code, 2)
        self.assertIn("status=dirty", out)
        self.assertEqual(self.head(), before)


class ResolveGeneratedTests(unittest.TestCase):
    def test_block_only_the_branch_adds_keeps_its_body(self):
        base = b"a\nb\n"
        ours = b"a\nb\nmain line\n"
        theirs = b"a\n<!-- gen:new -->7<!-- /gen:new -->\nb\n"
        self.assertEqual(rebase_pr.resolve_generated(base, ours, theirs),
                         b"a\n<!-- gen:new -->7<!-- /gen:new -->\nb\nmain line\n")

    def test_duplicated_block_is_not_mechanical(self):
        block = b"<!-- gen:x -->1<!-- /gen:x -->\n"
        base = b"top\n" + block + b"bottom\n"
        ours = b"top\n" + block.replace(b"1", b"2") + b"bottom\n"
        theirs = b"top\n" + block.replace(b"1", b"3") + b"bottom\n" + block
        self.assertIsNone(rebase_pr.resolve_generated(base, ours, theirs))

    def test_crlf_is_kept_byte_for_byte(self):
        base = b"x\r\n<!-- gen:n -->1<!-- /gen:n --> tail\r\ny\r\n"
        ours = base.replace(b">1<", b">2<")
        theirs = base.replace(b" tail", b" other")
        self.assertEqual(rebase_pr.resolve_generated(base, ours, theirs),
                         b"x\r\n<!-- gen:n -->2<!-- /gen:n --> other\r\ny\r\n")


@unittest.skipUnless(find_bash(), "no bash")
class WrapperTests(RebaseCase):
    def test_wrapper_runs_the_script(self):
        self.repo.commit(self.wt, {"new.rs": "x\n"})
        env = dict(os.environ, PYTHON=sys.executable)
        p = subprocess.run([find_bash(), str(HERE / "rebase-pr.sh"), "-v", "feat"], cwd=self.repo.main,
                           capture_output=True, text=True, env=env)
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertIn("status=up-to-date", p.stdout)


if __name__ == "__main__":
    unittest.main()
