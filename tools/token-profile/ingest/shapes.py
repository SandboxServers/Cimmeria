"""The transcript shapes transcript-format.md lists. Anything else is unknown."""

RECORD_TYPES = {
    "assistant", "attachment", "user", "queue-operation", "mode", "permission-mode", "atis-latch",
    "last-prompt", "ai-title", "bridge-session", "pr-link", "system", "file-history-delta",
    "file-history-snapshot", "relocated", "worktree-state", "cost-state", "fork-context-ref",
    "started", "result", "launched", "frame-link", "artifact-autoreact-ledger",
    "artifact-comment-monitor",
}

SYSTEM_SUBTYPES = {
    "turn_duration", "away_summary", "local_command", "bridge_status", "informational",
    "compact_boundary", "agents_killed", "scheduled_task_fire", "model_refusal_fallback",
}

ATTACHMENT_TYPES = {
    "total_tokens_reminder", "deferred_tools_record", "edited_text_file", "environment",
    "queued_command", "prompt_snapshot", "deferred_tools_delta", "bash_output_audience_note",
    "nested_memory", "date", "remote_session_change", "mcp_instructions_delta", "skill_listing",
    "model", "session_context", "instructions", "agent_listing_delta", "silent_turn_reminder",
    "credential_org", "auto_mode", "batching_reminder_sent", "read_truncation_notice",
    "thinking_drop", "file", "hook_additional_context", "compact_file_reference", "task_status",
    "command_permissions", "inlined_image_paths", "hook_system_message", "selected_lines_in_ide",
    "opened_file_in_ide",
}

SYNTHETIC_MODEL = "<synthetic>"


def unknown_shape(rec):
    """The shape name if rec is not a shape the contract lists, else None."""
    t = rec.get("type")
    if not isinstance(t, str) or t not in RECORD_TYPES:
        return t if isinstance(t, str) else "<no type>"
    if t == "system":
        sub = rec.get("subtype")
        if sub not in SYSTEM_SUBTYPES:
            return f"system/{sub}"
    elif t == "attachment":
        a = rec.get("attachment")
        at = a.get("type") if isinstance(a, dict) else None
        if at not in ATTACHMENT_TYPES:
            return f"attachment/{at}"
    elif t == "assistant":
        msg = rec.get("message")
        if not isinstance(msg, dict):
            return "assistant/no-message"
        if msg.get("model") != SYNTHETIC_MODEL:
            if not rec.get("requestId"):
                return "assistant/no-requestId"
            if not isinstance(msg.get("usage"), dict):
                return "assistant/no-usage"
    return None
