"""Tests for mk-worktree.sh in a throwaway repository with a bare origin; nothing here
touches a real worktree.

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
from test_rebase_pr import Repo  # noqa: E402

SCRIPT = HERE / "mk-worktree.sh"


@unittest.skipUnless(find_bash(), "no bash")
class MkWorktreeShTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="test-mkwt-"))
        self.repo = Repo(self.tmp)
        (self.repo.main / "external").mkdir()
        (self.repo.main / "external" / "keep.txt").write_bytes(b"real external\n")
        self.repo.commit(self.repo.main, {".gitignore": ".claude/worktrees/\nexternal\n", "README.md": "# Demo\n"})
        self.repo.push_main()

    def tearDown(self):
        # Remove the link first, so rmtree never follows it into external/.
        ext = self.repo.main / ".claude" / "worktrees" / "one" / "external"
        if os.name == "nt":
            if ext.exists():
                subprocess.run(["cmd", "/c", "rmdir", str(ext)], capture_output=True)
        elif os.path.islink(ext):
            os.unlink(ext)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def run_script(self, *argv: str) -> subprocess.CompletedProcess:
        env = dict(os.environ)
        env.pop("CIMMERIA_TARGET_ROOT", None)
        return subprocess.run([find_bash(), SCRIPT.as_posix(), *argv], cwd=self.repo.main,
                              capture_output=True, text=True, env=env)

    def wt_external(self) -> Path:
        return self.repo.main / ".claude" / "worktrees" / "one" / "external"

    def test_links_external_and_the_worktree_builds_from_it(self):
        p = self.run_script("probe/one", "one")
        self.assertEqual(p.returncode, 0, p.stderr)
        keep = self.wt_external() / "keep.txt"
        self.assertEqual(keep.read_bytes(), b"real external\n")
        if os.name == "nt":
            self.assertIn("(external/ junctioned)", p.stdout)
        else:
            self.assertIn("(external/ symlinked)", p.stdout)
            self.assertTrue(os.path.islink(self.wt_external()))

    @unittest.skipIf(os.name == "nt", "the Linux branch; CI runs it")
    def test_missing_main_external_warns_but_links(self):
        shutil.rmtree(self.repo.main / "external")
        p = self.run_script("probe/one", "one")
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertIn("warning:", p.stderr)
        self.assertTrue(os.path.islink(self.wt_external()))

    def test_existing_worktree_is_left_alone(self):
        first = self.run_script("probe/one", "one")
        self.assertEqual(first.returncode, 0, first.stderr)
        second = self.run_script("probe/one", "one")
        self.assertEqual(second.returncode, 0, second.stderr)
        self.assertIn("exists:", second.stdout)


if __name__ == "__main__":
    unittest.main()
