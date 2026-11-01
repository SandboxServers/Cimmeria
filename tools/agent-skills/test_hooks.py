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


def guard(project, tool, tool_input, **extra):
    payload = json.dumps({"tool_name": tool, "tool_input": tool_input, **extra})
    return run_hook(ORCHESTRATOR_GUARD, project, payload)


def write_lines(path, count):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("".join(f"row {i:03d}\n" for i in range(1, count + 1)), encoding="utf-8")


def tool_record(name, tool_input):
    """A transcript record holding one tool use."""
    block = {"type": "tool_use", "id": "toolu", "name": name, "input": tool_input}
    return {"type": "assistant", "message": {"content": [block]}}


def prompt_record(prompt):
    """A transcript record holding one Agent dispatch with this prompt."""
    return tool_record("Agent", {"description": "dispatch", "prompt": prompt})


class ResumeCardTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.project = Path(self.tmp.name)
        self.pointer = str(self.project / ".claude" / "state" / "active-campaign")

    def tearDown(self):
        self.tmp.cleanup()

    def point_at(self, name):
        """Write the global pointer file, which the hook must ignore."""
        state = self.project / ".claude" / "state"
        state.mkdir(parents=True, exist_ok=True)
        (state / "active-campaign").write_text(name + "\n", encoding="utf-8")

    def transcript(self, *records):
        """Write a session transcript; return the hook payload that names it."""
        path = self.project / "session.jsonl"
        path.write_text("".join(json.dumps(record) + "\n" for record in records), encoding="utf-8")
        return json.dumps({"transcript_path": str(path)})

    def card(self, stdin_text):
        """Run the hook; return the additional context, or None when it prints nothing."""
        result = run_hook(RESUME_CARD, self.project, stdin_text)
        self.assertEqual(result.returncode, 0)
        if not result.stdout:
            return None
        return json.loads(result.stdout)["hookSpecificOutput"]["additionalContext"]

    def write_pointer(self, content):
        return tool_record("Write", {"file_path": self.pointer, "content": content})

    def test_no_card_when_the_session_never_wrote_the_pointer(self):
        # F7: the global file names "other", which has a RESUME, but this session
        # wrote nothing and dispatched nothing.
        self.point_at("other")
        write_lines(self.project / "docs/analysis/other/RESUME.md", 5)
        payload = self.transcript({"type": "user", "message": {"content": "hello"}})
        self.assertIsNone(self.card(payload))

    def test_last_write_of_the_pointer_names_the_campaign(self):
        self.point_at("other")
        write_lines(self.project / "docs/analysis/other/RESUME.md", 3)
        write_lines(self.project / "docs/analysis/demo/RESUME.md", 3)
        card = self.card(self.transcript(self.write_pointer("demo\n")))
        self.assertTrue(card.startswith("Context was compacted. Active campaign: demo. "))

    def test_later_write_wins_over_earlier_write(self):
        write_lines(self.project / "docs/analysis/alpha/RESUME.md", 3)
        write_lines(self.project / "docs/analysis/demo/RESUME.md", 3)
        payload = self.transcript(self.write_pointer("alpha\n"), self.write_pointer("demo\n"))
        self.assertIn("Active campaign: demo.", self.card(payload))

    def test_edit_new_string_names_the_campaign(self):
        write_lines(self.project / "docs/analysis/demo/RESUME.md", 3)
        edit = {"file_path": self.pointer, "old_string": "old\n", "new_string": "demo\n"}
        card = self.card(self.transcript(tool_record("Edit", edit)))
        self.assertIn("Active campaign: demo.", card)

    def test_empty_last_write_means_no_card_even_with_agent_prompts(self):
        write_lines(self.project / "docs/analysis/demo/RESUME.md", 3)
        records = [self.write_pointer(""), *(prompt_record("Read docs/analysis/demo/RESUME.md")
                                              for _ in range(5))]
        self.assertIsNone(self.card(self.transcript(*records)))

    def test_inferred_card_when_three_of_four_prompts_name_the_campaign(self):
        write_lines(self.project / "docs/analysis/demo/RESUME.md", 3)
        write_lines(self.project / "docs/analysis/other/RESUME.md", 3)
        prompts = ["Read docs/analysis/demo/RESUME.md", "Check docs/analysis/demo/x.md",
                   ".claude/reports/demo/HT-01.md", "Read docs/analysis/other/RESUME.md"]
        card = self.card(self.transcript(*(prompt_record(p) for p in prompts)))
        self.assertTrue(card.startswith("Context was compacted. Inferred campaign"))
        self.assertIn("demo", card.splitlines()[0])

    def test_no_card_when_two_of_four_prompts_name_the_campaign(self):
        write_lines(self.project / "docs/analysis/demo/RESUME.md", 3)
        write_lines(self.project / "docs/analysis/other/RESUME.md", 3)
        prompts = ["docs/analysis/demo/a.md", "docs/analysis/demo/b.md",
                   "docs/analysis/other/a.md", "docs/analysis/other/b.md"]
        self.assertIsNone(self.card(self.transcript(*(prompt_record(p) for p in prompts))))

    def test_no_card_when_there_are_only_two_prompts(self):
        write_lines(self.project / "docs/analysis/demo/RESUME.md", 3)
        prompts = ["docs/analysis/demo/a.md", "docs/analysis/demo/b.md"]
        self.assertIsNone(self.card(self.transcript(*(prompt_record(p) for p in prompts))))

    def test_prompt_naming_a_campaign_without_resume_does_not_count(self):
        # Unfiltered, ghost and demo split 3 to 3 and no card is printed.
        write_lines(self.project / "docs/analysis/demo/RESUME.md", 3)
        prompts = ["docs/analysis/ghost/a.md"] * 3 + ["docs/analysis/demo/a.md"] * 3
        card = self.card(self.transcript(*(prompt_record(p) for p in prompts)))
        self.assertIn("dispatches): demo.", card)

    def test_three_of_three_prompts_name_the_campaign(self):
        write_lines(self.project / "docs/analysis/demo/RESUME.md", 3)
        prompts = ["docs/analysis/demo/a.md"] * 3
        card = self.card(self.transcript(*(prompt_record(p) for p in prompts)))
        self.assertTrue(card.startswith("Context was compacted. Inferred campaign"))
        self.assertIn("dispatches): demo.", card)

    def test_two_of_three_prompts_name_the_campaign(self):
        # 2 of 3 is exactly the threshold: 3 * 2 >= 2 * 3.
        write_lines(self.project / "docs/analysis/demo/RESUME.md", 3)
        write_lines(self.project / "docs/analysis/other/RESUME.md", 3)
        prompts = ["docs/analysis/demo/a.md", "docs/analysis/demo/b.md", "docs/analysis/other/a.md"]
        card = self.card(self.transcript(*(prompt_record(p) for p in prompts)))
        self.assertIn("dispatches): demo.", card)

    def test_only_the_last_40_prompts_vote(self):
        # The first 10 prompts name other. The last 40 are 27 demo and 13 other:
        # 3 * 27 >= 2 * 40, so demo wins. All 50 would be 27 of 50 and no card.
        write_lines(self.project / "docs/analysis/demo/RESUME.md", 3)
        write_lines(self.project / "docs/analysis/other/RESUME.md", 3)
        prompts = (["docs/analysis/other/a.md"] * 10 + ["docs/analysis/demo/a.md"] * 27
                   + ["docs/analysis/other/b.md"] * 13)
        card = self.card(self.transcript(*(prompt_record(p) for p in prompts)))
        self.assertIn("dispatches): demo.", card)

    def test_campaign_name_is_matched_in_mixed_case(self):
        # The needle is case-sensitive, so only the path's case is folded.
        self.point_at("other")
        write_lines(self.project / "docs/analysis/demo/RESUME.md", 3)
        path = str(self.project / ".CLAUDE" / "State" / "active-campaign")
        card = self.card(self.transcript(tool_record("Write", {"file_path": path, "content": "demo\n"})))
        self.assertIn("Active campaign: demo.", card)

    def test_multiedit_last_edit_names_the_campaign(self):
        write_lines(self.project / "docs/analysis/demo/RESUME.md", 3)
        edits = [{"old_string": "x", "new_string": "alpha\n"}, {"old_string": "y", "new_string": "demo\n"}]
        card = self.card(self.transcript(tool_record("MultiEdit", {"file_path": self.pointer, "edits": edits})))
        self.assertIn("Active campaign: demo.", card)

    def test_pointer_lookalike_path_does_not_count(self):
        # Only .claude/state/active-campaign is the pointer, not any path ending in its name.
        write_lines(self.project / "docs/analysis/other/RESUME.md", 3)
        decoy = str(self.project / ".claude" / "state" / "other" / "active-campaign")
        self.assertIsNone(self.card(self.transcript(tool_record("Write", {"file_path": decoy, "content": "other\n"}))))

    def test_truncates_resume_at_40_lines(self):
        write_lines(self.project / "docs/analysis/demo/RESUME.md", 100)
        card = self.card(self.transcript(self.write_pointer("demo\n")))
        self.assertIn("row 040", card)
        self.assertNotIn("row 041", card)

    def test_pointer_with_bom_still_names_the_campaign(self):
        write_lines(self.project / "docs/analysis/demo/RESUME.md", 3)
        card = self.card(self.transcript(self.write_pointer("﻿demo\r\n")))
        self.assertIn("Active campaign: demo.", card)

    def test_rejects_campaign_name_that_is_a_path(self):
        # The RESUME.md sits where "../demo" and "..\demo" would resolve, so
        # only the name check keeps the card empty.
        write_lines(self.project / "docs/demo/RESUME.md", 3)
        for name in ("../demo", "..\\demo", "."):
            payload = self.transcript(self.write_pointer(name + "\n"))
            self.assertIsNone(self.card(payload), name)

    def test_exits_zero_on_malformed_json(self):
        write_lines(self.project / "docs/analysis/demo/RESUME.md", 3)
        self.assertIsNone(self.card("{not json"))

    def test_exits_zero_when_building_the_card_raises(self):
        # The payload parses, so the failure must be forced inside main().
        spec = importlib.util.spec_from_file_location("resume_card_under_test", RESUME_CARD)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with mock.patch.object(module, "build_card", side_effect=RuntimeError("boom")), \
                mock.patch.object(sys, "stdin", io.StringIO("{}")), \
                mock.patch.object(sys, "stdout", new_callable=io.StringIO) as out:
            self.assertEqual(module.main(), 0)
        self.assertEqual(out.getvalue(), "")

    def test_missing_transcript_means_no_card(self):
        self.assertIsNone(self.card("{}"))
        missing = str(self.project / "gone.jsonl")
        self.assertIsNone(self.card(json.dumps({"transcript_path": missing})))


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

    def test_subagent_payload_is_not_guarded(self):
        # Hooks from orchestrator.md also fire inside dispatched subagents; their
        # payload carries agent_id, so code edits and big reads pass.
        result = guard(self.project, "Edit", {"file_path": "crates/foo.rs"}, agent_id="a1b2c3")
        self.assertEqual(result.returncode, 0, result.stderr)
        write_lines(self.project / "crates/big.rs", 500)
        result = guard(self.project, "Read", {"file_path": "crates/big.rs"}, agent_id="a1b2c3")
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_payload_without_agent_id_is_still_denied(self):
        # agent_type alone must not exempt: the orchestrator main thread may carry it.
        for marker in ({}, {"agent_id": ""}, {"agent_id": None}, {"agent_type": None},
                       {"agent_type": "packet-coder"}, {"agent_type": "orchestrator"}):
            result = guard(self.project, "Edit", {"file_path": "crates/foo.rs"}, **marker)
            self.assertEqual(result.returncode, 2, marker)

    def test_allows_write_to_reports_dir(self):
        target = ".claude/reports/lean-orchestration/lo-guard-coder.md"
        result = guard(self.project, "Write", {"file_path": target})
        self.assertEqual(result.returncode, 0, result.stderr)
        result = guard(self.project, "Edit", {"file_path": str(self.project / target)})
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_allows_worktree_ledger_from_main_checkout(self):
        # The orchestrator's cwd is the main checkout, so no payload cwd names the worktree.
        worktree = self.project / ".claude" / "worktrees" / "x"
        ledger = str(worktree / "docs" / "analysis" / "x" / "README.md")
        result = guard(self.project, "Write", {"file_path": ledger})
        self.assertEqual(result.returncode, 0, result.stderr)
        state = str(worktree / ".claude" / "state" / "active-campaign")
        result = guard(self.project, "Write", {"file_path": state})
        self.assertEqual(result.returncode, 0, result.stderr)
        report = str(worktree / ".claude" / "reports" / "x" / "r.md")
        result = guard(self.project, "Edit", {"file_path": report})
        self.assertEqual(result.returncode, 0, result.stderr)
        write_lines(worktree / "docs" / "analysis" / "x" / "big.md", 500)
        big = str(worktree / "docs" / "analysis" / "x" / "big.md")
        result = guard(self.project, "Read", {"file_path": big})
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_denies_worktree_code_from_main_checkout(self):
        worktree = self.project / ".claude" / "worktrees" / "x"
        code = str(worktree / "crates" / "foo.rs")
        self.assertEqual(guard(self.project, "Edit", {"file_path": code}).returncode, 2)
        write_lines(worktree / "crates" / "big.rs", 500)
        big = str(worktree / "crates" / "big.rs")
        self.assertEqual(guard(self.project, "Read", {"file_path": big}).returncode, 2)

    def test_denies_escape_through_worktrees_dotdot(self):
        escape = str(self.project / ".claude" / "worktrees" / "x" / ".." / ".." / ".." / "crates" / "foo.rs")
        self.assertEqual(guard(self.project, "Write", {"file_path": escape}).returncode, 2)
        # No worktree-name segment: parts[3:] is ("analysis", "a.md"), which matches no dir.
        no_name = str(self.project / ".claude" / "worktrees" / "docs" / "analysis" / "a.md")
        self.assertEqual(guard(self.project, "Write", {"file_path": no_name}).returncode, 2)

    def test_session_cwd_worktree_resolves_its_own_paths(self):
        # The project root is the main checkout; a managed worktree under it is the session cwd.
        worktree = self.project / ".claude" / "worktrees" / "x"
        worktree.mkdir(parents=True)
        cwd = {"cwd": str(worktree)}
        ledger = str(worktree / "docs" / "analysis" / "x" / "README.md")
        self.assertEqual(guard(self.project, "Edit", {"file_path": ledger}, **cwd).returncode, 0)
        relative = {"file_path": "docs/analysis/x/README.md"}
        self.assertEqual(guard(self.project, "Edit", relative, **cwd).returncode, 0)
        code = str(worktree / "crates" / "foo.rs")
        self.assertEqual(guard(self.project, "Edit", {"file_path": code}, **cwd).returncode, 2)
        write_lines(worktree / "crates" / "big.rs", 500)
        result = guard(self.project, "Read", {"file_path": "crates/big.rs"}, **cwd)
        self.assertEqual(result.returncode, 2)

    def test_cwd_outside_project_is_ignored(self):
        # A payload cwd that is not the project root or a managed worktree grants nothing.
        outside = tempfile.TemporaryDirectory()
        self.addCleanup(outside.cleanup)
        target = str(Path(outside.name) / "docs" / "analysis" / "x" / "README.md")
        result = guard(self.project, "Write", {"file_path": target}, cwd=outside.name)
        self.assertEqual(result.returncode, 2, result.stderr)

    def test_cwd_outside_managed_worktrees_is_ignored(self):
        # Inside the project but not under .claude/worktrees/<name> is not a session scope.
        plain = self.project / "wt"
        plain.mkdir()
        target = str(plain / "docs" / "analysis" / "x" / "README.md")
        result = guard(self.project, "Write", {"file_path": target}, cwd=str(plain))
        self.assertEqual(result.returncode, 2, result.stderr)


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

    def test_banner_is_the_first_session_start_entry(self):
        settings = json.loads((REPO / ".claude/settings.json").read_text(encoding="utf-8"))
        first = settings["hooks"]["SessionStart"][0]
        self.assertNotIn("matcher", first)
        command = first["hooks"][0]["command"]
        self.assertIn("mode_banner.sh", command)
        self.assertIn("|| exit 0", command)

    def test_every_python_hook_goes_through_the_launcher(self):
        settings = json.loads((REPO / ".claude/settings.json").read_text(encoding="utf-8"))
        commands = [h["command"] for event in ("SessionStart", "PreToolUse")
                    for entry in settings["hooks"][event] for h in entry["hooks"]]
        front = (REPO / ".claude/agents/orchestrator.md").read_text(encoding="utf-8").split("---")[1]
        commands.append(re.search(r"command: (.+)", front).group(1))
        codex = json.loads((REPO / ".codex/hooks.json").read_text(encoding="utf-8"))
        commands += [h["command"] for event in codex["hooks"].values()
                     for entry in event for h in entry["hooks"]]
        python_commands = [c for c in commands if ".py" in c]
        self.assertTrue(python_commands)
        for command in python_commands:
            self.assertIn("py.sh", command)
            self.assertNotIn("; python ", command)


if __name__ == "__main__":
    unittest.main()
