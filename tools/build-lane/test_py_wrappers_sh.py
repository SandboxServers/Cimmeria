"""Tests for the Python lookup in ship.sh and rebase-pr.sh: each script runs under real bash
with fake interpreters first on PATH, so the order and the refusal are what is tested.

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


@unittest.skipUnless(find_bash(), "no bash")
class PyWrapperTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="py-wrappers-"))
        self.bin = self.tmp / "bin"
        self.bin.mkdir()
        self.real = Path(sys.executable).as_posix()

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def shim(self, name: str, body: str) -> None:
        path = self.bin / name
        path.write_text(f"#!/bin/sh\n{body}\n", encoding="utf-8", newline="\n")
        path.chmod(0o755)

    def run_script(self, script: str, *argv: str, env_extra: dict[str, str] | None = None):
        env = dict(os.environ, PATH=str(self.bin) + os.pathsep + os.environ["PATH"])
        env.pop("PYTHON", None)
        env.update(env_extra or {})
        return subprocess.run([find_bash(), (HERE / script).as_posix(), *argv],
                              capture_output=True, text=True, env=env, cwd=self.tmp)

    def shim_python3_and_python(self) -> None:
        self.shim("python3", f'exec "{self.real}" "$@"')
        self.shim("python", "echo wrong-python >&2\nexit 97")

    def test_ship_prefers_python3_over_python(self):
        self.shim_python3_and_python()
        p = self.run_script("ship.sh", "pr", "-C", str(self.tmp / "gone"), "-m", "x")
        self.assertEqual(p.returncode, 2, p.stderr)
        self.assertIn("status=refused", p.stdout)
        self.assertNotIn("wrong-python", p.stderr)

    def test_rebase_pr_prefers_python3_over_python(self):
        self.shim_python3_and_python()
        p = self.run_script("rebase-pr.sh", "--help")
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertNotIn("wrong-python", p.stderr)

    def test_python_env_wins(self):
        self.shim("mypy3", f'echo picked >&2\nexec "{self.real}" "$@"')
        p = self.run_script("ship.sh", "pr", "-C", str(self.tmp / "gone"), "-m", "x",
                            env_extra={"PYTHON": (self.bin / "mypy3").as_posix()})
        self.assertIn("picked", p.stderr)

    def test_no_working_python_refuses(self):
        for name in ("python3", "python", "py"):
            self.shim(name, "exit 1")
        p = self.run_script("ship.sh", "pr", "-C", "x", "-m", "x")
        self.assertEqual(p.returncode, 2, p.stderr)
        self.assertIn("no Python", p.stdout)

    def test_no_working_python_refuses_rebase_pr(self):
        for name in ("python3", "python", "py"):
            self.shim(name, "exit 1")
        p = self.run_script("rebase-pr.sh", "feat")
        self.assertEqual(p.returncode, 2, p.stderr)
        self.assertIn("no Python", p.stdout)


if __name__ == "__main__":
    unittest.main()
