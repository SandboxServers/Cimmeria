"""Tests for the Codex hook probe in tools/agent-skills/codex_hook_probe.py."""

import contextlib
import importlib.util
import io
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

SCRIPT = Path(__file__).resolve().parent / "codex_hook_probe.py"
spec = importlib.util.spec_from_file_location("codex_hook_probe", SCRIPT)
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)

ORIGINAL = {
    "hooks": {
        "SessionStart": [
            {"hooks": [{"type": "command", "command": "board hook session-start", "timeout": 30}]}
        ],
        "PreToolUse": [
            {
                "matcher": "Bash",
                "hooks": [{"type": "command", "command": "shell_guard.py --harness codex", "timeout": 10}],
            }
        ],
    }
}


def commands(hooks: dict, event: str) -> list[str]:
    return [h["command"] for entry in hooks["hooks"][event] for h in entry["hooks"]]


class ProbeHooksTest(unittest.TestCase):
    def test_extends_original_hooks(self):
        hooks = probe.probe_hooks(ORIGINAL, SCRIPT)
        session = commands(hooks, "SessionStart")
        pre = commands(hooks, "PreToolUse")
        self.assertIn("board hook session-start", session)
        self.assertIn("shell_guard.py --harness codex", pre)
        self.assertEqual(sum("--record" in c for c in session), 1)
        self.assertEqual(sum("--record" in c for c in pre), 1)
        self.assertEqual(len(session), 2)
        self.assertEqual(len(pre), 2)
        self.assertNotIn("matcher", hooks["hooks"]["SessionStart"][-1])
        self.assertNotIn("matcher", hooks["hooks"]["PreToolUse"][-1])

    def test_does_not_mutate_original(self):
        before = json.dumps(ORIGINAL, sort_keys=True)
        probe.probe_hooks(ORIGINAL, SCRIPT)
        self.assertEqual(json.dumps(ORIGINAL, sort_keys=True), before)


class RecordTest(unittest.TestCase):
    def test_session_start_logs_and_injects_token(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            out = probe.record("session-start", '{"source":"startup"}', root)
            rows = probe.read_rows(root)
            self.assertEqual(len(rows), 1)
            self.assertEqual(rows[0]["event"], "session-start")
            self.assertEqual(rows[0]["source"], "startup")
            self.assertEqual(json.loads(out)["hookSpecificOutput"]["additionalContext"], probe.TOKEN)

    def test_pre_tool_use_logs_tool_and_returns_nothing(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            payload = '{"tool_name":"apply_patch","tool_input":{"patch":"x","path":"y"}}'
            self.assertIsNone(probe.record("pre-tool-use", payload, root))
            rows = probe.read_rows(root)
            self.assertEqual(rows[0]["tool_name"], "apply_patch")
            self.assertEqual(rows[0]["keys"], ["patch", "path"])

    def test_bad_stdin_is_tolerated(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.assertIsNone(probe.record("pre-tool-use", "not json", root))
            self.assertEqual(probe.read_rows(root)[0]["tool_name"], None)

    def test_deny_hook_denies_crates_writes_only(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            crates = '{"tool_name":"apply_patch","tool_input":{"patch":"*** Add File: crates\\\\a.rs"}}'
            out = probe.record("pre-tool-use-deny", crates, root)
            decision = json.loads(out)["hookSpecificOutput"]
            self.assertEqual(decision["permissionDecision"], "deny")
            other = '{"tool_name":"apply_patch","tool_input":{"patch":".claude/state/x.txt"}}'
            self.assertIsNone(probe.record("pre-tool-use-deny", other, root))
            self.assertEqual([r["event"] for r in probe.read_rows(root)], ["pre-tool-use-deny"] * 2)


class RunProbeTest(unittest.TestCase):
    def test_timeout_restores_hooks_byte_identical(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            hooks_dir = root / ".codex"
            hooks_dir.mkdir()
            original = b'{\r\n  "hooks": {}\r\n}\r\n'
            (hooks_dir / "hooks.json").write_bytes(original)
            with mock.patch.object(probe.subprocess, "run", side_effect=subprocess.TimeoutExpired("codex", 1)):
                summary = probe.run_probe(root, "codex", 1)
            self.assertEqual((hooks_dir / "hooks.json").read_bytes(), original)
            self.assertFalse(summary["context_injected"])
            self.assertFalse(summary["resume_ran"])
            self.assertTrue(any("timed out" in note for note in summary["notes"]))


CRATES_PAYLOAD = '{"tool_name":"apply_patch","tool_input":{"patch":"*** Add File: crates/x.txt"}}'
LS_PAYLOAD = '{"tool_name":"shell","tool_input":{"command":"ls"}}'


class FakeCodex:
    """Stands in for subprocess.run: each codex call logs the hook rows a scenario names."""

    def __init__(self, root: Path, scenario: dict):
        self.root = root
        self.scenario = scenario
        self.hooks_seen = None

    def __call__(self, args, **kwargs):
        def done(stdout="", stderr="", code=0):
            return subprocess.CompletedProcess(args, code, stdout, stderr)

        if args[1:] == ["--version"]:
            return done("codex-cli 9.9.9\n")
        if args[1] == "exec" and args[2] == "resume":
            call = "resume"
        elif "crates/lo09c-deny-probe.txt" in args[-1]:
            call = "deny"
        else:
            call = "first"
        if call == "first":
            self.hooks_seen = (self.root / ".codex" / "hooks.json").read_text(encoding="utf-8")
        for event, payload in self.scenario.get(call, []):
            probe.record(event, payload, self.root)
        if call == "deny" and self.scenario.get("creates_crates_file"):
            (self.root / "crates").mkdir(exist_ok=True)
            (self.root / "crates" / "lo09c-deny-probe.txt").write_text("probe", encoding="utf-8")
        return done(
            stdout=self.scenario.get("stdout", "") if call == "first" else "",
            stderr=self.scenario.get("stderr", "") if call == "first" else "",
        )


class RunProbeSummaryTest(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.root = Path(self._tmp.name)
        (self.root / ".codex").mkdir()
        self.original = b'{\r\n  "hooks": {}\r\n}\r\n'
        (self.root / ".codex" / "hooks.json").write_bytes(self.original)

    def run_with(self, scenario: dict, deny: bool = False) -> dict:
        fake = FakeCodex(self.root, scenario)
        with mock.patch.object(probe.subprocess, "run", side_effect=fake):
            summary = probe.run_probe(self.root, "codex", 5, deny)
        self.fake = fake
        return summary

    def test_summary_separates_the_resume_call(self):
        scenario = {
            "first": [
                ("session-start", '{"source":"startup"}'),
                ("pre-tool-use", '{"tool_name":"shell","tool_input":{"command":"ls"}}'),
                ("pre-tool-use", '{"tool_name":"apply_patch","tool_input":{"patch":"x"}}'),
            ],
            "resume": [("session-start", '{"source":"resume"}')],
            "stdout": probe.TOKEN,
        }
        summary = self.run_with(scenario)
        self.assertEqual(summary["codex_version"], "codex-cli 9.9.9")
        self.assertTrue(summary["session_start_ran"])
        self.assertEqual(summary["session_start_sources"], ["startup"])
        self.assertTrue(summary["context_injected"])
        self.assertEqual(summary["pre_tool_use_tools"], ["shell", "apply_patch"])
        self.assertTrue(summary["edit_tool_hooked"])
        self.assertTrue(summary["resume_ran"])
        self.assertEqual(summary["resume_source"], "resume")
        self.assertIn("--record session-start", self.fake.hooks_seen)

    def test_no_session_start_in_resume_call_means_resume_did_not_run(self):
        scenario = {"first": [("session-start", '{"source":"startup"}')]}
        summary = self.run_with(scenario)
        self.assertTrue(summary["session_start_ran"])
        self.assertFalse(summary["resume_ran"])
        self.assertIsNone(summary["resume_source"])
        self.assertFalse(summary["context_injected"])
        self.assertFalse(summary["edit_tool_hooked"])

    def test_shell_only_pre_tool_use_is_not_an_edit_hook(self):
        summary = self.run_with({"first": [("pre-tool-use", LS_PAYLOAD)]})
        self.assertEqual(summary["pre_tool_use_tools"], ["shell"])
        self.assertFalse(summary["edit_tool_hooked"])

    def test_empty_log_is_reported_with_hook_lines_from_codex_output(self):
        scenario = {"stderr": "hook: SessionStart Failed\nunrelated line\nhook: SessionStart Failed\n"}
        summary = self.run_with(scenario)
        self.assertFalse(summary["session_start_ran"])
        self.assertEqual(summary["pre_tool_use_tools"], [])
        self.assertEqual(summary["hook_output"], ["hook: SessionStart Failed"])

    def test_hooks_and_edit_files_are_restored_when_a_call_crashes(self):
        (self.root / ".claude" / "state").mkdir(parents=True)
        edit = self.root / ".claude" / "state" / "codex-probe-edit.txt"
        edit.write_text("probe", encoding="utf-8")
        with mock.patch.object(probe.subprocess, "run", side_effect=RuntimeError("boom")):
            with self.assertRaises(RuntimeError):
                probe.run_probe(self.root, "codex", 5)
        self.assertEqual((self.root / ".codex" / "hooks.json").read_bytes(), self.original)
        self.assertFalse(edit.exists())

    def test_deny_blocks_only_when_a_call_was_denied_and_no_file_exists(self):
        blocked = self.run_with({"deny": [("pre-tool-use-deny", CRATES_PAYLOAD)]}, deny=True)
        self.assertTrue(blocked["deny_hook_ran"])
        self.assertEqual(blocked["denied_tools"], ["apply_patch"])
        self.assertFalse(blocked["crates_file_created"])
        self.assertTrue(blocked["crates_edit_blocked"])

    def test_deny_hook_that_only_saw_other_calls_does_not_count_as_blocking(self):
        summary = self.run_with({"deny": [("pre-tool-use-deny", LS_PAYLOAD)]}, deny=True)
        self.assertTrue(summary["deny_hook_ran"])
        self.assertEqual(summary["denied_tools"], [])
        self.assertFalse(summary["crates_edit_blocked"])

    def test_deny_reports_and_removes_a_created_crates_file(self):
        scenario = {"deny": [("pre-tool-use-deny", CRATES_PAYLOAD)], "creates_crates_file": True}
        summary = self.run_with(scenario, deny=True)
        self.assertTrue(summary["crates_file_created"])
        self.assertFalse(summary["crates_edit_blocked"])
        self.assertFalse((self.root / "crates" / "lo09c-deny-probe.txt").exists())


class DefaultCodexTest(unittest.TestCase):
    def test_prefers_the_newest_bundled_desktop_binary(self):
        with tempfile.TemporaryDirectory() as tmp:
            old = Path(tmp, "OpenAI", "Codex", "bin", "0.1.0", "codex.exe")
            new = Path(tmp, "OpenAI", "Codex", "bin", "0.2.0", "codex.exe")
            for index, path in enumerate((old, new)):
                path.parent.mkdir(parents=True)
                path.write_text("x", encoding="utf-8")
                os.utime(path, (1000 + index, 1000 + index))
            with mock.patch.dict(os.environ, {"LOCALAPPDATA": tmp}):
                self.assertEqual(probe.default_codex(), str(new))

    def test_falls_back_to_codex_on_path(self):
        with tempfile.TemporaryDirectory() as tmp:
            with mock.patch.dict(os.environ, {"LOCALAPPDATA": tmp}):
                self.assertEqual(probe.default_codex(), "codex")


class MainTest(unittest.TestCase):
    def test_missing_codex_executable_exits_3(self):
        with mock.patch.object(probe.shutil, "which", return_value=None):
            with mock.patch.object(probe, "run_probe") as run:
                with contextlib.redirect_stderr(io.StringIO()) as err:
                    self.assertEqual(probe.main(["--codex", "no-such-codex"]), 3)
        self.assertIn("not found", err.getvalue())
        run.assert_not_called()


if __name__ == "__main__":
    unittest.main()
