"""Tests for the resume-card and orchestrator-guard hooks in .claude/hooks/."""

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HOOKS = Path(__file__).resolve().parents[2] / ".claude/hooks"
RESUME_CARD = HOOKS / "resume_card.py"
ORCHESTRATOR_GUARD = HOOKS / "orchestrator_guard.py"


def run_hook(script, project, stdin_text):
    env = dict(os.environ, CLAUDE_PROJECT_DIR=str(project))
    return subprocess.run(
        [sys.executable, str(script)],
        input=stdin_text, capture_output=True, text=True, check=False,
        cwd=str(project), env=env,
    )


def guard(project, tool, tool_input):
    payload = json.dumps({"tool_name": tool, "tool_input": tool_input})
    return run_hook(ORCHESTRATOR_GUARD, project, payload)


def write_lines(path, count):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("".join(f"row {i:03d}\n" for i in range(1, count + 1)), encoding="utf-8")


class ResumeCardTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.project = Path(self.tmp.name)

    def tearDown(self):
        self.tmp.cleanup()

    def point_at(self, name):
        state = self.project / ".claude" / "state"
        state.mkdir(parents=True, exist_ok=True)
        (state / "active-campaign").write_text(name + "\n", encoding="utf-8")

    def test_prints_nothing_without_pointer(self):
        write_lines(self.project / "docs/analysis/demo/RESUME.md", 5)
        result = run_hook(RESUME_CARD, self.project, "{}")
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "")

    def test_prints_card_with_pointer(self):
        self.point_at("demo")
        write_lines(self.project / "docs/analysis/demo/RESUME.md", 5)
        result = run_hook(RESUME_CARD, self.project, "{}")
        self.assertEqual(result.returncode, 0)
        output = json.loads(result.stdout)["hookSpecificOutput"]
        self.assertEqual(output["hookEventName"], "SessionStart")
        self.assertTrue(output["additionalContext"].startswith(
            "Context was compacted. Active campaign: demo. Read docs/analysis/demo/RESUME.md "
            "and the ledger README before acting."))
        self.assertIn("row 005", output["additionalContext"])

    def test_truncates_resume_at_40_lines(self):
        self.point_at("demo")
        write_lines(self.project / "docs/analysis/demo/RESUME.md", 100)
        result = run_hook(RESUME_CARD, self.project, "{}")
        card = json.loads(result.stdout)["hookSpecificOutput"]["additionalContext"]
        self.assertIn("row 040", card)
        self.assertNotIn("row 041", card)

    def test_exits_zero_on_malformed_json(self):
        self.point_at("demo")
        write_lines(self.project / "docs/analysis/demo/RESUME.md", 3)
        result = run_hook(RESUME_CARD, self.project, "{not json")
        self.assertEqual(result.returncode, 0)

    def test_rejects_campaign_name_that_is_a_path(self):
        self.point_at("../demo")
        write_lines(self.project / "docs/analysis/demo/RESUME.md", 3)
        result = run_hook(RESUME_CARD, self.project, "{}")
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "")


class OrchestratorGuardTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.project = Path(self.tmp.name)

    def tearDown(self):
        self.tmp.cleanup()

    def test_allows_ledger_edit(self):
        result = guard(self.project, "Edit", {"file_path": "docs/analysis/x/README.md"})
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_allows_state_write_by_absolute_path(self):
        target = str(self.project / ".claude" / "state" / "active-campaign")
        result = guard(self.project, "Write", {"file_path": target})
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_denies_code_edit_and_names_coder(self):
        result = guard(self.project, "Edit", {"file_path": "crates/foo.rs"})
        self.assertEqual(result.returncode, 2)
        self.assertIn("packet-coder", result.stderr)

    def test_denies_write_that_escapes_docs_analysis(self):
        result = guard(self.project, "Write", {"file_path": "docs/analysis/../../crates/foo.rs"})
        self.assertEqual(result.returncode, 2)

    def test_denies_500_line_read(self):
        write_lines(self.project / "crates/big.rs", 500)
        result = guard(self.project, "Read", {"file_path": "crates/big.rs"})
        self.assertEqual(result.returncode, 2)
        self.assertIn("Explore", result.stderr)

    def test_allows_500_line_read_with_limit(self):
        write_lines(self.project / "crates/big.rs", 500)
        result = guard(self.project, "Read", {"file_path": "crates/big.rs", "limit": 100})
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_allows_short_read_and_unparseable_input(self):
        write_lines(self.project / "crates/small.rs", 10)
        result = guard(self.project, "Read", {"file_path": "crates/small.rs"})
        self.assertEqual(result.returncode, 0)
        result = run_hook(ORCHESTRATOR_GUARD, self.project, "{broken")
        self.assertEqual(result.returncode, 0)


if __name__ == "__main__":
    unittest.main()
