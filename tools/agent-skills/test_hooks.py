"""Tests for the resume-card and orchestrator-guard hooks in .claude/hooks/."""

import importlib.util
import io
import json
import os
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

REPO = Path(__file__).resolve().parents[2]
HOOKS = REPO / ".claude/hooks"
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

    def test_exits_zero_when_building_the_card_raises(self):
        # The stdin payload is never parsed, so a bad payload cannot reach the
        # catch-all; force the failure inside main() instead.
        spec = importlib.util.spec_from_file_location("resume_card_under_test", RESUME_CARD)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with mock.patch.object(module, "build_card", side_effect=RuntimeError("boom")), \
                mock.patch.object(sys, "stdin", io.StringIO("{}")), \
                mock.patch.object(sys, "stdout", new_callable=io.StringIO) as out:
            self.assertEqual(module.main(), 0)
        self.assertEqual(out.getvalue(), "")

    def test_pointer_with_bom_still_names_the_campaign(self):
        state = self.project / ".claude" / "state"
        state.mkdir(parents=True)
        (state / "active-campaign").write_bytes(b"\xef\xbb\xbfdemo\r\n")
        write_lines(self.project / "docs/analysis/demo/RESUME.md", 3)
        result = run_hook(RESUME_CARD, self.project, "{}")
        card = json.loads(result.stdout)["hookSpecificOutput"]["additionalContext"]
        self.assertIn("Active campaign: demo.", card)

    def test_rejects_campaign_name_that_is_a_path(self):
        # The RESUME.md sits where "../demo" and "..\demo" would resolve, so
        # only the name check keeps the card empty.
        write_lines(self.project / "docs/demo/RESUME.md", 3)
        for name in ("../demo", "..\\demo"):
            self.point_at(name)
            result = run_hook(RESUME_CARD, self.project, "{}")
            self.assertEqual(result.returncode, 0)
            self.assertEqual(result.stdout, "", name)


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

    def test_read_line_cap_boundary(self):
        write_lines(self.project / "crates/edge.rs", 399)
        self.assertEqual(guard(self.project, "Read", {"file_path": "crates/edge.rs"}).returncode, 0)
        write_lines(self.project / "crates/edge.rs", 400)
        self.assertEqual(guard(self.project, "Read", {"file_path": "crates/edge.rs"}).returncode, 2)

    def test_limit_over_200_does_not_unlock_a_large_read(self):
        write_lines(self.project / "crates/big.rs", 500)
        result = guard(self.project, "Read", {"file_path": "crates/big.rs", "limit": 201})
        self.assertEqual(result.returncode, 2)
        result = guard(self.project, "Read", {"file_path": "crates/big.rs", "limit": 200})
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_allows_large_read_inside_docs_analysis(self):
        write_lines(self.project / "docs/analysis/x/big.md", 500)
        result = guard(self.project, "Read", {"file_path": "docs/analysis/x/big.md"})
        self.assertEqual(result.returncode, 0, result.stderr)

    @unittest.skipUnless(os.name == "nt", "Windows path spelling")
    def test_windows_backslash_and_case_variants(self):
        ledger = self.project / "docs" / "analysis" / "x" / "README.md"
        self.assertEqual(guard(self.project, "Edit", {"file_path": str(ledger).upper()}).returncode, 0)
        relative = "docs\\analysis\\x\\README.md"
        self.assertEqual(guard(self.project, "Edit", {"file_path": relative}).returncode, 0)
        escape = "docs\\analysis\\..\\..\\crates\\foo.rs"
        self.assertEqual(guard(self.project, "Edit", {"file_path": escape}).returncode, 2)
        outside = str(self.project / "crates" / "foo.rs").replace("\\", "/")
        self.assertEqual(guard(self.project, "Edit", {"file_path": outside}).returncode, 2)

    def test_allows_short_read_and_unparseable_input(self):
        write_lines(self.project / "crates/small.rs", 10)
        result = guard(self.project, "Read", {"file_path": "crates/small.rs"})
        self.assertEqual(result.returncode, 0)
        result = run_hook(ORCHESTRATOR_GUARD, self.project, "{broken")
        self.assertEqual(result.returncode, 0)


class WiringTest(unittest.TestCase):
    """The hooks only matter if settings.json and the persona call them."""

    def test_settings_registers_resume_card_and_keeps_existing_hooks(self):
        settings = json.loads((REPO / ".claude/settings.json").read_text(encoding="utf-8"))
        entries = settings["hooks"]["SessionStart"]
        compact = [entry for entry in entries if entry.get("matcher") == "compact"]
        self.assertEqual(len(compact), 1)
        self.assertIn("resume_card.py", compact[0]["hooks"][0]["command"])
        self.assertTrue(any("agent-board" in h["command"] for e in entries for h in e["hooks"]))
        self.assertTrue(any("shell_guard.py" in h["command"]
                            for e in settings["hooks"]["PreToolUse"] for h in e["hooks"]))
        self.assertEqual(settings["agent"], "orchestrator")

    def test_orchestrator_frontmatter_wires_the_guard_with_existence_check(self):
        text = (REPO / ".claude/agents/orchestrator.md").read_text(encoding="utf-8")
        front = text.split("---")[1]
        self.assertIn('matcher: "Edit|Write|Read"', front)
        command = re.search(r"command: (.+)", front).group(1)
        self.assertIn("orchestrator_guard.py", command)
        # python exits 2 for a missing script, which would deny every Edit,
        # Write and Read; the guard must skip when the file is absent.
        self.assertIn("|| exit 0", command)


if __name__ == "__main__":
    unittest.main()
