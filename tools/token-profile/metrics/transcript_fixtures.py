"""Writes synthetic harness transcripts and agent files under a temp folder, for the metrics tests.

Not a test module: the tests import it. Every path it writes is relative to the folder the test gives it.
"""

import json
from pathlib import Path

FIXTURE_PREFIX = "C--fixture-repo"


def _write_jsonl(path: Path, recs) -> None:
    """Write `recs` as JSON lines to `path`, creating its folder."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("".join(json.dumps(r) + "\n" for r in recs), encoding="utf-8")


def write_subagent(root, session_id, agent_id, agent_type, brief, steps, final_text, ts="2026-10-10T10:00:00Z"):
    """Write one subagent transcript and its .meta.json, and return the transcript's path.

    `steps` is a list of (tool_name, tool_input, result_text, context_tokens). Each step is an assistant record with
    a tool_use and its usage (input_tokens = context_tokens, output_tokens = 100), then a user record with the
    tool_result. A final assistant text record follows the steps.
    """
    path = Path(root) / FIXTURE_PREFIX / session_id / "subagents" / f"agent-{agent_id}.jsonl"
    recs = [{"type": "user", "timestamp": ts, "message": {"role": "user", "content": brief}}]
    for i, (name, tool_input, result_text, context_tokens) in enumerate(steps):
        tid = f"toolu_{agent_id}_{i}"
        recs.append({"type": "assistant", "timestamp": ts, "message": {
            "id": f"msg_{agent_id}_{i}", "role": "assistant",
            "content": [{"type": "tool_use", "id": tid, "name": name, "input": tool_input}],
            "usage": {"input_tokens": context_tokens, "output_tokens": 100}}})
        recs.append({"type": "user", "timestamp": ts, "message": {"role": "user", "content": [
            {"type": "tool_result", "tool_use_id": tid, "content": result_text}]}})
    recs.append({"type": "assistant", "timestamp": ts, "message": {
        "id": f"msg_{agent_id}_final", "role": "assistant",
        "content": [{"type": "text", "text": final_text}],
        "usage": {"input_tokens": 0, "output_tokens": 100}}})
    _write_jsonl(path, recs)
    path.with_name(f"agent-{agent_id}.meta.json").write_text(json.dumps({"agentType": agent_type}), encoding="utf-8")
    return path


def write_main(root, session_id, records):
    """Write a main session transcript from `records` to <root>/<FIXTURE_PREFIX>/<session_id>.jsonl; return its path."""
    path = Path(root) / FIXTURE_PREFIX / f"{session_id}.jsonl"
    _write_jsonl(path, records)
    return path


def write_agent_file(repo, name, model, reporting=True):
    """Write <repo>/.claude/agents/<name>.md with the given model, and a "## Reporting" section when `reporting`."""
    path = Path(repo) / ".claude" / "agents" / f"{name}.md"
    path.parent.mkdir(parents=True, exist_ok=True)
    text = f"---\nname: {name}\nmodel: {model}\n---\n\nAgent body.\n"
    if reporting:
        text += "\n## Reporting\n\nWrite the report to the path in the brief.\n"
    path.write_text(text, encoding="utf-8")
    return path
