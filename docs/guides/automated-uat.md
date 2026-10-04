---
title: Automated in-game UAT with the lab
type: how-to
audience: agents and developers running unified-UAT rows through the live research lab
last_updated: 2026-09-29
companion_docs:
  - unified-uat.md
  - live-research-lab.md
  - ../architecture/live-research-lab.md
---

# Automated in-game UAT with the lab

> Type: how-to, with the spec and bundle formats as reference. Audience: an agent or developer with the `cimmeria-lab` MCP server running.
> Updated: 2026-09-29. Companions: [unified UAT guide](unified-uat.md), [live research lab](live-research-lab.md).

The [unified UAT guide](unified-uat.md) is written for a person at the keyboard. This page runs the same rows from the lab: each row is data in a TOML spec, the `lab_uat_run` tool drives the client through the lab tools, checks every expected clause, and writes an evidence bundle plus the guide's "Recording results" blocks, ready to paste into the campaign ledger.

The runner never decides a row passed on weaker evidence than a tester would have. A row is **PASS** only when every step ran at the row's required native level, every required clause was checked and held, and no tool it needs is missing.

## Before you start

1. **Hold the lab lock.** Only one agent drives a client at a time. Create `%LOCALAPPDATA%\cimmeria-lab\live.lock` as a directory (creating a directory is atomic), write your name and purpose into `live.lock\owner`, and remove the directory when you finish. If it already exists, someone else is live: wait and retry every minute.
2. **One `SGW.exe`, the `lab` account.** `lab_client_start` refuses while another client runs. The account and character come from `lab-account.json` ([live research lab, Setup](live-research-lab.md)).
3. **Colo rule 6.** `.announce`, `/gmshout`, `.bm_seed`, `.mute` and content reloads need the owner's say-so in this session. The runner refuses them (the row is BLOCKED) unless the run passes that word in `owner_approvals`.
4. **The screensaver.** A secure screensaver locks the desktop and stops the client rendering. Keep the display awake (hold `ES_DISPLAY_REQUIRED`) or stop a non-secure `*.scr` before launching.
5. **Server evidence.** The runner reads `server_*` tools from `cimmeria-lab-mcp` when `CIMMERIA_LAB_MCP_URL` and `CIMMERIA_LAB_MCP_TOKEN` are set. When that endpoint is unreachable (the colo answered HTTP 403 "Host header is not allowed" on 2026-09-29), server and packet clauses are UNVERIFIED and the row rests on its SigNoz clauses, which you attest (below).

## Run rows

Call `lab_uat_run` from any MCP client of `cimmeria-lab`:

```json
{ "sections": ["gm-parity"], "rows": ["M1-1"], "server_version": "<service.version>" }
```

| Argument | Meaning |
|---|---|
| `sections` | Section ids (the spec's `[section] id`); omit for every section |
| `rows` | Only these row ids |
| `specs_dir` | Spec directory; default `CIMMERIA_LAB_UAT_SPECS`, else the repo's `docs/guides/uat-specs` found from the working directory |
| `run_dir` | Add rows to an existing run instead of starting one |
| `server_version` | The server's `service.version` (git SHA) from SigNoz; can be attested later |
| `owner_approvals` | Colo rule-6 words the owner approved: `announce`, `bm_seed`, `mute`, `gmshout`, `content_reload` |
| `vars` | Extra `${var}` values for the specs, such as `player_id` |
| `character` | Override `lab-account.json`'s character |
| `plan_only` | Check the specs and which tools exist; drive nothing. Ready rows come back SKIPPED, the rest BLOCKED with the missing tool named |

Start with `plan_only: true` to see what can run. Each row then runs as:

1. Standing checks: the row's `blocked` reason, a second player, a non-GM account, a colo rule-6 command, and every tool the row names. Any of these makes the row BLOCKED before anything moves.
2. Reach the row's state with the flows by name: `lab_client_status`, then `lab_client_start`, `lab_login`, `lab_logout`, `lab_create_character` and `lab_play_character` as needed. These are setup actions.
3. The anchor: type `.bug uat <row id>` and read `Bookmark <id> recorded`. The bookmark id is the server's epoch milliseconds, so it also gives the server clock offset.
4. Setup, then the steps. Clauses and evidence tied to a step label run right after that step.
5. The remaining clauses, the evidence, a final screenshot, then teardown.

Long rows can outlast an MCP client's call timeout: run a section a few rows at a time and pass the same `run_dir`.

## Attest SigNoz rows and human answers

The runner has no SigNoz client. Each SigNoz clause is written PENDING with its filter, the row's time window (10 s before the row to 2 min after it) and the `service.version` filter when the run knows the build. Run that query with the SigNoz MCP (see the [SigNoz log-mining notes](../../.claude/agent-memory/main-session/reference_signoz_log_mining.md)), then attest the result:

```json
{ "row": "CD3", "clause": "up-to-date", "row_count": 1,
  "rows": [{ "outcome": "up_to_date", "category_id": 12 }],
  "query_ran": "event = 'cooked_data.version_reply' AND category_id = 12" }
```

`lab_uat_attest` grades the rows by the spec's `min_rows`, `max_rows` and `field op value`, not by your judgement, re-grades the row and rewrites `ledger.md`. The same tool records a person's answer to a human clause (`verdict: "pass"` or `"fail"`, `answer`, `by`), a server clause checked another way, and the run's `server_version`.

`lab_uat_report` summarizes a run per section and lists every row that did not pass with its first reason; `ledger: true` adds every Recording-results block.

## Results

| Result | Meaning | A pass? |
|---|---|---|
| PASS | Every required clause held; every step ran at the required native level | Yes |
| FAIL | A required clause's observation contradicts it, or a step action failed | No |
| BLOCKED | The row could not reach its step: a missing tool (named), a second player, owner-only files, a rule-6 command, a setup action that failed | No |
| SKIPPED | `plan_only`: ready to run, nothing driven | No |
| NEEDS_HUMAN | Every automatic clause held; a person must answer the human question (screenshots attached) | Not yet |
| UNVERIFIED | A required clause is pending (SigNoz not attested) or its reader failed (a Lua read that errored, an unreachable endpoint) | Not yet |
| NATIVE_SHORTFALL | Everything held, but a step ran below the row's required native level (an N3 fallback, a server shortcut) | No |

When several apply, the first in the table's order after PASS wins, and every reason is listed.

### Native levels

| Tier | Drive | Who may use it |
|---|---|---|
| `N1` | Real input: keys, clicks, typed chat, and the flows built on them | Steps (the default requirement) |
| `N2` | A slash command typed into chat that stands in for a UI action | Steps, when the row allows N2 |
| `N3` | The stock UI's Lua binding through `client_lua_eval` | Setup; a step at N3 is never a native pass |
| `G` | A GM command typed into chat | Setup and teardown |
| `X` | A server shortcut (`server_console_exec`, a database write) | Setup only, flagged in the bundle |

A spec may declare a known tool *less* native than its floor (a typed `/useitem` standing in for a double-click is N2) but never more: `client_lua_eval` is N3 at best. A tool the runner does not know (a new tool from another change) must carry its tier in the spec.

## The evidence bundle

One directory per run under `CIMMERIA_LAB_UAT_DIR`, default `%LOCALAPPDATA%\cimmeria-lab\uat-runs\`:

```text
<date>-<build8>-<run id>/
  run.json                       builds, clocks, account, characters, tools, specs
  rows/<section>/<row>.json      one row's evidence
  rows/<section>/<row>/          final.png, other screenshots, tool reads (.json), packet-tap.json
  ledger.md                      summary table + one Recording-results block per row
```

`run.json` holds the server build (`service.version` and where it came from: `arg`, `attested` or unknown), the client fingerprint (SHA-256 of `Binaries\SGW.exe` and of the injected telemetry and patch DLLs), the account, every character seen at character select, the host clock at start and the latest server offset (from the anchor's bookmark id), every tool the router exposed, each spec file with its SHA-256, and the owner approvals.

Each row JSON holds:

| Field | What it is |
|---|---|
| `result`, `reasons`, `flags` | The grade, why it is not PASS, and informational notes (setup that used a server shortcut, a teardown that failed) |
| `required_native`, `native_used` | The row's requirement and the least native tier any step ran at |
| `actions[]` | Every action with its role (`setup`, `step`, `teardown`, `anchor`), the tool that ran (and whether a fallback did), its arguments, tier and where the tier came from, host start time, elapsed ms, result or error, and the individual calls a chat line expanded into |
| `clauses[]` | Every expected clause: its text, source, what it expects, the observed value, the verdict, and for SigNoz the exact query and window |
| `anchor` | The `.bug` note, the bookmark id, host send and seen times, and the server offset |
| `attachments[]` | Screenshots and tool reads, with the tool and host time; `packet_tap` (`packet-tap.json`) holds every message the row's packet tap captured |
| `character`, `account_kind`, `started_utc`, `ended_utc`, `host_*_ms` | Who ran it and when, on both clocks |
| `attestations[]` | Every `lab_uat_attest` applied to the row |

## Write a row spec

Specs live in [docs/guides/uat-specs/](uat-specs/), one file per guide section. TOML, because the repo already reads TOML everywhere, it keeps comments and multi-line strings for the guide's prose, and `[[row]]` / `[[row.expect]]` tables read like the guide's own table.

```toml
schema = 1

[section]
id = "gm-parity"                     # used by `sections` and the bundle paths
system = "GM console command parity" # the ledger's System: line
guide = "docs/guides/unified-uat.md#gm-console-command-parity"
ledger = "docs/analysis/legacy-command-parity/README.md#validation-and-uat-gates"
account = "gm"                       # gm | non-gm
character = "lab"                    # lab | fresh (then a [section.fresh] table)

[[row]]
id = "M1-1"                          # the campaign's step id
title = "Search and readout commands answer in chat"
expected = "Each answers in chat; ..."   # the guide's Expect text
required_native = "N1"               # default N1
state = "in_world"                   # in_world | char_select | client_stopped | any
step = [
  { chat = ".help", label = "help" },
  { chat = ".searchitem pistol", label = "searchitem" },
]

[[row.expect]]
id = "searchitem"
text = ".searchitem names real items"
source = "chat"
since = "searchitem"                 # only lines after that step started
matches = "searchitem 'pistol': [0-9]+ match"
```

**Row fields:** `id`, `title`, `expected`, `required_native`, `state`, `players` (above 1 is BLOCKED until a second player exists), `known_issues`, `blocked` (a standing reason; nothing runs), `anchor` (default true in world), `relog` (what `After relog:` says when the row checks one), `notes`, `setup`, `step`, `teardown`, `expect`, `evidence`.

**Actions** (exactly one of the first four):

| Field | Meaning |
|---|---|
| `tool` + `args` | Call a lab tool by name, or by `@capability` from the capability table (below). A name that is not routed BLOCKs the row with the name |
| `chat` | Type a line into chat: `client_chat_send` when that tool exists, else focus, Enter, `client_type_text`, Enter. The lab types letters, digits, space and `- _ / .` today |
| `wait_ms` | Sleep (for server round trips the runner cannot observe) |
| `capture` = `"chat"` + `regex` + `var` | Store regex group 1 of the newest matching chat line in `${var}` |
| `tier` | Declared native level (see above) |
| `label` | A name for `at`, `since` and timing clauses |
| `fallback` | Actions tried in order when the primary tool is not routed; their tier is what counts |
| `optional` | An error is recorded but does not fail the row |

`${character}`, `${run_id}`, `${row_id}`, `${section}`, `${bookmark_id}`, the run's `vars` and captured values substitute into every string.

**Expected clauses** (`[[row.expect]]`): `id`, `text`, `source`, `required` (default true), `at` (evaluate right after that step; default after all steps).

| `source` | Fields | Verdict |
|---|---|---|
| `chat` | `contains` and/or `matches`, `count` (exactly N lines), `absent`, `since`, `capture_var` | Lines that arrived after `since` (default: before the first step) |
| `tool` | `tool`, `args`, `pointer` (JSON pointer), `op`, `value` | Any lab tool's JSON |
| `lua` | `chunk`, `pointer`, `op`, `value` | `client_lua_eval`'s first result; a Lua error is UNVERIFIED |
| `wait` | `lua_condition`, `timeout_ms` | `client_wait_for` met or not |
| `timing` | `action` (a label), `max_ms` | That step's elapsed time |
| `signoz` | `filter`, `min_rows`, `max_rows`, `field` + `op` + `value` | PENDING until attested |
| `server` | `tool`, `args`, `pointer`, `op`, `value` | A `cimmeria-lab-mcp` tool; UNVERIFIED when unreachable |
| `packet` | `message`, `direction`, `entity`, `min_rows`, `max_rows`, `field` + `op` + `value` (+ `tolerance`) | Decoded Mercury messages from the row's packet tap (below); UNVERIFIED when the endpoint is unreachable |
| `human` | `question` | NEEDS_HUMAN until answered |

`op` is one of `eq`, `ne`, `contains`, `not_contains`, `matches`, `gt`, `gte`, `lt`, `lte`, `exists`, `absent`, `truthy`, `falsy`, `len_gte`, `approx`. Numbers compare numerically even when Lua returns them as strings. `approx` takes a numeric `value` and a `tolerance`: `op = "approx"`, `value = 15`, `tolerance = 1` passes 14 to 16, the spec form of `complete_in_s ~ 15 ± 1`.

### Packet clauses

A `packet` clause asserts what crossed the wire, as the server decoded it. When any clause in a row has `source = "packet"`, the runner starts a packet tap (`server_packet_tap_start`) on the lab character's session right after the row's `.bug` anchor, and before teardown reads it once (`server_packet_tap_read`) and stops it (`server_packet_tap_stop`). It stops the tap on every path that started one, including a failed setup or step, so a row never leaves a tap buffering. The three calls are recorded on the row as setup and teardown actions, and the full read is the row's `packet_tap` attachment.

| Field | Meaning |
|---|---|
| `message` | The message name as the tap decodes it (its `msg_name`, the dispatch-table name); case does not matter |
| `direction` | `to_client` (the server sent it) or `to_server` (the client sent it) |
| `entity` | Only messages sent for this entity (an outbound row's `target_entity_id`): a number or a `${var}`, usually `"${player_entity_id}"` |
| `min_rows`, `max_rows` | How many matching messages; default at least one. `max_rows = 0` asserts the message was never sent |
| `field` + `op` + `value` (+ `tolerance`) | Every matching message must satisfy it. `field` is a decoded argument name, a tap column (`ts_ms`, `method_index`, `args_len`, `args_hex`), or a JSON pointer (`/decoded/...`) |

```toml
[[row.expect]]
id = "cooldown-sent"
text = "the server sends a 15 s cooldown to the caster"
source = "packet"
message = "onTimerUpdate"
direction = "to_client"
entity = "${player_entity_id}"
field = "complete_in_s"
op = "approx"
value = 15
tolerance = 1
```

The session is found by the character's name in `server_sessions`; pass `vars.player_entity_id` to skip that lookup. The runner sets `${player_entity_id}` once it knows it. A packet clause is graded over the whole row, so it takes no `at` or `since`. A clause is UNVERIFIED, naming the reason, when the endpoint is not configured or refuses, the session is not found, or the tap could not be read. When the tap's ring dropped messages (`dropped` in the read), a PASS that depends on an upper bound or on every row becomes UNVERIFIED, because the dropped messages were never checked. Packet clauses cross-check the client's own decode (ability-mechanics AB-C3): the tap and a `client_event` clause on the same row must agree.

**Evidence** (`[[row.evidence]]`): `name`, `tool`, `args`, `at`. Images become PNG attachments; JSON results become `.json` attachments. Every row that ran also gets `final.png`.

### The capability table

`crates/lab/src/uat/tools.rs` maps each capability to the tool that provides it and the most native tier it can claim: `@world_click` is `client_world_click` at N1, `@inventory` is the read `client_inventory`. It lists the tools on `main`, including the world tools (#1099: `@entity_find`, `@world_click`, `@target`, `@move_to`, `@camera`) and the combat tools (#1100: `@use_ability`, `@combat_log`, `@die_and_respawn`, `@wait_event`, `@hotbar`), and the planned ones (UI and items: `@window_read`, `@window_click_row`, `@chat_log`, `@inventory`, `@player_state`, `@item_action`, `@drag_drop`; and `@chat_send`, `@cache_files`). Write planned tools by alias in specs: when one lands under another name, the fix is one line in the table. A tool that reports how it drove the game (`native_level` as a word, `real_input` / `slash_command` / `ui_lua` / `server_shortcut`, as the world tools do, or as `{tier: "N1".."X"}`, as the combat tools do) overrides its table tier when it fell back lower, so a `client_target` that used `targetUnit` counts as N3.

`cargo test -p cimmeria-lab` parses and validates every committed spec, so a typo in a field name or a dangling label fails the build.

## Spec coverage

Rows authored in [docs/guides/uat-specs/](uat-specs/), 2026-09-29, and what a `plan_only` run against today's router says (the `committed_specs_plan_against_main_tools` test pins it):

| Section | Ready now | Blocked, and on what |
|---|---|---|
| `gm-parity` | M1-1, M4-1b | M1-2 (second player, X1), M1-4 (non-GM account, X2) |
| `chat` | 6 (solo half), 9a, 9c | 7-solo (the tell syntax, L11) |
| `pets` | U1, U2b, U12 | U16 (pet-bar window names, L10), U13 (non-GM account) |
| `bank` (fresh character) | 1, 12, 2 (`@world_click`, #1099) | |
| `black-market` | U1, U22 | U0 (needs `owner_approvals: ["bm_seed"]`) |
| `crafting` (fresh Scientist) | 1, 2, 3, 5, 19 | |
| `consumables` | I1 | I2 (`@item_action`, `@inventory`) |
| `cooked-data` | CD3, CD6 | CD1, CD5 (`@cache_files`), CD2 (owner-only files), CD4 (`@cache_files`, `@world_click`) |
| `castle-cellblock` | T01/T02 (makes and deletes its own character) | |

22 rows are ready and 12 are blocked. None of them has run against a live client yet. The first live run should take them in this order, each proving one more part of the runner:

1. `gm-parity` M1-1: typed chat, the `.bug` anchor and server clock, `since` chat marks, the ledger block.
2. `chat` 9a and 9c, `black-market` U1 and U22: exact-count clauses, refusals, teardown.
3. `pets` U12, then `chat` 6: setup in G, captures feeding a later command.
4. `cooked-data` CD3 and `pets` U2b: relogs inside a row, timing clauses, SigNoz attestation.
5. `gm-parity` M4-1b: cross-world travel (the chat box across load screens).
6. `bank` 1, 12 and 2 (the first world click), `crafting` 1-19, `castle-cellblock` T01/T02: fresh characters, Lua reads (the stock bindings named there are unproven: a read that errors is UNVERIFIED, and the spec is fixed from what the client shows).
7. `consumables` I1 and `cooked-data` CD6.

The [lab tooling backlog](../analysis/lab-automation/tooling-backlog.md) lists the tools the blocked rows wait for. When one lands, a spec that names it runs with no runner change. The colo also seeds `lab2` to `lab5` now (#1093), so the two-player rows can be written against a second lab instance ([Two clients](live-research-lab.md#two-clients-two-player-scenarios)); `Castle_CellBlock` is instanced per login, so both players move to Castle (world 8) first.

## Troubleshooting

| Symptom | Cause and fix |
|---|---|
| Every row BLOCKED "typing chat needs tool ..." | The router is missing an input tool: the supervisor build is older than the flows (#1080) |
| A chat clause FAIL with "the chat box was cleared or overflowed" | More than 150 lines arrived, or a relog cleared the box: put the clause `at` an earlier step |
| A Lua clause UNVERIFIED "Lua read failed" | The stock binding name is wrong or not loaded yet; read it once with `client_lua_eval` and fix the spec |
| `.bug` anchor shows "no bookmark reply seen" | The reply took over 4 s or the account is not a GM; the row still runs, but the server clock offset is missing |
| The run stops mid-section | An MCP call timeout: rerun the remaining rows with the same `run_dir` |
