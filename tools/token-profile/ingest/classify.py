"""Trigger classification, rules R3-R15 of transcript-format.md.

R1 (retry) and R2 (mixed) depend on what came before a record, so the
transcript reader applies them; this module classifies one turn start.
Nothing returned here carries message text: source_ref is an id or a name.
"""

import re

TASK_ID = re.compile(r"<task-id>\s*([^<\s]+)\s*</task-id>")
TEAMMATE_ID = re.compile(r"<teammate-message\b[^>]*\bteammate_id=\"([^\"]+)\"")
FROM_NAME = re.compile(r"<cross-session-message\b[^>]*\bfrom-name=\"([^\"]+)\"")
AGENT_FROM = re.compile(r"<agent-message\b[^>]*\bfrom=\"([^\"]+)\"")
IDLE = re.compile(r"\"type\"\s*:\s*\"idle_notification\"")
SAFE_REF = re.compile(r"^[A-Za-z0-9_.:@-]{1,128}$")


def text_of(content):
    """The text of a message content: a string, or the text blocks of a list."""
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        return "\n".join(b.get("text", "") for b in content
                         if isinstance(b, dict) and b.get("type") == "text" and isinstance(b.get("text"), str))
    return ""


def _ref(value):
    """An id or a name, never free text."""
    return value if isinstance(value, str) and SAFE_REF.match(value) else None


def classify(text, origin=None, is_meta=False, is_compact_summary=False, scheduled=False,
             first_in_subagent=False, command_mode=None):
    """(kind, rule, source_ref) for one turn start or queued_command item."""
    origin = origin if isinstance(origin, dict) else {}
    okind = origin.get("kind")
    stripped = text.lstrip()

    if is_compact_summary:
        return "compact_summary", "R3", None
    if scheduled:
        return "scheduled_task", "R4", None
    if okind == "task-notification" or command_mode == "task-notification" \
            or stripped.startswith("<task-notification>"):
        m = TASK_ID.search(text)
        ref = _ref(m.group(1)) if m else None
        if "<event>" in text or "Monitor event:" in text:
            return "monitor_event", "R5", ref
        return "background_completion", "R6", ref
    if "<teammate-message" in text:
        m = TEAMMATE_ID.search(text)
        ref = _ref(m.group(1)) if m else None
        if IDLE.search(text):
            return "idle_notification", "R7", ref
        return "teammate_message", "R8", ref
    if okind == "peer":
        if "<cross-session-message" in text:
            m = FROM_NAME.search(text)
            return "cross_session_message", "R9", _ref(origin.get("name")) or (_ref(m.group(1)) if m else None)
        m = AGENT_FROM.search(text)
        ref = _ref(origin.get("senderTaskId")) or _ref(origin.get("from")) or _ref(origin.get("name")) \
            or (_ref(m.group(1)) if m else None)
        return "agent_message", "R10", ref
    if okind == "coordinator" or first_in_subagent:
        return "subagent_prompt", "R11", None
    if stripped.startswith("<command-name>") or stripped.startswith("<local-command-caveat>") \
            or stripped.startswith("<command-message>"):
        return "local_command", "R12", None
    if okind == "human" or (okind is None and not is_meta):
        return "human_prompt", "R13", None
    if is_meta:
        return "auxiliary", "R14", None
    return "unknown", "R15", None
