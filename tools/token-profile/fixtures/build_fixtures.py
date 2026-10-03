"""Build a synthetic Claude Code transcript tree for the token profiler.

The tree mimics ~/.claude/projects/ with one main session and one subagent,
covering every shape in transcript-format.md that the ingest must handle:
streaming partials, thinking tokens, every trigger kind (one request each at
least), an API error and its retry, a compaction, pr-link, cost-state and one
deliberately unknown record type. Values that must never reach
a report (local paths, credentials, tokens, private addresses) are planted in
tool inputs and results.

expected.json, written next to the tree, is what a correct ingest produces.
TP-01a's tests ingest this tree and compare; test_contract.py checks that the
fixture itself can tell a right ingest from a wrong one.

    python tools/token-profile/fixtures/build_fixtures.py <out-dir>
"""

import json
import sys
from pathlib import Path

PROJECT_DIR = "C--fixture-Cimmeria"
SESSION = "00000000-0000-4000-8000-000000000001"
AGENT = "a0fixture000000001"
WORKER_BRANCH = "feat/fixture-worker"
VERSION = "2.1.999"

# Strings that must never appear in any report built from this tree.
HOSTILE = [
    "C:\\Users\\Steve\\secret-project",
    "postgres://user:password@10.0.0.5/db",
    "Authorization: Bearer abc123def456ghi789",
    "https://user:pass@example.internal/",
    "--token supersecret",
    "10.0.0.5",
    "supersecret",
    "abc123def456ghi789",
]


def ts(minute, second=0):
    return f"2026-10-01T12:{minute:02d}:{second:02d}.000Z"


class Transcript:
    def __init__(self, agent_id=None, branch="main"):
        self.lines = []
        self.agent_id = agent_id
        self.branch = branch
        self.n = 0
        self.parent = None

    def _base(self, kind, when):
        self.n += 1
        uuid = f"{'s' if self.agent_id else 'm'}-{self.n:04d}"
        rec = {
            "type": kind,
            "uuid": uuid,
            "parentUuid": self.parent,
            "timestamp": when,
            "sessionId": SESSION,
            "version": VERSION,
            "gitBranch": self.branch,
            "cwd": "C:\\Users\\Steve\\source\\projects\\Cimmeria",
            "isSidechain": bool(self.agent_id),
            "userType": "external",
            "entrypoint": "cli",
        }
        if self.agent_id:
            rec["agentId"] = self.agent_id
        self.parent = uuid
        return rec

    def add(self, rec):
        self.lines.append(rec)
        return rec

    def user_text(self, when, text, **extra):
        rec = self._base("user", when)
        rec["message"] = {"role": "user", "content": text}
        rec.update(extra)
        return self.add(rec)

    def tool_result(self, when, tool_use_id, text):
        rec = self._base("user", when)
        rec["message"] = {
            "role": "user",
            "content": [{"type": "tool_result", "tool_use_id": tool_use_id, "content": text}],
        }
        rec["toolUseResult"] = {"stdout": text}
        rec["sourceToolUseID"] = tool_use_id
        return self.add(rec)

    def assistant(self, when, request_id, usage, content=None, model="claude-opus-5-5"):
        rec = self._base("assistant", when)
        rec["requestId"] = request_id
        rec["message"] = {
            "id": "msg_" + request_id,
            "role": "assistant",
            "model": model,
            "content": content or [{"type": "text", "text": "ok"}],
            "usage": usage,
        }
        return self.add(rec)

    def attachment(self, when, attachment):
        rec = self._base("attachment", when)
        rec["attachment"] = attachment
        return self.add(rec)

    def system(self, when, subtype, **extra):
        rec = self._base("system", when)
        rec["subtype"] = subtype
        rec["isMeta"] = False
        rec.update(extra)
        return self.add(rec)

    def raw(self, rec):
        return self.add(rec)


def usage(inp, out, think, read, w5, w1h):
    u = {
        "input_tokens": inp,
        "output_tokens": out,
        "cache_read_input_tokens": read,
        "cache_creation_input_tokens": w5 + w1h,
        "cache_creation": {
            "ephemeral_5m_input_tokens": w5,
            "ephemeral_1h_input_tokens": w1h,
        },
        "service_tier": "standard",
        "server_tool_use": {"web_search_requests": 0, "web_fetch_requests": 0},
    }
    if think is not None:
        u["output_tokens_details"] = {"thinking_tokens": think}
    return u


def request(t, when, rid, final, partial_output=3, content=None, model="claude-opus-5-5"):
    """Write a streaming partial, then the final record, for one request."""
    partial = dict(final)
    partial["output_tokens"] = partial_output
    partial.pop("output_tokens_details", None)
    t.assistant(when, rid, partial, content=[{"type": "text", "text": ""}], model=model)
    t.assistant(when, rid, final, content=content, model=model)


def build(out):
    out = Path(out)
    proj = out / PROJECT_DIR
    sub = proj / SESSION / "subagents"
    sub.mkdir(parents=True, exist_ok=True)
    expected = {"requests": {}, "triggers": {}, "compactions": 1, "pr_links": [4242],
                "unknown_shapes": ["future-record-type"], "hostile": HOSTILE,
                "cost_state_total_usd": 12.5}

    def expect(rid, kind, rule, u, agent=None):
        expected["requests"][rid] = {
            "trigger_kind": kind, "rule": rule, "agent_id": agent,
            "input_tokens": u["input_tokens"], "output_tokens": u["output_tokens"],
            "thinking_tokens": (u.get("output_tokens_details") or {}).get("thinking_tokens", 0),
            "cache_read": u["cache_read_input_tokens"],
            "cache_write_5m": u["cache_creation"]["ephemeral_5m_input_tokens"],
            "cache_write_1h": u["cache_creation"]["ephemeral_1h_input_tokens"],
        }

    m = Transcript()
    m.raw({"type": "permission-mode", "permissionMode": "default", "sessionId": SESSION})

    # Turn 1: a human prompt, a Bash call with hostile arguments, two requests.
    m.user_text(ts(0), "Run the tests", origin={"kind": "human"}, promptSource="typed")
    u = usage(5, 400, 120, 60000, 2000, 0)
    bash = {"type": "tool_use", "id": "toolu_fixture_bash", "name": "Bash", "input": {
        "command": "cd C:\\Users\\Steve\\secret-project && DATABASE_URL=postgres://user:password@10.0.0.5/db "
                   "cargo nextest run --token supersecret",
        "description": "Run tests"}}
    request(m, ts(0, 5), "req_A", u, content=[bash])
    expect("req_A", "human_prompt", "R13", u)
    m.tool_result(ts(1), "toolu_fixture_bash",
                  "Authorization: Bearer abc123def456ghi789\nfetching https://user:pass@example.internal/\n" + "x" * 5000)
    u = usage(3, 50, 0, 62000, 500, 0)
    request(m, ts(1, 5), "req_B", u)
    expect("req_B", "human_prompt", "R13", u)
    m.raw({"type": "pr-link", "sessionId": SESSION, "prNumber": 4242,
           "prUrl": "https://github.com/SandboxServers/Cimmeria/pull/4242",
           "prRepository": "SandboxServers/Cimmeria", "timestamp": ts(1, 10)})

    # Turn 2: background completion of the worker agent.
    m.user_text(ts(10), "<task-notification>\n<task-id>" + AGENT + "</task-id>\n<status>completed</status>\n"
                "<summary>Agent \"fixture worker\" completed</summary>\n</task-notification>",
                origin={"kind": "task-notification"}, promptSource="system")
    u = usage(4, 30, None, 64000, 300, 0)  # older record: no output_tokens_details
    request(m, ts(10, 3), "req_C", u)
    expect("req_C", "background_completion", "R6", u)

    # Turn 3: monitor event (cache expired: large 5m write after a long gap).
    m.user_text(ts(30), "<task-notification>\n<task-id>bmonitor01</task-id>\n<summary>Monitor event: \"CI checks\""
                "</summary>\n<event>checks passed</event>\n</task-notification>",
                origin={"kind": "task-notification"})
    u = usage(4, 20, 0, 0, 64500, 0)
    request(m, ts(30, 2), "req_D", u)
    expect("req_D", "monitor_event", "R5", u)

    # Turn 4: teammate idle notification (no origin field, as observed).
    m.user_text(ts(31), "Another Claude session sent a message:\n<teammate-message teammate_id=\"fixture-worker\">\n"
                + json.dumps({"type": "idle_notification", "from": "fixture-worker", "idleReason": "available"})
                + "\n</teammate-message>")
    u = usage(4, 10, 0, 64600, 100, 0)
    request(m, ts(31, 2), "req_E", u)
    expect("req_E", "idle_notification", "R7", u)

    # Turn 5: cross-session message from a peer session.
    m.user_text(ts(32), "Another Claude session sent a message:\n<cross-session-message from=\"uds:fixture\" "
                "from-name=\"cimmeria-zz\">FYI</cross-session-message>",
                origin={"kind": "peer", "name": "cimmeria-zz", "body": "FYI"}, isMeta=True)
    u = usage(4, 15, 0, 64700, 100, 0)
    request(m, ts(32, 2), "req_F", u)
    expect("req_F", "cross_session_message", "R9", u)

    # Turn 6: human prompt; mid-turn a queued task notification arrives.
    m.user_text(ts(33), "Check the PR", origin={"kind": "human"})
    u = usage(4, 60, 10, 64800, 200, 0)
    gh = {"type": "tool_use", "id": "toolu_fixture_gh", "name": "Bash",
          "input": {"command": "gh pr checks 4242", "description": "Check CI"}}
    request(m, ts(33, 2), "req_G", u, content=[gh])
    expect("req_G", "human_prompt", "R13", u)
    m.tool_result(ts(33, 30), "toolu_fixture_gh", "all checks passed")
    m.attachment(ts(33, 31), {"type": "queued_command", "commandMode": "task-notification",
                              "prompt": "<task-notification>\n<task-id>bshell01</task-id>\n<status>completed"
                                        "</status>\n</task-notification>",
                              "source_uuid": "q-0001", "timestamp": ts(33, 31)})
    u = usage(4, 25, 0, 65000, 150, 0)
    request(m, ts(33, 35), "req_H", u)
    expect("req_H", "background_completion", "R6", u)

    # Turn 7: a scheduled task fires.
    m.system(ts(34), "scheduled_task_fire", content="Scheduled task fired", taskId="sched01", cron="0 * * * *")
    m.user_text(ts(34, 1), "Run the weekly report", scheduledTaskId="sched01", isMeta=True)
    u = usage(4, 30, 0, 65100, 100, 0)
    request(m, ts(34, 3), "req_J", u)
    expect("req_J", "scheduled_task", "R4", u)

    # Turn 8: a teammate message that is not an idle notification.
    m.user_text(ts(35), "Another Claude session sent a message:\n<teammate-message teammate_id=\"fixture-worker\" "
                "summary=\"question\">Which branch?</teammate-message>")
    u = usage(4, 30, 0, 65200, 100, 0)
    request(m, ts(35, 2), "req_K", u)
    expect("req_K", "teammate_message", "R8", u)

    # Turn 9: an agent message from a peer agent.
    m.user_text(ts(36), "<agent-message from=\"na10\">evidence ready</agent-message>",
                origin={"kind": "peer", "from": "na10", "senderTaskId": "a0peer", "name": "na10", "body": "x"},
                isMeta=True)
    u = usage(4, 30, 0, 65300, 100, 0)
    request(m, ts(36, 2), "req_L", u)
    expect("req_L", "agent_message", "R10", u)

    # Turn 10: a local slash command.
    m.user_text(ts(37), "<command-name>/model</command-name>\n<command-message>model</command-message>\n"
                "<command-args></command-args>")
    u = usage(4, 30, 0, 65400, 100, 0)
    request(m, ts(37, 2), "req_M", u)
    expect("req_M", "local_command", "R12", u)

    # Turn 11: harness-injected meta content with no origin.
    m.user_text(ts(37, 30), "<system-reminder>context</system-reminder>", isMeta=True)
    u = usage(4, 30, 0, 65500, 100, 0)
    request(m, ts(37, 32), "req_N", u)
    expect("req_N", "auxiliary", "R14", u)

    # Turn 12: hostile second words in shell commands, then an API error and a retry.
    m.user_text(ts(38), "Fetch the thing", origin={"kind": "human"})
    hostile_cmds = [
        ("toolu_fixture_curl", "curl https://user:pass@example.internal/"),
        ("toolu_fixture_echo", "echo abc123def456ghi789"),
        ("toolu_fixture_py", "C:\\Python\\python.exe C:\\Users\\Steve\\secret-project\\x.py"),
    ]
    u = usage(4, 70, 0, 65600, 100, 0)
    request(m, ts(38, 2), "req_O", u, content=[
        {"type": "tool_use", "id": tid, "name": "Bash", "input": {"command": cmd}} for tid, cmd in hostile_cmds])
    expect("req_O", "human_prompt", "R13", u)
    for tid, _ in hostile_cmds:
        m.tool_result(ts(38, 5), tid, "done")
    err = m.assistant(ts(38, 6), "req_ERR", usage(0, 0, 0, 0, 0, 0), model="<synthetic>",
                      content=[{"type": "text", "text": "API Error: overloaded"}])
    err["isApiErrorMessage"] = True
    u = usage(4, 40, 0, 65700, 100, 0)
    request(m, ts(38, 20), "req_R", u)
    expect("req_R", "retry", "R1", u)
    expected["requests"]["req_R"]["trigger_id"] = "req_R:retry"

    # Turn 13: one delivery carrying a task notification and a peer message.
    m.user_text(ts(39), "Keep going", origin={"kind": "human"})
    u = usage(4, 20, 0, 65800, 100, 0)
    request(m, ts(39, 2), "req_P", u, content=[{"type": "tool_use", "id": "toolu_fixture_ls", "name": "Glob",
                                                "input": {"pattern": "*.md"}}])
    expect("req_P", "human_prompt", "R13", u)
    m.tool_result(ts(39, 3), "toolu_fixture_ls", "a.md")
    m.attachment(ts(39, 4), {"type": "queued_command", "commandMode": "task-notification",
                             "prompt": "<task-notification>\n<task-id>bshell02</task-id>\n</task-notification>",
                             "source_uuid": "q-0002", "timestamp": ts(39, 4)})
    m.attachment(ts(39, 4), {"type": "queued_command", "commandMode": "prompt", "origin": {"kind": "peer"},
                             "prompt": "<cross-session-message from=\"uds:fixture\">hi</cross-session-message>",
                             "source_uuid": "q-0003", "timestamp": ts(39, 4)})
    u = usage(4, 20, 0, 65900, 100, 0)
    request(m, ts(39, 6), "req_Q", u)
    expect("req_Q", "mixed", "R2", u)

    # Turn 14: an origin kind this contract does not know.
    m.user_text(ts(39, 30), "something new", origin={"kind": "future-origin"})
    u = usage(4, 20, 0, 66000, 100, 0)
    request(m, ts(39, 32), "req_U", u)
    expect("req_U", "unknown", "R15", u)

    # Compaction, then the summary starts the next turn.
    m.system(ts(40), "compact_boundary", content="Conversation compacted", level="info",
             compactMetadata={"trigger": "auto", "preTokens": 65200, "postTokens": 9000,
                              "cumulativeDroppedTokens": 56200, "durationMs": 30000})
    m.user_text(ts(40, 30), "This session is being continued from a previous conversation...",
                isCompactSummary=True, isVisibleInTranscriptOnly=True)
    u = usage(9000, 40, 0, 0, 9100, 0)
    request(m, ts(40, 35), "req_I", u)
    expect("req_I", "compact_summary", "R3", u)

    # A record type this contract does not know.
    m.raw({"type": "future-record-type", "sessionId": SESSION, "payload": 1})
    m.raw({"type": "cost-state", "sessionId": SESSION, "totalCostUSD": 12.5, "hasUnknownModelCost": False,
           "totalLinesAdded": 10, "totalLinesRemoved": 2, "startTime": 0,
           "modelUsage": {"claude-opus-5-5": {"inputTokens": 1, "outputTokens": 1, "thinkingTokens": 0,
                                              "cacheReadInputTokens": 1, "cacheCreationInputTokens": 1,
                                              "webSearchRequests": 0, "costUSD": 12.5}}})

    # Subagent: an in-process teammate, 5m cache, one Read with a local path. Its own git call in its
    # worktree names the branch it works on (attribution.md § Work branch).
    s = Transcript(agent_id=AGENT, branch=WORKER_BRANCH)
    s.user_text(ts(2), "Implement the fixture packet", origin={"kind": "coordinator"}, isMeta=True)
    u = usage(6, 300, 200, 21000, 40000, 0)
    read = {"type": "tool_use", "id": "toolu_fixture_read", "name": "Read",
            "input": {"file_path": "C:\\Users\\Steve\\source\\projects\\Cimmeria\\.claude\\worktrees\\fx\\docs\\gap-analysis.md"}}
    status = {"type": "tool_use", "id": "toolu_fixture_git", "name": "Bash",
              "input": {"command": "git -C .claude/worktrees/fx status"}}
    request(s, ts(2, 5), "req_S1", u, content=[read, status])
    expect("req_S1", "subagent_prompt", "R11", u, agent=AGENT)
    s.tool_result(ts(2, 10), "toolu_fixture_read", "y" * 20000)
    s.tool_result(ts(2, 11), "toolu_fixture_git", f"On branch {WORKER_BRANCH}\nnothing to commit\n")
    u = usage(3, 900, 0, 61000, 20000, 0)
    request(s, ts(9, 0), "req_S2", u)  # 7 minutes idle: the 5m cache was rewritten
    expect("req_S2", "subagent_prompt", "R11", u, agent=AGENT)

    meta = {"agentType": "fixture-worker", "customAgentType": "rust-gameserver-dev", "name": "fixture-worker",
            "spawnDepth": 0, "requestShape": "background", "model": "opus", "taskKind": "in_process_teammate"}

    write_jsonl(proj / f"{SESSION}.jsonl", m.lines)
    write_jsonl(sub / f"agent-{AGENT}.jsonl", s.lines)
    (sub / f"agent-{AGENT}.meta.json").write_text(json.dumps(meta), encoding="utf-8")
    expected["tool_fingerprints"] = {
        "toolu_fixture_bash": "cargo nextest",
        "toolu_fixture_gh": "gh pr",
        "toolu_fixture_read": "docs/gap-analysis.md",
        "toolu_fixture_git": "git",
        "toolu_fixture_curl": "curl",
        "toolu_fixture_echo": "echo",
        "toolu_fixture_py": "python",
        "toolu_fixture_ls": None,
    }
    expected["agents"] = {AGENT: {"custom_agent_type": "rust-gameserver-dev", "branch": WORKER_BRANCH}}
    (out / "expected.json").write_text(json.dumps(expected, indent=1, sort_keys=True), encoding="utf-8")
    return out


def write_jsonl(path, records):
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        for r in records:
            f.write(json.dumps(r) + "\n")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    print(build(sys.argv[1]))
