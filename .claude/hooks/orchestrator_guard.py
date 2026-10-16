#!/usr/bin/env python3
"""PreToolUse guard for the orchestrator persona (Edit, Write and Read).

The orchestrator dispatches, decides and keeps the ledger. It does not research
or write code, so this hook refuses:

1. Edit or Write outside `docs/analysis/**`, `.claude/state/**` and `.claude/reports/**`.
   Code goes to packet-coder and docs to documentation-writer. Calls from a subagent
   (payload `agent_id` or `agent_type`) are allowed.
2. Read of a file of 400 lines or more outside `docs/analysis/**`, unless `limit`
   is given and is at most 200. Large reads go to Explore.

Hook protocol as in shell_guard.py: the tool call arrives as JSON on stdin; exit
code 2 refuses it and the stderr text goes back to the model. Any parse failure
allows the call. Wired in the frontmatter `hooks:` of .claude/agents/orchestrator.md.
"""

import json
import os
import sys
from pathlib import Path

READ_LINE_CAP = 400
READ_LIMIT_MAX = 200
WRITABLE_DIRS = (("docs", "analysis"), (".claude", "state"), (".claude", "reports"))
READABLE_DIRS = (("docs", "analysis"),)

WRITE_REASON = (
    "orchestrator_guard: the orchestrator writes only docs/analysis/**, .claude/state/** "
    "and .claude/reports/**. Dispatch packet-coder for code or documentation-writer for docs."
)
READ_REASON = (
    "orchestrator_guard: file is 400+ lines outside docs/analysis/**. Dispatch Explore "
    "to read it, or Read with limit <= 200."
)


def project_dir():
    """The project root that Claude Code exports, or the working directory."""
    return Path(os.environ.get("CLAUDE_PROJECT_DIR") or os.getcwd())


def is_subagent(payload):
    """True when the hook fired inside a subagent.

    Subagent hook payloads carry `agent_id` and/or `agent_type`; either one counts.
    Observed: the hooks in orchestrator.md frontmatter also fire inside the subagents
    the orchestrator dispatches, so without this exemption packet-coder was denied its
    own Edit and Write calls. The guard constrains the orchestrator session only.
    """
    return any(isinstance(payload.get(key), str) and payload.get(key) for key in ("agent_id", "agent_type"))


def scope_roots(payload, root):
    """The project root and, when the payload names one, the session cwd (a worktree)."""
    cwd = payload.get("cwd")
    if isinstance(cwd, str) and cwd:
        return [root, Path(cwd)]
    return [root]


def resolve_target(root, file_path):
    """Absolute, symlink-free path of file_path (relative paths start at root)."""
    target = Path(file_path)
    if not target.is_absolute():
        target = root / target
    return Path(os.path.normcase(os.path.realpath(target)))


def under_dirs(root, target, dirs):
    """True when target lies inside the project under one of the dirs."""
    try:
        parts = target.relative_to(Path(os.path.normcase(os.path.realpath(root)))).parts
    except ValueError:
        return False
    return any(parts[: len(prefix)] == prefix for prefix in dirs)


def has_at_least_lines(path, count):
    """True when the file has at least count lines. Unreadable files count as short."""
    try:
        with open(path, "rb") as handle:
            seen = 0
            for _ in handle:
                seen += 1
                if seen >= count:
                    return True
    except OSError:
        return False
    return False


def verdict(payload, root):
    """Return the refusal reason for a tool call, or None to allow it."""
    if is_subagent(payload):
        return None
    tool = payload.get("tool_name")
    tool_input = payload.get("tool_input") or {}
    file_path = tool_input.get("file_path")
    if not isinstance(file_path, str) or not file_path:
        return None
    roots = scope_roots(payload, root)
    target = resolve_target(roots[-1], file_path)
    if tool in ("Edit", "Write"):
        return None if any(under_dirs(r, target, WRITABLE_DIRS) for r in roots) else WRITE_REASON
    if tool == "Read":
        if any(under_dirs(r, target, READABLE_DIRS) for r in roots):
            return None
        limit = tool_input.get("limit")
        if isinstance(limit, int) and not isinstance(limit, bool) and limit <= READ_LIMIT_MAX:
            return None
        return READ_REASON if has_at_least_lines(target, READ_LINE_CAP) else None
    return None


def main():
    try:
        payload = json.load(sys.stdin)
        if not isinstance(payload, dict):
            return 0
        reason = verdict(payload, project_dir())
    except Exception:
        return 0
    if reason:
        print(reason, file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
