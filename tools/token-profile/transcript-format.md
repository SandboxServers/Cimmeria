# Claude Code transcript format, as the profiler reads it

> Type: reference. Audience: whoever works on `tools/token-profile/`.
> Observed 2026-10-03 over 911 local transcript files (Claude Code 2.1.x, 2026-09-14 to 2026-10-03). Issue [#957](https://github.com/SandboxServers/Cimmeria/issues/957). Ledger: [docs/analysis/token-usage/](../../docs/analysis/token-usage/README.md).

Claude Code's transcript format is not a published contract, and it changes between versions. This page records the shapes the profiler depends on. When the ingest meets a shape that isn't listed here, it records it in `unknown_shapes` and fails the run (see [Unknown shapes](#unknown-shapes)); update this page and the ingest together.

The tables these shapes land in are defined in [`schema.sql`](schema.sql). How a request is mapped to a PR is in [`attribution.md`](attribution.md).

## Where transcripts live

| Path under `~/.claude/projects/` | Holds |
|---|---|
| `<project-dir>/<session-id>.jsonl` | One top-level session. `<project-dir>` is the session's starting cwd with separators replaced by `-`, so a session started inside `.claude/worktrees/<name>` gets its own directory (`C--Users-…-Cimmeria--claude-worktrees-<name>`). Ingest every directory whose name starts with the repo's. |
| `<project-dir>/<session-id>/subagents/agent-<agent-id>.jsonl` | One subagent or teammate spawned by that session. |
| `<project-dir>/<session-id>/subagents/agent-<agent-id>.meta.json` | Its definition: `agentType`, `customAgentType` (the `.claude/agents/<x>.md` it ran), `name`, `model`, `taskKind`, `teamName`, `spawnDepth`, `requestShape`, `permissionMode`. Missing for some older agents; set `agents.meta_missing`. |
| `<project-dir>/<session-id>/tool-results/` | Tool results too large to inline. The transcript keeps a short "Output too large … saved to" stub, so `result_chars` counts the stub, and `tool_calls.result_persisted` is set. |

Transcripts are kept 365 days on this machine, so the "before" for any change in the ledger is still on disk.

## Record types

Every line is one JSON object with a `type`. Observed counts over the 911 files:

| `type` | Count | Used for |
|---|---:|---|
| `assistant` | 175,075 | `requests`, `tool_calls` (the `tool_use` blocks) |
| `attachment` | 112,503 | `queued_command` triggers; the rest is context, see below |
| `user` | 105,345 | `triggers`, tool results |
| `queue-operation` | 7,659 | not used |
| `mode`, `permission-mode`, `atis-latch`, `last-prompt`, `ai-title`, `bridge-session` | ~33k | not used |
| `pr-link` | 4,713 | `pr_links` |
| `system` | 1,158 | `compactions` (`compact_boundary`), scheduled-task triggers |
| `file-history-delta`, `file-history-snapshot` | ~1.6k | not used |
| `relocated`, `worktree-state` | 137 each | session worktree |
| `cost-state` | 118 | `cost_states` |
| `fork-context-ref` | 7 | `sessions.forked_from` |
| `started`, `result`, `launched`, `frame-link`, `artifact-autoreact-ledger`, `artifact-comment-monitor` | <100 | not used |

"Not used" types are still known: they are skipped, not reported as unknown.

### `assistant`: one API request, written more than once

An assistant record carries `requestId`, `message.model`, `message.usage`, `timestamp`, `sessionId`, `agentId` (in subagent files), `gitBranch`, `cwd`, `version` (the Claude Code version) and `isApiErrorMessage`.

**A request is written as several records.** The first is a streaming partial whose `output_tokens` is far too small; later records repeat the same `requestId` with the final counts. Deduplicate by `requestId` and **keep the last record in file order**. Keeping the first undercounted output four-fold in the issue's original numbers (18M against the real 70.8M). The fixture `dedupe-final` fails a first-record dedupe.

`message.usage` fields:

| Field | Column | Notes |
|---|---|---|
| `input_tokens` | `input_tokens` | uncached input only |
| `output_tokens` | `output_tokens` | includes thinking |
| `output_tokens_details.thinking_tokens` | `thinking_tokens` | **a subset of `output_tokens`**, never added on top. Absent on older records (0). |
| `cache_read_input_tokens` | `cache_read` | |
| `cache_creation.ephemeral_5m_input_tokens` | `cache_write_5m` | |
| `cache_creation.ephemeral_1h_input_tokens` | `cache_write_1h` | |
| `cache_creation_input_tokens` | (check only) | equals the sum of the two above |
| `server_tool_use.web_search_requests`, `.web_fetch_requests` | `web_search`, `web_fetch` | |
| `service_tier`, `speed`, `inference_geo`, `iterations`, `fallback_credit` | not stored | |

**API errors are not requests.** A failed call is written as an assistant record with `isApiErrorMessage: true` and model `<synthetic>` (91 of them observed, all with zero usage). It is not stored in `requests`; it only marks the next request in the same turn as a retry (rule R1).

Context size of a request is `input + cache_read + cache_write_5m + cache_write_1h`.

### `user`: triggers and tool results

A `user` record is either a tool result (its `message.content` is a list of `tool_result` blocks; `toolUseResult` and `sourceToolUseID` are set) or the start of a turn. Turn starts become `triggers` rows, classified below. Fields the classifier reads: `origin.kind`, `isMeta`, `isCompactSummary`, `promptSource`, `scheduledTaskId`, and the leading tag of the text content.

`origin.kind` is absent on records from older versions, and on teammate messages even now, so the content tag is the fallback.

### `attachment`

`attachment.type` values seen: `total_tokens_reminder`, `deferred_tools_record`, `edited_text_file`, `environment`, `queued_command`, `prompt_snapshot`, `deferred_tools_delta`, `bash_output_audience_note`, `nested_memory`, `date`, `remote_session_change`, `mcp_instructions_delta`, `skill_listing`, `model`, `session_context`, `instructions`, `agent_listing_delta`, `silent_turn_reminder`, `credential_org`, `auto_mode`, `batching_reminder_sent`, `read_truncation_notice`, `thinking_drop`, `file`, `hook_additional_context`, `compact_file_reference`, `task_status`, `command_permissions`, `inlined_image_paths`, `hook_system_message`, `selected_lines_in_ide`, `opened_file_in_ide`.

Only `queued_command` creates a trigger. It is something delivered into a turn already running, between tool calls: a task notification, a queued human prompt or a peer message. Its fields are `prompt`, `commandMode` (`task-notification`, `prompt`, absent), `origin.kind`, `source_uuid` and `timestamp`.

### `system`

Subtypes seen: `turn_duration`, `away_summary`, `local_command`, `bridge_status`, `informational`, `compact_boundary`, `agents_killed`, `scheduled_task_fire`, `model_refusal_fallback`.

`compact_boundary` marks a compaction. Its `compactMetadata` holds `trigger` (`auto` or `manual`), `preTokens`, `postTokens`, `cumulativeDroppedTokens` and `durationMs`. The next `user` record has `isCompactSummary: true` and carries the summary; it is the trigger of the first request after the compaction (`compact_summary`). Only 6 compactions happened in the 911 files.

### `pr-link`

`{type, sessionId, prNumber, prUrl, prRepository, timestamp}`. Written when the session creates or views a PR. A session can link the same PR many times; 4,713 records in all.

### `cost-state`

Claude Code's own running total for the session: `totalCostUSD`, `hasUnknownModelCost`, `totalAPIDuration`, `totalToolDuration`, `totalDuration`, `totalLinesAdded`, `totalLinesRemoved`, `startTime`, and `modelUsage` keyed by model (`inputTokens`, `outputTokens`, `thinkingTokens`, `cacheReadInputTokens`, `cacheCreationInputTokens`, `webSearchRequests`, `costUSD`). Keep the last record per session. Its model keys can carry a suffix such as `claude-opus-5[1m]`. It does not split cache writes into 5m and 1h.

## Trigger classification

Every request belongs to one trigger. Normally that is the turn start it follows: the nearest turn-starting `user` record or `queued_command` attachment before it in the same transcript. Tool results continue a turn; they never start one. A `queued_command` is something delivered into a turn already running, so the requests after it, up to the next turn start, belong to it rather than to the turn's original trigger: the injected event is what they were spent on.

The rules are applied in order and the first match wins. R1 and R2 are overrides and are checked before the per-record rules, which would otherwise always match first. The rule id is stored in `triggers.rule`.

| Rule | Matches | `kind` |
|---|---|---|
| R1 | a request whose previous request in the same turn was an API error (`isApiErrorMessage`). The retry gets its own trigger, id `<request_id>:retry`. | `retry` |
| R2 | one `queued_command` delivery (consecutive `queued_command` attachments with no request between them) whose items would classify as different kinds under R3-R14 | `mixed` |
| R3 | `isCompactSummary` is true | `compact_summary` |
| R4 | `scheduledTaskId` is set, or the record follows a `system/scheduled_task_fire` | `scheduled_task` |
| R5 | `origin.kind == "task-notification"`, `commandMode == "task-notification"` or text starting `<task-notification>`, that contains `<event>` or a `Monitor event:` summary | `monitor_event` |
| R6 | as R5 without the monitor markers | `background_completion` |
| R7 | text containing `<teammate-message` whose JSON payload has `"type":"idle_notification"` | `idle_notification` |
| R8 | text containing `<teammate-message` otherwise | `teammate_message` |
| R9 | `origin.kind == "peer"` and text containing `<cross-session-message` | `cross_session_message` |
| R10 | `origin.kind == "peer"` otherwise (`<agent-message`, `senderTaskId`) | `agent_message` |
| R11 | `origin.kind == "coordinator"`, or the first turn start in a subagent file | `subagent_prompt` |
| R12 | text starting `<command-name>` or `<local-command-caveat>` | `local_command` |
| R13 | `origin.kind == "human"`, or no `origin`, not `isMeta`, and none of the tags above | `human_prompt` |
| R14 | `isMeta` and none of the above | `auxiliary` |
| R15 | anything else, such as an `origin.kind` this page does not list | `unknown` |

A `queued_command` is classified by the same rules applied to its `prompt` and `origin`.

Never store message text in `triggers`. `source_ref` holds the task id, the teammate id or the peer name only.

## Tool-call fingerprints

`tool_calls.fingerprint` is the only place tool input reaches the database, and it is built so a report can show it. Nothing from a command's arguments is kept unless it is on an allowlist, because any argument can carry a credential, a URL or a local path.

- **Bash and PowerShell:** drop leading `VAR=value` assignments and a leading `cd <dir> &&` (or `;`), then take the executable's base name (`C:\Python\python.exe` gives `python`). If the executable and its first argument form a pair on the allowlist, keep both, for example `cargo nextest`, `cargo clippy`, `git rebase`, `gh pr`, `sed -n`, `bash tools/build-lane/lane.sh`. Otherwise keep the executable alone: `curl https://user:pass@host` gives `curl`, `echo <token>` gives `echo`, `python C:\Users\...\x.py` gives `python`. The allowlist lives in the ingest, and adding to it is a reviewed change.
- **Read, Edit, Write, Grep, Glob:** the path made repo-relative by cutting everything up to and including the checkout root or `.claude/worktrees/<name>/`. A path outside a checkout becomes `<external>`.
- **MCP tools:** the tool name only. `mcp_server` is the segment after `mcp__`.
- **Everything else:** NULL.

The TP-01b privacy scrubber runs over every report anyway; the fixtures in `fixtures/` carry hostile values, including hostile second words, that must never come through.

## Unknown shapes

A record is unknown when its `type`, its `system` subtype or its `attachment.type` is not listed on this page, or when an `assistant` record that is not `<synthetic>` lacks `requestId` or `message.usage`. The ingest counts each unknown shape once per Claude Code version in `unknown_shapes` and exits non-zero unless `--allow-unknown` is given. Report totals always say how many records were unknown.
