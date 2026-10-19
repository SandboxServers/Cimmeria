"""Tests for the agent contract that lets ops run the .sh twins on Linux and WSL hosts."""

import re
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
OPS_MD = REPO / ".claude/agents/ops.md"
BUILD_LANE = REPO / "tools/build-lane"
SCRIPT_RE = re.compile(r"tools/build-lane/([a-z][a-z-]*)\.ps1")
REQUIRED = {
    "mk-worktree",
    "ship",
    "rm-worktree",
    "commit-paths",
    "merge-noff",
    "branch-at",
    "push-branch",
    "rebase-onto",
}


class OpsTwinsTest(unittest.TestCase):
    """ops.md names PowerShell scripts; each one must have a bash twin that ops is told to run."""

    def setUp(self):
        self.text = OPS_MD.read_text(encoding="utf-8")

    def test_every_script_has_its_bash_twin(self):
        names = set(SCRIPT_RE.findall(self.text))
        self.assertTrue(REQUIRED <= names, f"ops.md misses scripts: {sorted(REQUIRED - names)}")
        for name in sorted(names):
            with self.subTest(name=name):
                self.assertTrue((BUILD_LANE / f"{name}.sh").is_file(), f"{name}.sh is missing")
                self.assertIn(f"bash tools/build-lane/{name}.sh", self.text)

    def test_frontmatter_grants_bash_and_the_body_keys_on_platform(self):
        parts = self.text.split("---", 2)
        self.assertGreaterEqual(len(parts), 3, "ops.md has no frontmatter block")
        tools_lines = [ln for ln in parts[1].splitlines() if ln.startswith("tools:")]
        self.assertEqual(len(tools_lines), 1, "frontmatter needs one tools: line")
        granted = [item.strip() for item in tools_lines[0].split(":", 1)[1].split(",")]
        self.assertIn("Bash", granted)
        self.assertIn("Platform:", self.text)
        self.assertIn("win32", self.text)

    def test_other_platforms_are_refused_not_bash(self):
        self.assertIn("On `linux`", self.text)
        self.assertIn("(Windows, Linux and WSL only)", self.text)
        self.assertNotIn("(`linux`, `darwin`)", self.text)


if __name__ == "__main__":
    unittest.main()
