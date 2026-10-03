# Experiment G: Ghidra MCP vs headless, SigNoz search vs aggregate

> Type: explanation. Audience: the #957 coordinator and anyone choosing an RE or log-mining tool.
> Packet: TP-08 (Wave 2 of [#957](https://github.com/SandboxServers/Cimmeria/issues/957)). Ledger: [README.md](README.md). Worknote: [worknotes/TP-08.md](worknotes/TP-08.md).
> Status: **complete** (2026-10-03). The tasks, answer key and scoring were committed before any controlled run; the [results](#results) and the [guidance](#guidance) were added afterwards. Git history of this file shows the pre-registered version.

## Question

The issue asks two tool-choice questions:

1. Is headless Ghidra (`analyzeHeadless.bat` with [Probe.java](../../../tools/re/ghidra-headless/README.md)) cheaper than the Ghidra MCP bridge for the same RE lookups, and at what cost in wall time and correctness?
2. Is SigNoz `signoz_aggregate_logs` cheaper than `signoz_search_logs` for the same log question?

Cost is never the only dependent variable. Each comparison is scored on result characters, tool calls, requests, wall time, correctness and completeness.

## Part 1: historical cut

Source: the profiler database ingested 2026-10-03 13:15Z, window 2026-09-13 to 2026-10-03, 100,080 tool calls. Result characters and exposure (characters x later requests in the same context) are context measures, not dollars, as the profiler's rules require.

Headless runs are not visible in the profiler's command fingerprints: only 27 of 115 probe runs fingerprint as `analyzeHeadless.bat`; the rest start with `cd` or a quoted path. They were found by a local scan of the transcripts for Bash or PowerShell calls containing `analyzeHeadless` and at least one Probe token, then joined to `tool_calls` by `tool_use_id`. Follow-up reads are the Bash, Read, Grep and PowerShell calls in the same transcript that name a probe output file.

### Ghidra

| Measure | Ghidra MCP | Headless probe |
|---|---:|---:|
| Calls | 1,871 MCP calls | 115 probe runs + 137 follow-up reads |
| Lookups | 1,871 (one per call) | 398 Probe tokens (p50 3 per run, p90 6, max 20) |
| Transcripts | 98 (2026-09-17 to 09-29) | 12 (2026-09-27 to 09-29) |
| Result characters | 3.04M | 0.10M from the runs + 0.50M from follow-up reads |
| Result characters per lookup (mean) | 1,624 | ~1,500 |
| Exposure | 248.5M | 7.0M + 44.6M |
| Exposure per lookup (mean) | 133k | ~130k |
| Errors | 23 (1.2%) | 3 runs (2.6%) |
| Wall time per call, p50 / p90 | 0.3 s / 1.2 s | 5.7 s / 11 s per run |

107 of 115 probe runs redirected their output to a file, so the run itself returned a few hundred characters (p50 252) and the agent then read or grepped the file. Per lookup, the two routes put about the same amount of text into context. Headless is not cheaper per lookup in practice; it is cheaper only when the agent greps the output narrowly instead of reading it whole.

Ghidra MCP result size by tool (characters per call):

| Tool | Calls | p50 | p90 | p99 | Max | Exposure |
|---|---:|---:|---:|---:|---:|---:|
| `decompile_function` | 675 | 926 | 6,455 | 28,217 | 36,565 | 145.9M |
| `get_xrefs_to` | 264 | 55 | 163 | 1,754 | 4,541 | 3.6M |
| `search_strings` | 168 | 399 | 3,913 | 19,174 | 24,468 | 25.1M |
| `read_memory` | 118 | 160 | 558 | 2,186 | 2,570 | 3.0M |
| `search_functions` | 91 | 74 | 1,161 | 7,320 | 16,508 | 2.2M |
| `disassemble_bytes` | 72 | 3,592 | 8,848 | 20,860 | 25,101 | 21.8M |
| `get_function_by_address` | 70 | 143 | 233 | 263 | 266 | 0.6M |
| `disassemble_function` | 64 | 1,056 | 8,401 | 23,968 | 26,860 | 22.4M |
| `get_function_callers` | 48 | 48 | 152 | 897 | 1,145 | 0.4M |
| `list_open_programs` | 44 | 419 | 419 | 419 | 419 | 1.2M |
| `list_tool_groups` | 5 | 10,231 | 10,231 | 10,231 | 10,231 | 4.5M |

Decompiles are 59% of Ghidra MCP characters and 59% of its exposure. `list_tool_groups` costs 10k characters a call and is never needed for the read-only tools.

Spend of the transcripts that used each route (estimated list-price USD, a plan-usage proxy, D-TP1). The tasks differ, so this is context, not a comparison:

| Transcripts | n | USD p50 | USD total | Requests p50 |
|---|---:|---:|---:|---:|
| Ghidra MCP only | 91 (81 subagents) | 7.2 | 1,149 | 95 |
| Headless only | 5 (all subagents) | 3.2 | 28 | 49 |
| Both | 7 (all subagents) | 9.0 | 84 | 120 |

### SigNoz

| Tool | Calls | p50 | p90 | p99 | Max | Total chars | Exposure | Errors |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `signoz_search_logs` | 396 | 2,976 | 28,942 | 45,694 | 49,673 | 3.58M | 446.1M | 1 |
| `signoz_aggregate_logs` | 324 | 831 | 4,274 | 9,899 | 17,616 | 0.56M | 60.5M | 14 (4.3%) |
| `signoz_get_field_keys` | 10 | 912 | 6,313 | 41,347 | 45,240 | 0.05M | 11.6M | 0 |
| All other SigNoz tools | 49 | | | | | 0.11M | 4.7M | 0 |

The issue's "about 10k characters per `search_logs` call" holds: the mean is 9,037. `search_logs` is 83% of SigNoz characters and 85% of SigNoz exposure. An aggregate call is 3.6x smaller at the median and 6.8x smaller at p90. Aggregates fail more often (14 errors against 1), and each failure costs a retry.

### Scale

Both tool families are small next to file reads. Of 23.95B exposure characters in the window, Ghidra (MCP and headless) is about 1.3%, SigNoz 2.2% and `Read` 25%. The same 58 SigNoz-using transcripts cost $1,393 in total. A better habit here saves real context in the sessions that use these tools, but it is not a workflow-wide lever like static context (TP-03) or read discipline (TP-07).

## Part 2: controlled runs (pre-registered)

### Setup

- **Ghidra MCP arm.** The bridge is reachable from this session with SGW.exe open (173,225 functions).
- **Headless arm.** The GUI holds the live project (`SGW.lock` present), and the probe needs exclusive access. The arm runs against a **copy** of the project (`SGW.gpr` + `SGW.rep`, 2.5 GB, last saved 2026-09-29) in a scratch folder, with `-noanalysis -readOnly`, one run at a time. A setup smoke run (`BLOCKS` only) took 27 s. The live project is never touched.
- **Agents.** Each run is one fresh `general-purpose` subagent (Opus, the same model as every other worker). It gets the task text below and its arm's tool rules, and is told not to read the repository, its docs or memory files, so the answer must come from the binary or the logs. The issue suggested `game-archaeology-specialist`; that agent is told to check `docs/` first, where the answers are, so it would measure doc lookup, not the tool.
- **Repetitions.** Two per arm (the issue suggested three; two keeps the packet inside its budget, and the spread between repetitions is reported).
- **Run order.** Drawn with `random.SystemRandom().shuffle` before any run: Ghidra **HL-2, MCP-2, MCP-1, HL-1**; SigNoz **AGG-2, AGG-1, SEARCH-1, SEARCH-2**. Ghidra runs go one at a time; each SigNoz run may overlap a Ghidra run.
- **Measures per run,** from the subagent's own transcript: tool calls by tool, result characters, requests, tokens and estimated USD, wall time from first to last record, then the score below.

### Ghidra tasks and answer key

The binary is `SGW.exe`. The answers come from [client-engine-sinks-and-seams.md § BigWorld message helper](../../reverse-engineering/findings/client-engine-sinks-and-seams.md#bigworld-message-helper). If both arms agree with each other and disagree with the doc, the doc is checked against the binary and the loser is fixed.

| Task | Prompt given to the agent | Expected evidence | Points |
|---|---|---|---:|
| G1 decompile | Decompile the function at `0x00a36460`. Give its calling convention and stack cleanup, the condition under which it drops a message, where it looks for message callbacks, and the function it calls as the default output. | `__thiscall`, `ret 0xc` (3 stack args: header, fmt, va_list); drops unless `header[0] + impl[0x3c] <= header[1]` (impl = `*this`); callbacks at `impl + 0x30`; default output `0x00a353b0` | 4 |
| G2 xrefs | List every code reference to `0x00a36460` with the function each sits in. Then count the call sites that call `0x00a35210`, and those that call `0x00a351d0`. | Two references, in `0x00a35210` and `0x00a36650`; 30 callers of `0x00a35210`; 14 callers of `0x00a351d0` | 3 |
| G3 strings | Find the string containing `Do you want to enter debugger` and the function that uses it. Then find the address of the table of pointers to the priority names `TRACE`, `DEBUG`, `INFO`, `NOTICE`, `WARNING`, `ERROR`, `CRITICAL`, `HACK`. | Used by `0x00a36900`; table at `0x01922380` | 2 |
| G4 dependent chain | Start at `0x00a351d0`. Follow its calls, one function at a time, until you reach the function at `0x00a36460`. Give each function on the path in order, and the format string passed in the final call. | `0x00a351d0` -> `0x00a36ac0` -> `0x00a36900` -> `0x00a36650` -> `0x00a36460`, four hops; format `"%s"` | 5 (one per hop, one for the format) |

Scoring: a point needs the exact value (address, count, condition). A count within 10% of the key earns half a point and is reported. A claimed fact with no tool evidence behind it in the transcript earns nothing. Total 14.

### SigNoz questions and answer key

Window **W** = 2026-09-29T00:00:00Z to 2026-09-30T00:00:00Z (epoch ms 1790640000000 to 1790726400000), `service.name = cimmeria-server`, all `deployment.environment = colo`. The window is closed, so the counts cannot change. The key was computed before the arms with `signoz_aggregate_logs`, and the S4 rows with one `signoz_search_logs` call of limit 4. That is the aggregate arm's own method, so a mismatch in the search arm is checked again by hand before it is scored.

| Q | Question | Expected answer | Points |
|---|---|---|---:|
| S1 | How many `player session ended` events, and how many for each `disconnect_reason`? | 28: `duplicate_login` 17, `inactivity_timeout` 11 | 3 |
| S2 | How many `player entered world` events, and how many distinct server builds (`service.version`) emitted them? | 115 events, 7 builds (the largest build has 41) | 2 |
| S3 | How many ERROR-severity logs, and which `client_target` emitted the most of them, with its count? | 270; `client.ui.cegui_log` 269 (the other is `client.ue3.log`, 1) | 3 |
| S4 | How many gate-travel world transitions (`Gate travel: sending RESET_ENTITIES for world transition`), to which worlds, and the UTC time of the first and last? | 4; `Castle` 3, `Castle_CellBlock` 1; first 11:22:45Z, last 19:04:59Z | 4 |

Total 12. Arm rules:

- **SEARCH:** `signoz_search_logs` only (plus `signoz_get_field_keys` with a `searchText` filter and `signoz_get_field_values` for discovery). Counts come from the returned rows; the agent may page and may condense a persisted result file with a script, as [the log-mining notes](../../../.claude/agent-memory/main-session/reference_signoz_log_mining.md) recommend.
- **AGG:** `signoz_aggregate_logs` and `signoz_execute_builder_query` first; `signoz_search_logs` only with `limit` 10 or less (for a field shape or a timestamp), plus the same discovery tools.

### Ghidra arm rules

- **MCP:** `mcp__ghidra__*` tools only; no shell access to Ghidra.
- **HL:** the headless probe only, through Bash or PowerShell, against the project copy; no `mcp__ghidra__*` tools. The agent is given the probe's token table and the exact command line. It may batch tokens, grep its output files, and run again for dependent hops.

## Results

All eight runs ran on 2026-10-03 in the pre-registered order, one fresh subagent each. Every agent kept to its arm's tools (checked in the transcripts). Measures come from each subagent's transcript: requests deduplicated by `requestId`, list-price USD from the profiler's 2026-10-03 price table, result characters as returned into context, and wall time from the first to the last record.

### Answer-key correction (G2)

All four Ghidra runs counted **51** call sites to `0x00a35210` and **41** to `0x00a351d0`, not the 30 and 14 in the key. A check with `get_function_callers` gives 32 and 36 distinct calling functions, so the doc's numbers are wrong whichever way "callers" is read; the project has gained functions since the doc was written. As the protocol says, the loser was fixed in this PR: [client-engine-sinks-and-seams.md](../../reverse-engineering/findings/client-engine-sinks-and-seams.md#bigworld-message-helper) and [address-map.md](../../reverse-engineering/address-map.md) now give both counts. Scored against the pre-registered key every run gets 12/14; against the corrected key every run gets 14/14.

### Ghidra

| Run (order) | Requests | Tool calls | Result chars | Wall time | USD | Score |
|---|---:|---|---:|---:|---:|---:|
| HL-2 (1st) | 9 | 8 Bash (3 probe runs) | 22,046 | 77 s | 0.58 | 14/14 |
| MCP-2 (2nd) | 7 | 14 Ghidra MCP + 1 ToolSearch | 18,706 | 50 s | 0.54 | 14/14 |
| MCP-1 (3rd) | 7 | 15 Ghidra MCP + 1 ToolSearch | 18,787 | 44 s | 0.53 | 14/14 |
| HL-1 (4th) | 9 | 8 Bash (4 probe runs) | 16,719 | 98 s | 0.59 | 14/14 |
| **MCP mean** | **7** | **15.5** | **18,747** | **47 s** | **0.535** | |
| **HL mean** | **9** | **8** | **19,383** | **88 s** | **0.585** | |

- **Correctness is the same.** Every run found every answer, including the four-hop chain and the `"%s"` format, and cited tool evidence for it.
- **Completeness differs a little, in both arms.** Only MCP-1 read all eight priority-name strings; HL-1 read three and the other two runs inferred the order from Ghidra's `PTR_s_TRACE_` label. All three said so in their answers.
- **Headless took 86% longer and 2 more requests.** Each dependent hop costs a JVM start, so the four-hop chain took 3-4 probe runs. The MCP agents issued several lookups per request, 7 requests in all.
- **Result characters are the same:** 18.7k against 19.4k, because the headless agents grepped their output files instead of reading them whole. This matches the historical cut (about 1.5k characters per lookup either way).
- **Cost follows requests, not result size.** Every run started at 61.5k-62.2k tokens of context, the static context TP-03 measures, so each extra request re-reads about 90k tokens, more than all of a run's tool results add up to over every later request. Headless was 9% more expensive.
- **No order effect is visible.** The first and last runs were both headless, and the last one was the slowest.
- **Probe usability bug.** A token with a comma (`VT:<addr>,<n>`, `I:<addr>,<n>`) loses its count when it passes through `analyzeHeadless.bat`, quoted or not; HL-1 hit this and a repro confirmed it. The [probe README](../../../tools/re/ghidra-headless/README.md) now says so. A warm probe run took 7 s, a cold one 27 s.

### SigNoz

| Run (order) | Requests | Tool calls | Result chars in context | Raw rows fetched | Wall time | USD | Score |
|---|---:|---|---:|---:|---:|---:|---:|
| AGG-2 (1st) | 4 | 7 aggregate, 1 search (limit 10) | 6,719 | none | 21 s | 0.40 | 12/12 |
| AGG-1 (2nd) | 4 | 4 aggregate, 1 builder, 1 search (limit 10) | 7,291 | none | 24 s | 0.41 | 12/12 |
| SEARCH-1 (3rd) | 9 | 4 search, 3 Bash | 32,435 | 402k chars | 36 s | 0.57 | 12/12 |
| SEARCH-2 (4th) | 9 | 4 search, 3 Bash | 32,540 | 402k chars | 34 s | 0.56 | 12/12 |

- **Correctness is the same.** All four runs answered all four questions exactly.
- **Search cost 39% more:** 9 requests against 4, 4.6x the characters in context, and 56% more wall time. Two of its four calls (the 115 and 270 rows) were too big to show inline. They were saved to files (104k and 298k characters) and counted with a script; the search agents followed the documented habit. Without that habit the rows would have entered context.
- **The aggregate arm checked itself.** Both aggregate runs compared each grouped count with an ungrouped total, and one also counted ERROR logs with no `client_target` (0). The search arm's counts are only right because each result fitted in one page; at this window's 8,159 WARN rows, search would need paging.
- **A search with a small limit is still the right tool for rows.** Both aggregate runs got S4's timestamps from one `search_logs` call with limit 10 (3.8k characters).

### Limits

- Two runs per arm on small task sets. The direction is the same in every pair, but the size of the gap is not tight.
- Agents were told which route to use, so this measures the tools, not an agent's choice of tool.
- The Ghidra tasks all sit in one subsystem, and the decompiles are small. A task set heavy in large decompiles favours whichever route avoids reading them whole. That is the headless route only if its agent greps narrowly, and the historical cut says it usually does not save anything.
- The headless arm needed a 2.5 GB copy of the project because the GUI held it. Without the GUI open, there is no MCP arm at all, so in practice the two routes cover different situations more than they compete.

## Guidance

For the coordinator to adopt or not. Evidence is from the sections above.

| Situation | Use | Why |
|---|---|---|
| Ghidra GUI open with SGW.exe; dependent lookups (call chains, follow an xref, then decompile) | **Ghidra MCP** | Same correctness, 47 s against 88 s, 7 requests against 9, 9% cheaper. Dependent hops are where a JVM start per run hurts. |
| GUI closed, nobody at the machine, or a long batch of independent lookups known in advance | **Headless probe**, output to a file, grepped narrowly | It is the only route without the GUI. One run batches many tokens. Per lookup it puts the same text into context as MCP, so it is no cheaper, but no dearer either. Run it on a project copy if the GUI holds the lock. |
| Large decompiles, either route | Read or grep the part you need | Decompiles are 59% of Ghidra MCP characters, p99 28k per call. |
| Ghidra MCP housekeeping | Skip `list_tool_groups` (10k characters) and repeated `list_open_programs` | 49 calls and 5.7M exposure for no answer. |
| SigNoz counts, group-bys, distinct values, totals | **`signoz_aggregate_logs`** (scalar, `groupBy`), checked against an ungrouped total | Same correctness, 4 requests against 9, a fifth of the characters, 39% cheaper. Its cost does not grow with the number of rows. |
| SigNoz individual rows (timestamps, a field's shape, a handful of events) | **`signoz_search_logs` with a small `limit`** (10 or less) | One limit-10 search gave S4's timestamps for 3.8k characters. |
| SigNoz rows in bulk (a session timeline, wire traces) | `signoz_search_logs`, with the saved result condensed by a script | Never read the raw saved file. Inline results go up to about 50k characters (historical max 49.7k). |
| SigNoz field discovery | `signoz_get_field_keys` only with `searchText` | Unfiltered, a call can return 45k characters. |

Scale check: Ghidra is about 1.3% and SigNoz about 2.2% of all context exposure in the window, against 25% for `Read`. Following this table saves real context in the sessions that use these tools, but it is not a workflow-wide lever. The issue asks for the Ghidra recommendation in [reverse-engineering-with-claude.md](../../guides/reverse-engineering-with-claude.md); that guide is unchanged here, pending the coordinator's decision.
