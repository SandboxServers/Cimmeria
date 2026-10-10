---
name: telemetry-triage
description: Investigate Cimmeria behavior from SigNoz telemetry: server, network, trace and client logs; what a player session did; whether an event fired; and profiling of Claude Code usage (tools, skills, agents, MCP calls, cost). Use when asked to "check SigNoz", "look at the logs", "what happened in that session", "did X fire on the colo", "mine the telemetry", "profile our Claude usage", or when the SigNoz MCP is down and you need ClickHouse directly. Also covers the logging rules a restored system must meet so it is debuggable from SigNoz alone.
---

# Telemetry triage

The colo SigNoz holds every service's OTLP logs, which you can query with the SigNoz MCP or with ClickHouse directly.

- Setup: [docs/operations/signoz-remote-access.md](../../../docs/operations/signoz-remote-access.md)
- Targets and filters: [docs/architecture/observability.md](../../../docs/architecture/observability.md)
- Claude Code export: [docs/operations/claude-code-telemetry.md](../../../docs/operations/claude-code-telemetry.md)

## Services (`service.name`)

| Service | What it holds |
|---|---|
| `cimmeria-server` | Game server logs (INFO and above, plus pinned targets) |
| `cimmeria-network` | Decoded wire traffic: `wire_inbound`, `wire_outbound` |
| `cimmeria-trace` | TRACE-level only |
| `cimmeria-client` | Client telemetry DLL events. Raw fields are in `attributes_string['fields']` as a JSON string; `method_name`, `entity_name` and `class_name` are separate keys |
| `claude-code` | Claude Code OTel events (see *Claude usage profiling*) |

## Query discipline

1. **Aggregate before you search.** For counts, distributions and "did X happen", use an aggregate query grouped by a key. Fetch bodies only for the few lines you need. In experiment G an aggregate needed 4 requests and 7k chars where a search needed 9 and 32.5k ([experiment-g.md](../../../docs/analysis/token-usage/experiment-g.md)).
2. **Never list field keys unfiltered.** Always pass `searchText`, because the key list is huge. Ambiguous keys need `attribute.` or `resource.` in front.
3. **Time:** the MCP takes epoch **milliseconds**, ClickHouse `timestamp` is epoch **nanoseconds**. Convert with code, not in your head. Times the owner quotes from Discord are US Central.
4. **Oversized results** (over ~25k tokens) land in a tool-results file. Condense it with a short script (`HH:MM:SS.mmm LEVEL scope | body | k=v`); never read the raw JSON.
5. **Negative filters match missing fields.** Write `key EXISTS AND key != 'x'`.

## Filters that work

- `body = 'wire_inbound' AND peer = '<ip:port>'`
- `method_name IN ('useAbility', 'chatJoin')`. Since NT-30, `msg_name` on `wire.in` is the Mercury message name (`cellMethod`, `baseMethod`), not the method name.
- `decoded CONTAINS '"update_id":0,'` (`update_id` itself isn't indexed)
- `witness_id = N` on `wire_outbound`
- `body NOT CONTAINS 'movement.validation_reject'` drops navmesh-reject noise
- `cast_id = C AND (entity_id = E OR source_id = E OR invoker_id = E)` finds one ability cast. Cast ids are unique per caster, not per server.

## Session anchors

| Anchor | Fields |
|---|---|
| `World entry: sending RESET_ENTITIES` | login: addr, entity_id, position |
| `Gate travel: sending RESET_ENTITIES` | |
| `player entered world` | cell side, entity_id only |
| `player session ended` | `disconnect_reason`: `logOff`, `duplicate_login` or `inactivity_timeout` |

**Normal, not faults:**

- An idle client sends `AUTHENTICATE` about 6 times a second, and `perfStats` every 15 s.
- A fresh client's first `avatarUpdateExplicit` has `update_id` 0 and `vel == pos`.

## ClickHouse fallback (MCP down, or a query the MCP can't express)

Use the colo SSH host alias from your SSH config; never write the address in the repo. From PowerShell, pipe the SQL in, with no heredoc:

```powershell
$sql = @'
SELECT toDateTime(intDiv(timestamp, 1000000000)) t, severity_text, scope_name, body
FROM signoz_logs.distributed_logs_v2
WHERE resources_string['service.name'] = 'cimmeria-server'
  AND timestamp > toUnixTimestamp(now() - INTERVAL 1 HOUR) * 1000000000
  AND body LIKE '%player session ended%'
ORDER BY timestamp DESC LIMIT 50
'@
$sql | ssh <colo-ssh-alias> 'docker exec -i signoz-clickhouse clickhouse-client -n'
```

- **Columns:** `timestamp` (ns), `severity_text`, `scope_name`, `body`, `attributes_string[...]`, `attributes_number[...]` and `resources_string['service.name']`.
- **Several statements:** end each with `;` (`-n` runs them all).
- **One sample of each message type:** `LIMIT 1 BY <expr>` with `FORMAT Vertical`.

## Claude usage profiling

Events are `attribute.event.name`: `api_request`, `tool_decision`, `tool_result`, `user_prompt`, `skill_activated`, `subagent_completed` and so on. Useful keys are `tool_name`, `skill.name`, `agent.name`, `mcp_server.name`, `mcp_tool.name`, `model` and `cost_usd`. The tool's input is in `tool_input` as JSON, cut at 512 chars. For transcript-based token reports and PR stats, use [docs/guides/token-profiling.md](../../../docs/guides/token-profiling.md) instead.

**With the MCP:** run an aggregate with `count`, filter `service.name = 'claude-code'` and time range `30d`. Group by one of:

| Group by | Add filter |
|---|---|
| `attribute.event.name` | (none) |
| `tool_name` | `tool_name EXISTS` |
| `skill.name` | `skill.name EXISTS` |
| `agent.name` | `agent.name EXISTS` |
| `mcp_server.name, mcp_tool.name` | `mcp_tool.name EXISTS` |

**With ClickHouse:**

```sql
-- cost by agent and model
SELECT attributes_string['agent.name'] a, attributes_string['model'] m, count() req,
       round(sum(toFloat64OrZero(attributes_string['cost_usd'])) + sum(attributes_number['cost_usd']), 0) usd
FROM signoz_logs.distributed_logs_v2
WHERE resources_string['service.name'] = 'claude-code' AND attributes_string['event.name'] = 'api_request'
GROUP BY a, m ORDER BY usd DESC LIMIT 25;

-- what shell calls are spent on (extend the multiIf with the patterns you care about)
WITH JSONExtractString(attributes_string['tool_input'], 'command') AS c
SELECT multiIf(
   c LIKE '%lane.sh%' OR c LIKE '%lane.ps1%', 'build-lane',
   c LIKE '%ship.%', 'ship',
   c LIKE '%worktree%', 'worktree',
   c LIKE '%gh pr checks%' OR c LIKE '%gh run%', 'gh-ci',
   c LIKE '%gh pr%', 'gh-pr',
   c LIKE '%ssh %', 'ssh-colo',
   c LIKE '%psql%' OR c LIKE '%reload-db%' OR c LIKE '%live-db%', 'db',
   c LIKE '%cargo %', 'cargo-direct',
   c LIKE 'grep%' OR c LIKE 'sed -n%' OR c LIKE 'cat %' OR c LIKE 'Get-Content%', 'read/search',
   c LIKE '%sleep%', 'sleep/poll',
   'other') cat,
 attributes_string['tool_name'] tool, count() n
FROM signoz_logs.distributed_logs_v2
WHERE resources_string['service.name'] = 'claude-code' AND attributes_string['event.name'] = 'tool_result'
  AND attributes_string['tool_name'] IN ('Bash', 'PowerShell')
GROUP BY cat, tool ORDER BY n DESC;
```

To see which script names show up most, select `extract(c, '([A-Za-z0-9_./-]+\\.py)')` and group by it.

**Privacy:** this data holds command lines and local paths. Commit only aggregates, never rows, paths, session ids or names.

## When you add or restore a system

Every restored system must be debuggable from SigNoz alone. Follow these, and add an `OTEL_FILTER` row and pin for each new target ([observability.md](../../../docs/architecture/observability.md)):

- [instrumentation-discipline.md](../../../docs/architecture/instrumentation-discipline.md): an info span per dispatch entrypoint, `event="..."` per state transition, no per-tick spans, enumerated metric labels, `account_id` and `player_id` on the event.
- [negative-logging-convention.md](../../../docs/architecture/negative-logging-convention.md): every expectation seam logs, with a LogCapture test.
