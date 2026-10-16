"""Tests for the Codex hook probe in tools/agent-skills/codex_hook_probe.py."""

import importlib.util
import json
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


if __name__ == "__main__":
    unittest.main()
