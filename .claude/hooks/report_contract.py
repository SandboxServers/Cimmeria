#!/usr/bin/env python3
"""SubagentStop hook: blocks a subagent's stop until its return line and report match the contract.

Reads the SubagentStop payload from stdin. An agent whose file has a `## Reporting` section
must end with one return line (LINE). A haiku agent (line mode) must also write no file. Any
other agent (report mode) must name a report under the brief's path, and that file must be
fresh. A block prints `{"decision": "block", "reason": ...}`. Every other path, and every
error, exits 0 with no output (fail open).
"""

import json
import os
import re
import sys
from datetime import datetime
from pathlib import Path

# Copied verbatim from tools/token-profile/metrics/transcripts.py (C1); HT-09 checks that the copies match.
RP = re.compile(r"[^\s`'\"()<>]*\.claude[\\/]reports[\\/][^\\/\s`'\"()<>]+[\\/][^\\/\s`'\"()<>]+\.md")
LINE = re.compile(r"^(?P<id>\S+)\s+(?P<status>done|blocked|failed)\s+(?P<sha>[0-9a-f]{7,40}|-)\s+"
                  r"(?P<report>\S+\.md|-)(?P<extra>(?:\s+[a-z_]+=\S+)*)\s*$")
CAP_MARKER = "Context cap reached"
# A real deny's result text starts with the marker, or with "PreToolUse:<tool> hook error: " and the marker.
# Anchoring there keeps a brief, a Read or a Grep result that merely quotes the marker from counting.
CAP_RE = re.compile(r"^(?:PreToolUse:\S+ hook error: )?" + re.escape(CAP_MARKER))
SLACK_SECONDS = 5

REASON_LINE = (
    "report_contract: end your reply with exactly one line: <id> <done|blocked|failed> <commit-sha|-> - "
    "(add reason=<word> when blocked or failed). Write no file and call no tool."
)
REASON_NO_LINE = (
    "report_contract: your final message has no return line. Write your report to {path} if you have not, "
    "then end with exactly one line: <id> <done|blocked|failed> <commit-sha|-> <path>. "
    "Call no other tool after that."
)
REASON_WRONG_PATH = (
    "report_contract: your return line names {got}, but the brief's report path is {path}. "
    "Write the report there and end with exactly one line naming it. Call no other tool after that."
)
REASON_MISSING = (
    "report_contract: {path} is missing, empty or older than this run. Write your full report there, "
    "then end with exactly one line: <id> <done|blocked|failed> <commit-sha|-> <path>. "
    "Call no other tool after that."
)


def parse_ts(text):
    """Return the epoch seconds of an ISO timestamp, or None when it does not parse."""
    try:
        return datetime.fromisoformat(text.replace("Z", "+00:00")).timestamp()
    except (AttributeError, ValueError):
        return None


def message_text(rec):
    """Return the text of a transcript record: a string content, or its text blocks joined."""
    message = rec.get("message")
    content = message.get("content") if isinstance(message, dict) else None
    if isinstance(content, str):
        return content
    if not isinstance(content, list):
        return ""
    return "\n".join(b["text"] for b in content
                     if isinstance(b, dict) and b.get("type") == "text" and isinstance(b.get("text"), str))


def is_cap_deny(rec):
    """True when the record is a user-level tool_result whose text starts with the cap deny."""
    message = rec.get("message")
    content = message.get("content") if rec.get("type") == "user" and isinstance(message, dict) else None
    if not isinstance(content, list):
        return False
    for block in content:
        if not isinstance(block, dict) or block.get("type") != "tool_result":
            continue
        body = block.get("content")
        if isinstance(body, list):
            body = "\n".join(b["text"] for b in body
                             if isinstance(b, dict) and b.get("type") == "text" and isinstance(b.get("text"), str))
        if isinstance(body, str) and CAP_RE.match(body.lstrip()):
            return True
    return False


def read_transcript(path):
    """Return the cap flag, the brief, the first timestamp and the last assistant text of a transcript."""
    info = {"cap": False, "brief": "", "first_ts": None, "last_text": ""}
    with path.open(encoding="utf-8", errors="replace") as handle:
        for raw in handle:
            try:
                rec = json.loads(raw)
            except ValueError:
                continue
            if not isinstance(rec, dict):
                continue
            if is_cap_deny(rec):
                info["cap"] = True
            if info["first_ts"] is None and isinstance(rec.get("timestamp"), str):
                info["first_ts"] = parse_ts(rec["timestamp"])
            text = message_text(rec)
            if rec.get("type") == "user" and text and not info["brief"]:
                info["brief"] = text
            if rec.get("type") == "assistant" and text:
                info["last_text"] = text
    return info


def find_transcript(payload):
    """Return the subagent transcript path when it exists, else None."""
    explicit = payload.get("agent_transcript_path")
    if isinstance(explicit, str) and explicit:
        path = Path(explicit)
    else:
        main = payload.get("transcript_path")
        agent_id = payload.get("agent_id")
        if not isinstance(main, str) or not main or not agent_id:
            return None
        base = main[:-len(".jsonl")] if main.endswith(".jsonl") else main
        path = Path(base) / "subagents" / f"agent-{agent_id}.jsonl"
    return path if path.is_file() else None


def frontmatter_model(text):
    """Return the `model:` value from the block between the first two `---` lines."""
    lines = text.splitlines()
    dashes = [i for i, line in enumerate(lines) if line.strip() == "---"][:2]
    if len(dashes) < 2:
        return ""
    for line in lines[dashes[0] + 1:dashes[1]]:
        found = re.match(r"model:\s*(.*)$", line.strip())
        if found:
            return found.group(1).strip()
    return ""


def return_match(final):
    """Return the first LINE match among the lines of the final text, one pair of backticks stripped."""
    for line in final.splitlines():
        text = line.strip()
        if len(text) >= 2 and text[0] == text[-1] == "`":
            text = text[1:-1]
        found = LINE.match(text)
        if found:
            return found
    return None


def report_suffix(path):
    """Return the path from `.claude/reports/` on, normalised; empty when the path has no such part."""
    norm = path.strip().strip("'\"`").replace("\\", "/").lower()
    index = norm.find(".claude/reports/")
    return norm[index:] if index >= 0 else ""


def resolve(path, cwd, root):
    """Return the file a brief path names: relative paths are tried against cwd, then the root."""
    rel = Path(path.replace("\\", "/"))
    if rel.is_absolute():
        return rel
    for base in (cwd, root):
        if (base / rel).is_file():
            return base / rel
    return cwd / rel


def is_fresh(file, start):
    """True when the file is non-empty and its mtime is not older than the run (start minus the slack)."""
    if not file.is_file() or file.stat().st_size == 0:
        return False
    return start is None or file.stat().st_mtime >= start - SLACK_SECONDS


def contract_reason(payload):
    """Return the block reason for this stop, or None when the stop may go through."""
    if payload.get("stop_hook_active") is True:
        return None
    cwd = Path(str(payload.get("cwd") or os.getcwd()))
    root = Path(os.environ.get("CLAUDE_PROJECT_DIR") or cwd)
    agent_type = str(payload.get("agent_type") or "")
    if not re.fullmatch(r"[\w-]+", agent_type):
        return None
    agent_file = root / ".claude" / "agents" / f"{agent_type}.md"
    if not agent_file.is_file():
        return None
    agent_text = agent_file.read_text(encoding="utf-8", errors="replace")
    if "## Reporting" not in agent_text:
        return None
    line_mode = "haiku" in frontmatter_model(agent_text).lower()
    transcript = find_transcript(payload)
    scan = read_transcript(transcript) if transcript else None
    final = payload.get("last_assistant_message")
    if not isinstance(final, str) or not final.strip():
        final = scan["last_text"] if scan else ""
    if transcript is None and not final.strip():
        return None
    if scan and scan["cap"]:
        return None
    match = return_match(final)
    if line_mode:
        return None if match else REASON_LINE
    if scan is None or not scan["brief"]:
        return None
    paths = RP.findall(scan["brief"])
    if not paths:
        return None
    start = scan["first_ts"]
    stale = [p for p in paths if not is_fresh(resolve(p, cwd, root), start)]
    candidate = stale[-1] if stale else paths[-1]
    if match is None:
        return REASON_NO_LINE.format(path=candidate)
    report = match.group("report")
    suffix = report_suffix(report) if report != "-" else ""
    matched = [p for p in paths if suffix and report_suffix(p) == suffix]
    if not matched:
        return REASON_WRONG_PATH.format(got=report, path=candidate)
    target = matched[-1]
    if not is_fresh(resolve(target, cwd, root), start):
        return REASON_MISSING.format(path=target)
    return None


def main():
    """Read the payload from stdin and print a block decision when the contract is broken."""
    try:
        payload = json.loads(sys.stdin.read())
        if not isinstance(payload, dict):
            return 0
        reason = contract_reason(payload)
    except Exception:
        return 0
    if reason:
        print(json.dumps({"decision": "block", "reason": reason}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
