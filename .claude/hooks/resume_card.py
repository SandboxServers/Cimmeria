#!/usr/bin/env python3
"""SessionStart hook for the "compact" matcher: re-reads this session's campaign.

After a compaction the orchestrator has lost its working context. The card
names the campaign this session worked on, never a shared global pointer:

- Tier 1: the last Write, Edit or MultiEdit of `.claude/state/active-campaign`
  in this session's transcript. An empty write means no card.
- Tier 2, only when the session never wrote that pointer: the campaign that
  most of its last 40 Agent prompts name (at least 3 prompts, and 2 of every
  3 agree). That card says the campaign is inferred.

A card names `docs/analysis/<name>/RESUME.md`, which must exist, and holds its
first 40 lines. Print the SessionStart hook JSON with the card, or nothing.

Never fails the session: every error ends with exit 0 and no output.
"""

import json
import os
import re
import sys
from collections import Counter
from pathlib import Path

CARD_LINES = 40
LAST_PROMPTS = 40
MAX_BYTES = 200 * 1024 * 1024
POINTER_TOOLS = ("Write", "Edit", "MultiEdit")
POINTER = (
    "Context was compacted. Active campaign: {name}. "
    "Read docs/analysis/{name}/RESUME.md and the ledger README before acting."
)
INFERRED = (
    "Context was compacted. Inferred campaign (from this session's dispatches): {name}. "
    "Read docs/analysis/{name}/RESUME.md and the ledger README before acting; "
    "if this is not your campaign, ignore this card."
)
# Campaign names as the prompts spell them: a docs/analysis or reports folder.
NAME_RES = (
    re.compile(r"docs[\\/]+analysis[\\/]+([a-z0-9][a-z0-9.-]*[a-z0-9])"),
    re.compile(r"\.claude[\\/]+reports[\\/]+([a-z0-9][a-z0-9.-]*[a-z0-9])"),
)


def project_dir():
    """The project root that Claude Code exports, or the working directory."""
    return Path(os.environ.get("CLAUDE_PROJECT_DIR") or os.getcwd())


def norm_path(path):
    """A file path without quotes, with forward slashes and in lowercase."""
    return path.strip("'\"").replace("\\", "/").lower()


def valid_name(name):
    """A campaign name is one folder name under docs/analysis/, never a path."""
    return name not in ("", ".", "..") and "/" not in name and "\\" not in name


def resume_file(root, name):
    """The RESUME.md of a campaign, or None when the name is rejected or the file is missing."""
    if not valid_name(name):
        return None
    resume = root / "docs" / "analysis" / name / "RESUME.md"
    return resume if resume.is_file() else None


def matching_lines(path, needle):
    """The transcript lines that contain needle; a file over 200 MB is read from its end."""
    with open(path, "rb") as handle:
        size = os.fstat(handle.fileno()).st_size
        if size > MAX_BYTES:
            handle.seek(size - MAX_BYTES)
            handle.readline()  # the partial first line
        for raw in handle:
            if needle in raw:
                yield raw.decode("utf-8", errors="replace")


def tool_uses(transcript, needle, names):
    """(tool name, input) for each tool use with a name in names, in transcript order."""
    for line in matching_lines(transcript, needle):
        try:
            record = json.loads(line)
        except ValueError:
            continue
        message = record.get("message") if isinstance(record, dict) else None
        content = message.get("content") if isinstance(message, dict) else None
        for block in content if isinstance(content, list) else []:
            if not isinstance(block, dict) or block.get("type") != "tool_use":
                continue
            data = block.get("input")
            if block.get("name") in names and isinstance(data, dict):
                yield block["name"], data


def pointer_text(tool, data):
    """The text a pointer write leaves in the file: content, or the new_string."""
    if tool == "Write":
        return str(data.get("content") or "")
    if tool == "Edit":
        return str(data.get("new_string") or "")
    edits = data.get("edits")
    last = edits[-1] if isinstance(edits, list) and edits else {}
    return str(last.get("new_string") or "") if isinstance(last, dict) else ""


def tier1(transcript):
    """The text of the session's last pointer write, or None when it never wrote one."""
    text = None
    for tool, data in tool_uses(transcript, b"active-campaign", POINTER_TOOLS):
        if norm_path(str(data.get("file_path") or "")).endswith("/.claude/state/active-campaign"):
            text = pointer_text(tool, data)
    return text


def tier2(transcript, root):
    """The campaign that most of the last 40 Agent prompts name, or None."""
    prompts = [json.dumps(data).lower() for _, data in tool_uses(transcript, b'"Agent"', ("Agent",))]
    named = []
    for prompt in prompts[-LAST_PROMPTS:]:
        found = {m.group(1) for rx in NAME_RES for m in rx.finditer(prompt)}
        names = {name for name in found if resume_file(root, name)}
        if names:
            named.append(names)
    if len(named) < 3:
        return None
    top, count = Counter(name for names in named for name in names).most_common(1)[0]
    return top if 3 * count >= 2 * len(named) else None


def card_for(root, name, template):
    """The card for a campaign: the template line, then the first 40 lines of RESUME.md."""
    resume = resume_file(root, name)
    if resume is None:
        return None
    body = resume.read_text(encoding="utf-8", errors="replace").splitlines()[:CARD_LINES]
    return "\n".join([template.format(name=name), *body])


def build_card(root, transcript_path):
    """Return the card text for this session's transcript, or None."""
    if not isinstance(transcript_path, str) or not Path(transcript_path).is_file():
        return None
    transcript = Path(transcript_path)
    written = tier1(transcript)
    if written is not None:
        # A BOM (Windows editors) must not hide the folder name.
        lines = [line.strip() for line in written.lstrip("﻿").splitlines() if line.strip()]
        return card_for(root, lines[0] if lines else "", POINTER)
    name = tier2(transcript, root)
    return card_for(root, name, INFERRED) if name else None


def main():
    try:
        payload = json.loads(sys.stdin.read())
        transcript = payload.get("transcript_path") if isinstance(payload, dict) else None
        card = build_card(project_dir(), transcript)
        if card is None:
            return 0
        output = {
            "hookSpecificOutput": {
                "hookEventName": "SessionStart",
                "additionalContext": card,
            }
        }
        print(json.dumps(output))
    except Exception:
        return 0
    return 0


if __name__ == "__main__":
    sys.exit(main())
