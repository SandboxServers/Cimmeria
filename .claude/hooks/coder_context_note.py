#!/usr/bin/env python3
"""PostToolUse note for packet-coder: past 75k context, commit passing work before the cap denies every tool.

Reads the PostToolUse payload from stdin. When the caller is packet-coder and its latest request
crosses NOTE_AT below CAP, it prints one `additionalContext` note, once per dispatch. It never
denies a tool and never asks for a report (D-HT3), and it reads the cap but never changes it.
Every other path, and every error, exits 0 with no output (fail open).
"""

import json
import os
import sys
from pathlib import Path

NOTE_AT = 75_000
CAP = int(os.environ.get("HAIKU_CONTEXT_CAP", "90000"))  # read only: the user-level cap owns this value (D-HT3)
MARKER = "coder_context_note:"
USAGE_KEYS = ("input_tokens", "cache_read_input_tokens", "cache_creation_input_tokens")


def subagent_transcript(payload):
    """The subagent transcript named by the payload, or None."""
    given = payload.get("agent_transcript_path")
    if given and Path(given).is_file():
        return Path(given)
    agent_id = payload.get("agent_id")
    main = str(payload.get("transcript_path") or "")
    if not agent_id or not main:
        return None
    base = main[: -len(".jsonl")] if main.endswith(".jsonl") else main
    for name in (f"agent-{agent_id}.jsonl", f"{agent_id}.jsonl"):
        candidate = Path(base) / "subagents" / name
        if candidate.is_file():
            return candidate
    return None


def request_contexts(path):
    """Context size per request id, in first-seen order: the max usage total over each id's lines."""
    sizes = {}
    with open(path, encoding="utf-8", errors="replace") as handle:
        for number, line in enumerate(handle):
            if '"usage"' not in line:
                continue
            try:
                record = json.loads(line)
            except ValueError:
                continue
            message = record.get("message") if isinstance(record, dict) else None
            if not isinstance(message, dict) or record.get("type") != "assistant":
                continue
            usage = message.get("usage")
            if not isinstance(usage, dict):
                continue
            key = message.get("id") or ("line", number)
            total = sum(int(usage.get(name) or 0) for name in USAGE_KEYS)
            sizes[key] = max(sizes.get(key, 0), total)
    return list(sizes.values())


def note(last):
    """The additionalContext text for a request at `last` tokens."""
    return (
        f"{MARKER} your context is {last:,} tokens; at {CAP:,} the cap denies every tool. "
        "Finish only the step you are on and run its check. As soon as a check passes, commit the "
        "passing files with the subject suffix (part N), before any other read or edit. Never commit a "
        "failing state. Write no report."
    )


def main():
    """Print the note when the latest request is the first one past NOTE_AT and still below CAP."""
    payload = json.load(sys.stdin)
    if not isinstance(payload, dict):
        return
    if "agent_type" in payload and payload["agent_type"] != "packet-coder":
        return
    path = subagent_transcript(payload)
    if path is None:
        return
    sizes = request_contexts(path)
    if not sizes:
        return
    last = sizes[-1]
    prev = sizes[-2] if len(sizes) > 1 else 0
    if not prev < NOTE_AT <= last < CAP:
        return
    context = {"hookSpecificOutput": {"hookEventName": "PostToolUse", "additionalContext": note(last)}}
    print(json.dumps(context))


if __name__ == "__main__":
    try:
        main()
    except Exception:  # fail open: a broken note must never block a tool
        pass
    sys.exit(0)
