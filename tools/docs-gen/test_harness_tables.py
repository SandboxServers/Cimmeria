"""Unit tests for harness_tables.py. Run: python -m unittest discover -s tools/docs-gen -p "test_*.py"."""
from __future__ import annotations

import json
import pathlib
import sys
import tempfile
import types
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import harness_tables as ht  # noqa: E402
import regen  # noqa: E402

REPO = pathlib.Path(__file__).resolve().parents[2]
BOARD = '[ -x "$HOME/.agent-board/board" ] && "$HOME/.agent-board/board" hook session-start; exit 0'
BANNER = 'f="$CLAUDE_PROJECT_DIR/.claude/hooks/mode_banner.sh"; [ -n "$CLAUDE_PROJECT_DIR" ] && [ -f "$f" ] || exit 0; . "$f"'
GUARD = ('f="$CLAUDE_PROJECT_DIR/.claude/hooks/shell_guard.py"; [ -n "$CLAUDE_PROJECT_DIR" ] && [ -f "$f" ] || exit 0; '
         'set -- "$f" --harness claude; . "$CLAUDE_PROJECT_DIR/.claude/hooks/py.sh"')


def _write(root: pathlib.Path, rel: str, text: str) -> None:
    path = root / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


class HarnessTablesTest(unittest.TestCase):
    def test_render_hooks_from_fixture(self):
        settings = {
            "hooks": {
                "SessionStart": [
                    {"hooks": [{"type": "command", "command": BOARD, "timeout": 30}]},
                    {"hooks": [{"type": "command", "command": BANNER, "timeout": 5}]},
                ],
                "PreToolUse": [
                    {"matcher": "Bash|PowerShell", "hooks": [{"type": "command", "command": GUARD}]},
                ],
            }
        }
        codex = {"hooks": {"PreToolUse": [{"matcher": "Bash", "hooks": [
            {"type": "command", "command": 'set -- "$f" --harness codex; . "$r/.claude/hooks/py.sh"; x=".claude/hooks/shell_guard.py"'}]}]}}
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            _write(root, ".claude/settings.json", json.dumps(settings))
            _write(root, ".codex/hooks.json", json.dumps(codex))
            _write(root, ".claude/hooks/shell_guard.py", '"""Guard X.\n\nmore"""\n')
            _write(root, ".claude/hooks/mode_banner.sh", "# Banner Y.\necho hi\n")
            out = ht.render_hooks(root)
        self.assertTrue(out.startswith("\n") and out.endswith("\n"))
        self.assertIn("| Event | Matcher | Configured in | Runs | Why |", out)
        self.assertIn("| SessionStart | (all) | `.claude/settings.json` | `mode_banner.sh` | Banner Y. |", out)
        self.assertIn("| PreToolUse | Bash\\|PowerShell | `.claude/settings.json` | `shell_guard.py` | Guard X. |", out)
        self.assertIn("`.codex/hooks.json` | `shell_guard.py` | Guard X.", out)
        self.assertIn("| SessionStart | (all) | `.claude/settings.json` | `agent-board`", out)
        self.assertNotIn("`py.sh`", out)

    def test_agent_frontmatter_hooks_are_collected(self):
        agent = (
            "---\n"
            "name: orc\n"
            'description: "Orc. Does things."\n'
            "model: haiku\n"
            "hooks:\n"
            "  PreToolUse:\n"
            '    - matcher: "Edit|Write|Read"\n'
            "      hooks:\n"
            "        - type: command\n"
            "          command: 'f=\"$CLAUDE_PROJECT_DIR/.claude/hooks/orchestrator_guard.py\"; python \"$f\"'\n"
            "---\n\nBody.\n"
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            _write(root, ".claude/agents/orc.md", agent)
            _write(root, ".claude/hooks/orchestrator_guard.py", '"""Orchestrator guard.\n"""\n')
            out = ht.render_hooks(root)
        self.assertIn("| PreToolUse | Edit\\|Write\\|Read | `agent: orc` | `orchestrator_guard.py` | Orchestrator guard. |", out)

    def test_render_agents_sorted_and_escaped(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            _write(root, ".claude/agents/b.md", "---\nname: b\nmodel: haiku\neffort: low\nmaxTurns: 9\n"
                                                'description: "Does b. More."\n---\n')
            _write(root, ".claude/agents/a.md", "---\nname: a\n"
                                                'description: "Takes x|y input. Rest."\n---\n')
            out = ht.render_agents(root)
        lines = [line for line in out.splitlines() if line.startswith("| ") and not line.startswith("| Agent")]
        self.assertEqual(lines, [
            "| a | inherit | - | - | Takes x\\|y input. |",
            "| b | haiku | low | 9 | Does b. |",
        ])

    def test_every_repo_hook_states_its_reason(self):
        hooks = ht.collect_hooks(REPO)
        self.assertTrue(hooks)
        for hook in hooks:
            self.assertTrue(hook.reason, f"no reason for {hook.where} {hook.runs}")
        scripts = sorted((REPO / ".claude" / "hooks").glob("*"))
        scripts = [p for p in scripts if p.suffix in (".py", ".sh")]
        self.assertTrue(scripts)
        for path in scripts:
            self.assertTrue(ht.script_reason(path), f"{path.name} has no docstring or # reason")

    def test_missing_reason_and_inline_command_are_marked(self):
        settings = {"hooks": {"Stop": [{"hooks": [
            {"type": "command", "command": "echo done"},
            {"type": "command", "command": 'f="$CLAUDE_PROJECT_DIR/.claude/hooks/bare.py"; . "$f"'},
        ]}]}}
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            _write(root, ".claude/settings.json", json.dumps(settings))
            _write(root, ".claude/hooks/bare.py", "import sys\n")
            out = ht.render_hooks(root)
        self.assertIn("| Stop | (all) | `.claude/settings.json` | `(inline)` | (no reason: add a docstring) |", out)
        self.assertIn("| Stop | (all) | `.claude/settings.json` | `bare.py` | (no reason: add a docstring) |", out)

    def test_generators_registered(self):
        self.assertIn("harness-hooks", regen.GENERATORS)
        self.assertIn("harness-agents", regen.GENERATORS)

    def test_registered_generators_render_their_own_table(self):
        # A swapped or mis-wired registration would still pass the membership check above.
        ctx = types.SimpleNamespace(root=REPO)
        hooks = regen.GENERATORS["harness-hooks"](ctx, [], "")
        agents = regen.GENERATORS["harness-agents"](ctx, [], "")
        self.assertEqual(hooks, ht.render_hooks(REPO))
        self.assertEqual(agents, ht.render_agents(REPO))
        self.assertIn("| Event | Matcher |", hooks)
        self.assertIn("| Agent | Model |", agents)


if __name__ == "__main__":
    unittest.main()
