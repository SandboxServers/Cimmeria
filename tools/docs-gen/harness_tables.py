"""Render the AI-harness hook and agent tables for regen.py (gen:harness-hooks, gen:harness-agents).

Every hook is a command in .claude/settings.json, in an agent's frontmatter `hooks:`
block, or in .codex/hooks.json. The hook table names its event and matcher, the script
it runs and that script's one-line reason (its module docstring or its `# ` comment),
so a hook that gains no reason shows up as a row without one. The agent table gives
each agent's model, effort, turn cap and role. Standard library only: CI has no
PyYAML, so the frontmatter is read with the small subset the agent files use.
"""
from __future__ import annotations

import ast
import json
import pathlib
import re
from dataclasses import dataclass

INLINE_REASONS = {
    "agent-board": "Agent board: registers the session and shows unread board activity. External tool, skipped when it is not installed.",
}
NO_REASON = "(no reason: add a docstring)"
SCRIPT_RE = re.compile(r"\.claude/hooks/([\w-]+\.(?:py|sh))")
FIELD_RE = re.compile(r"^([A-Za-z_][\w-]*):\s*(.*)$")
EVENT_RE = re.compile(r"^  ([A-Za-z]+):\s*$")


@dataclass(frozen=True)
class Hook:
    event: str      # "SessionStart", "PreToolUse", ...
    matcher: str    # "" when the entry has none
    where: str      # ".claude/settings.json", ".codex/hooks.json" or "agent: <name>"
    runs: str       # script file name, e.g. "shell_guard.py", or "agent-board"
    reason: str


def hook_script(command: str) -> str | None:
    """The script a hook command runs: the first .claude/hooks script that is not py.sh."""
    for name in SCRIPT_RE.findall(command):
        if name != "py.sh":
            return name
    if ".agent-board" in command:
        return "agent-board"
    return None


def script_reason(path: pathlib.Path) -> str:
    """The first line of a .py module docstring, or the first `# ` comment of a .sh script."""
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return ""
    if path.suffix == ".py":
        try:
            doc = ast.get_docstring(ast.parse(text)) or ""
        except SyntaxError:
            return ""
        return next((line.strip() for line in doc.splitlines() if line.strip()), "")
    if path.suffix == ".sh":
        for line in text.splitlines():
            if line.startswith("# "):
                return line[2:].strip()
    return ""


def _hook(root: pathlib.Path, event: str, matcher: str, where: str, command: str) -> Hook:
    runs = hook_script(command) or "(inline)"
    if runs == "agent-board":
        reason = INLINE_REASONS[runs]
    else:
        reason = script_reason(root / ".claude" / "hooks" / runs)
    return Hook(event, matcher, where, runs, reason)


def json_hooks(root: pathlib.Path, rel: str, where: str) -> list[Hook]:
    """The command hooks of one JSON settings file; none when the file is absent."""
    path = root / rel
    if not path.is_file():
        return []
    data = json.loads(path.read_text(encoding="utf-8"))
    out = []
    for event, entries in data.get("hooks", {}).items():
        for entry in entries:
            for hook in entry.get("hooks", []):
                if "command" in hook:
                    out.append(_hook(root, event, entry.get("matcher", ""), where, hook["command"]))
    return out


def _frontmatter(text: str) -> list[str]:
    """The lines between the first two `---` lines, or none when the file has no frontmatter."""
    lines = [line.rstrip() for line in text.splitlines()]
    if not lines or lines[0] != "---" or "---" not in lines[1:]:
        return []
    return lines[1 : lines.index("---", 1)]


def _scalar(raw: str) -> str:
    raw = raw.strip()
    if raw.startswith('"'):
        return json.loads(raw)
    if len(raw) >= 2 and raw[0] == raw[-1] == "'":
        return raw[1:-1]
    return raw


def _fields(lines: list[str]) -> dict[str, str]:
    """The top-level `key: value` lines of frontmatter."""
    fields = {}
    for line in lines:
        m = FIELD_RE.match(line)
        if m:
            fields[m.group(1)] = _scalar(m.group(2))
    return fields


def _frontmatter_hooks(lines: list[str]) -> list[tuple[str, str, str]]:
    """(event, matcher, command) for each command under the frontmatter `hooks:` block."""
    found, in_hooks, event, matcher, entry_indent = [], False, "", "", None
    for line in lines:
        if line and not line[0].isspace():
            in_hooks = line == "hooks:"
            continue
        if not in_hooks:
            continue
        event_match = EVENT_RE.match(line)
        if event_match:
            event, matcher, entry_indent = event_match.group(1), "", None
            continue
        stripped = line.lstrip()
        indent = len(line) - len(stripped)
        if stripped.startswith("- "):
            if entry_indent is None:
                entry_indent = indent
            if indent == entry_indent:
                matcher = ""
            stripped = stripped[2:].lstrip()
        key, _, value = stripped.partition(":")
        if key == "matcher":
            matcher = _scalar(value)
        elif key == "command":
            found.append((event, matcher, _scalar(value)))
    return found


def agent_hooks(root: pathlib.Path) -> list[Hook]:
    """The hooks in .claude/agents/*.md frontmatter, in agent file name order."""
    out = []
    for path in sorted((root / ".claude" / "agents").glob("*.md")):
        lines = _frontmatter(path.read_text(encoding="utf-8"))
        name = _fields(lines).get("name", path.stem)
        for event, matcher, command in _frontmatter_hooks(lines):
            out.append(_hook(root, event, matcher, f"agent: {name}", command))
    return out


def collect_hooks(root: pathlib.Path) -> list[Hook]:
    """Settings hooks, then agent frontmatter hooks, then the Codex hooks."""
    return (
        json_hooks(root, ".claude/settings.json", ".claude/settings.json")
        + agent_hooks(root)
        + json_hooks(root, ".codex/hooks.json", ".codex/hooks.json")
    )


def _cell(text: str) -> str:
    return text.replace("|", "\\|")


def render_hooks(root: pathlib.Path) -> str:
    """The hook table for gen:harness-hooks."""
    out = ["| Event | Matcher | Configured in | Runs | Why |", "|---|---|---|---|---|"]
    for hook in collect_hooks(root):
        matcher = _cell(hook.matcher) or "(all)"
        reason = _cell(hook.reason) or NO_REASON
        out.append(f"| {hook.event} | {matcher} | `{hook.where}` | `{hook.runs}` | {reason} |")
    return "\n" + "\n".join(out) + "\n"


def render_agents(root: pathlib.Path) -> str:
    """The agent table for gen:harness-agents, sorted by agent name."""
    rows = []
    for path in sorted((root / ".claude" / "agents").glob("*.md")):
        fields = _fields(_frontmatter(path.read_text(encoding="utf-8")))
        description = fields.get("description", "")
        head, sep, _ = description.partition(". ")
        role = head + "." if sep else description
        rows.append((
            fields.get("name", path.stem),
            fields.get("model", "inherit"),
            fields.get("effort", "-"),
            fields.get("maxTurns", "-"),
            _cell(role),
        ))
    out = ["| Agent | Model | Effort | Max turns | Role |", "|---|---|---|---|---|"]
    out += [f"| {name} | {model} | {effort} | {turns} | {role} |" for name, model, effort, turns, role in sorted(rows)]
    return "\n" + "\n".join(out) + "\n"
