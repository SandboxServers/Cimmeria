---
title: Automated in-game UAT with the lab
type: how-to
audience: agents and developers running unified-UAT rows through the live research lab
last_updated: 2026-10-11
companion_docs:
  - unified-uat.md
  - live-research-lab.md
  - ../architecture/live-research-lab.md
---

# Automated in-game UAT with the lab

> Type: how-to, with the spec and bundle formats as reference. Audience: an agent or developer with the `cimmeria-lab` MCP server running.
> Updated: 2026-10-11. Companions: [unified UAT guide](unified-uat.md), [live research lab](live-research-lab.md).

The [unified UAT guide](unified-uat.md) is written for a person at the keyboard. This page runs the same rows from the lab: each row is data in a TOML spec, the `lab_uat_run` tool drives the client through the lab tools, checks every expected clause, and writes an evidence bundle plus the guide's "Recording results" blocks, ready to paste into the campaign ledger.

The runner never decides a row passed on weaker evidence than a tester would have. A row is **PASS** only when every step ran at the row's required native level, every required clause was checked and held, and no tool it needs is missing.

To check that a run still behaves the way known-good runs did, without a model reading it, compare it with the section's golden: see [Golden runs](#golden-runs).

## Before you start

1. **Hold the lab lease.** Only one agent drives a client at a time, and the supervisor enforces it. Either pass `lab_uat_run` the `lease_id` from your own `lab_lease_acquire {owner, purpose}`, or pass none and the run takes a lease of its own (owner `lab_uat_run`) for its duration; both are refused while another session holds the lab, naming who. `plan_only` needs no lease. The `%LOCALAPPDATA%\cimmeria-lab\live.lock` folder lock is obsolete: do not create it. Run the [shared daemon](live-research-lab.md#the-shared-daemon-cimmeria-lab---http) so every session sees the same lease; a stdio supervisor's watchdog relaunches a client another session killed, so ask its owner for `lab_client_stop` instead ([live research lab, before you drive the client](live-research-lab.md#before-you-drive-the-client-the-lab-lease)). Install the lab binaries from the commit you test ([Install or update the lab](live-research-lab.md#install-or-update-the-lab)).
2. **One `SGW.exe` per lab instance, the `lab` account for the default one.** `lab_client_start` refuses while an `SGW.exe` runs outside the lab, when another instance holds its bridge port, and past the client cap ([Parallel clients](live-research-lab.md#parallel-clients-up-to-five)). The account and character come from `lab-account.json` ([live research lab, Setup](live-research-lab.md)). A two-player row adds a second client, `p2`, on its own account ([Two-player rows](#two-player-rows)).
3. **Colo rule 6.** `.announce`, `/gmsendgmshout` (and the old `/gmshout`), `.bm_seed`, `.mute` and content reloads need the owner's say-so in this session. The runner refuses them (the row is BLOCKED) unless the run passes that word in `owner_approvals`.
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
| `owner_approvals` | Colo rule-6 words the owner approved: `announce`, `bm_seed`, `mute`, `gmshout` (covers `/gmsendgmshout` and `/gmshout`), `content_reload` |
| `vars` | Extra `${var}` values for the specs, such as `player_id` |
| `character` | Override `lab-account.json`'s character |
| `fixtures` | When a row's fixture applies: `auto` (default), `always` or `never` ([Row fixtures](#row-fixtures)) |
| `plan_only` | Check the specs and which tools exist; drive nothing. Ready rows come back SKIPPED, the rest BLOCKED with the missing tool named |

Start with `plan_only: true` to see what can run. Each row then runs as:

1. Standing checks: the row's `blocked` reason, a second player, a non-GM account, a colo rule-6 command, and every tool the row names. Any of these makes the row BLOCKED before anything moves.
2. Reach the row's state with the flows by name: `lab_client_status`, then `lab_client_start`, `lab_login`, `lab_logout`, `lab_create_character` and `lab_play_character` as needed. These are setup actions.
3. The anchor: type `.bug uat <row id>` and read `Bookmark <id> recorded`. The bookmark id is the server's epoch milliseconds, so it also gives the server clock offset.
4. Setup, then the steps. Clauses and evidence tied to a step label run right after that step.
5. The remaining clauses, the evidence, a final screenshot, then teardown.

Long rows can outlast an MCP client's call timeout: run a section a few rows at a time and pass the same `run_dir`.

### Run without an agent: `lab uat`

When every step, wait, coordinate and camera move is in the spec, no model is needed. [`lab uat`](live-research-lab.md#commands) calls `lab_uat_run` straight from PowerShell. It can run up to 5 lanes in parallel, each up to 20 times, and aggregates the results into one summary. `-Json` gives an agent or a script one compact object:

```powershell
lab uat first-session -Rows FS-01,FS-02,FS-P1,FS-P2,FS-P3,FS-P4,FS-P5 -Leases 2 -RunsPerLease 5
```

While a batch runs, [`lab watch`](live-research-lab.md#commands) in another terminal shows it live (each lane's run, the row under way, the last failure), and `lab watch -Once -Json` gives an agent a snapshot without waiting for the batch to end.

**`-Heal`.** When a row fails because a pose's click, target or camera move missed in the world, its summary carries a heal hint, `heal: { pose, step }`, with `step` one of `hover`, `on_screen`, `face`, `abs_pitch` or `abs_yaw` (the camera steps only when the closed loop ran and missed, not when the readout was missing). With `-Heal` (it needs `-Leases 1`, else exit 2 before the daemon is asked), `lab uat` then recalibrates each distinct `(row, pose)` pair of the failed runs' hints with one `lab_uat_calibrate` call each, on the batch's instance, at the default budget ([Calibrate a row](#calibrate-a-row)). Each proposal is copied to `heal-<row>-<pose>.diff` in the batch folder and printed. `batch.json`, `summary.md` and the `-Json` object gain `heal: [{row, pose, diff}]` only when a proposal was written (`-Json` lists at most 3, then `more_heal: <n>`), and the command then exits **8**, over every other code, 1 and 5 included. Nothing is applied: the spec file is never written. With no hints it prints `heal: no failed row carries a heal hint` and exits as the batch would. A heal adds up to 15 minutes of lab time per pose; Ctrl+C during the batch skips it, and Ctrl+C during the heal keeps the diffs already written.

### Hand a run to a cheap agent

When an agent should drive it instead, the [`lab-driver`](../../.claude/agents/lab-driver.md) agent (Haiku) makes one `lab_uat_run` call and reports each row's result. Its whole brief is the arguments:

```text
Run spec rows with no lease_id (the run takes and releases its own lease): sections ["first-session"],
rows ["FS-01","FS-02","FS-P1","FS-P2","FS-P3","FS-P4","FS-P5"], specs_dir "<repo>\docs\guides\uat-specs",
server_version "<service.version>".
```

That costs about 42k tokens and two tool calls per run, almost all of it the agent's fixed context. Leave the lease to the run: told to borrow a coordinator's lease and leave it alone, Haiku still released it at the end (2026-10-10). Then check its claims in the run's bundle, not its summary.

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
| BLOCKED | The row could not reach its step: a missing tool (named), a second player, owner-only files, a rule-6 command, a setup action that failed, a row fixture that was not established | No |
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

`lab_uat_run` with `fingerprint: true` captures what a golden fingerprint needs (`lab golden` and `lab uat -DiffGolden` set it). Every row that reaches the world gets a packet tap, not only rows with a packet clause. When the character is not in the world at the row's start, the runner checks for its session after each action and starts the tap after the first action that brings it in. Those checks are not recorded; the tap start that succeeds is. A row marked `[row.golden] skip = true` is left out. Fingerprint mode never changes a grade: its server calls are added to `actions[]` as setup actions after the row ends, so step numbers in the reasons stay the same. It never sets `${player_entity_id}`, and it never starts a late tap for a row that has a packet clause.

Each fingerprinted row gets three attachments, except a row blocked before its tap would start (it could not reach its state, or a two-player row could not bring p2 in), which gets none. A row cut off by a lost lease still gets them: the runner stops its tap and records whether the stop worked in `stopped`. `packet_tap` (`packet-tap.json`) gains `coverage` and `replaced_existing`. `coverage` is `full` when the tap started with the row, `late:<action>` when it started after that action (the action's label, else `setup#<i>` or `step#<i>`), `none` when it never started (with the `error`), and `unavailable` when no server lab MCP is configured. `replaced_existing: true` means another tap on the character was replaced, so its messages are gone. `entities` (`entities.json`) holds the player entity, its space and a snapshot of every entity in that space when the tap binds (`bind`) and before it is read (`finish`). Each snapshot keeps `entity_id`, `is_player`, `tag`, `spawn_id` and `template_id`. `windows` (`windows.json`) holds the visible windows from `client_ui_state` at the row's start and after every setup and step action; a sample whose read failed holds an `error` instead of `windows`. The [lab-golden contract](../analysis/lab-golden/contract.md) gives the exact shapes.

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

**Where rows run.** A row that tests a feature rather than a level (vendors, bankers, mail, trade, inventory, abilities, combat, chat) runs in the Debug Area (world 1300, `.gotolocation DebugArea`). It has a copy of every hub service, open ground and clear sight lines, where Castle_CellBlock's hub NPCs stand against walls the camera ends up behind. Use a level's own map only when the row is about that level: its missions, NPC placement, transitions or geometry.

**Row fields:** `id`, `title`, `expected`, `required_native`, `state`, `players` (2 drives the second lab client, `p2`, and is BLOCKED with the reason when none is configured; above 2 is always BLOCKED), `known_issues`, `blocked` (a standing reason; nothing runs), `anchor` (default true in world), `relog` (what `After relog:` says when the row checks one), `notes`, `pose` (the row's interaction poses, [below](#targets-poses-and-waits)), `setup`, `step`, `teardown`, `expect`, `evidence`.

**Actions** (exactly one of the first five):

| Field | Meaning |
|---|---|
| `tool` + `args` | Call a lab tool by name, or by `@capability` from the capability table (below). A name that is not routed BLOCKs the row with the name |
| `chat` | Type a line into chat: `client_chat_send` when that tool exists, else focus, Enter, `client_type_text`, Enter. The lab types letters, digits, space and `- _ / .` today |
| `wait_ms` | Sleep (for server round trips the runner cannot observe) |
| `capture` = `"chat"` + `regex` + `var` | Store regex group 1 of the newest matching chat line in `${var}` |
| `wait_server` = `{ log, target, fields, timeout_ms }` | Wait for a server log line instead of sleeping ([below](#wait-for-the-server-wait_server)) |
| `tier` | Declared native level (see above) |
| `label` | A name for `at`, `since` and timing clauses |
| `fallback` | Actions tried in order when the primary tool is not routed; their tier is what counts |
| `optional` | In `setup` or `teardown`, an error is recorded but does not block or fail the row. In `step`, the row goes on to the next step, but the failed step still fails the row |
| `client` | `p1` (default) or `p2`: which lab client runs it. `p2` needs `players = 2` |

The composites ([live-research-lab.md](live-research-lab.md#fewer-calls-composites-and-compact-results)) are routed tools like any other: `tool = "client_ui_sequence"` runs a scripted UI step as one action and reports the least native level it used, and a `tool` clause on `client_batch` reads one step's value with `pointer = "/steps/<id>"`. The runner calls tools in-process, so it always sees full results, never the compacted ones an MCP client gets. Reaching a row's state still uses the single flows (step 2 above), so each one's timing and failure stay in the evidence.

`${character}`, `${run_id}`, `${row_id}`, `${section}`, `${bookmark_id}`, `${p2_character}` (two-player rows), `${player_entity_id}` (rows with packet clauses or an `@ability_state` read of the lab character; a `wait_server` that names it reads it from the client, `client_entity_find` `/player/id`, when it is unset, and keeps it), `${cast_id}`, `${cast_entity_id}`, `${cast_player_id}` and `${cast_key}`, each also as `..._<label>` (after an ability press, [below](#client-event-clauses-and-cast_id)), `${dummy_id}` (after `@dummy`), `${fixture_character}` and `${fixture_mode}` (rows with a fixture, [below](#row-fixtures)), the run's `vars` and captured values substitute into every string. A string that is exactly one `${var}` takes the variable's own type, so `entity_id = "${dummy_id}"` reaches a tool as a number. A captured whole number (a mail or entity id) is stored as a number for the same reason.

**Expected clauses** (`[[row.expect]]`): `id`, `text`, `source`, `required` (default true), `at` (evaluate right after that step; default after all steps), `client` (`chat`, `tool`, `lua`, `wait` and `client_event` clauses: read `p2` instead of `p1`).

| `source` | Fields | Verdict |
|---|---|---|
| `chat` | `contains` and/or `matches`, `count` (exactly N lines), `absent`, `since`, `capture_var` | Lines that arrived after `since` (default: before the first step) |
| `tool` | `tool`, `args`, `pointer` (JSON pointer), `op`, `value`, `timeout_ms` | Any lab tool's JSON; with `timeout_ms`, polled ([below](#poll-a-clause-timeout_ms)) |
| `lua` | `chunk`, `pointer`, `op`, `value` | `client_lua_eval`'s first result; a Lua error is UNVERIFIED |
| `wait` | `lua_condition`, `timeout_ms` | `client_wait_for` met or not |
| `timing` | `action` (a label), `max_ms` | That step's elapsed time |
| `signoz` | `filter`, `min_rows`, `max_rows`, `field` + `op` + `value` | PENDING until attested |
| `server` | `tool`, `args`, `pointer`, `op`, `value`, `timeout_ms` | A `cimmeria-lab-mcp` tool; UNVERIFIED when unreachable. `@ability_state` (`server_ability_state`) with no `entity_id` reads the lab character; with `timeout_ms`, polled |
| `client_event` | `event`, `match_fields`, `since`, `min_rows`, `max_rows`, `field` + `op` + `value` (+ `tolerance`), `timeout_ms` | The client's own telemetry events (`client.ability.*`) from the lab event store ([below](#client-event-clauses-and-cast_id)) |
| `packet` | `message`, `direction`, `entity`, `match_fields`, `min_rows`, `max_rows`, `field` + `op` + `value` (+ `tolerance`) | Decoded Mercury messages from the row's packet tap (below); UNVERIFIED when the endpoint is unreachable |
| `human` | `question` | NEEDS_HUMAN until answered |

A `tool`, `server` or `lua` clause with `capture_var` keeps what it observed in that var, so a later clause can compare with it: a baseline read `at` an early step with `op = "exists"` and `capture_var = "mitigation_before"`, then `op = "eq"` and `value = "${mitigation_before}"` after the toggle goes off. Every clause's `value` and `match_fields` take `${var}`s.

`op` is one of `eq`, `ne`, `contains`, `not_contains`, `matches`, `gt`, `gte`, `lt`, `lte`, `exists`, `absent`, `truthy`, `falsy`, `len_gte`, `approx`. Numbers compare numerically even when Lua returns them as strings. `approx` takes a numeric `value` and a `tolerance`: `op = "approx"`, `value = 15`, `tolerance = 1` passes 14 to 16, the spec form of `complete_in_s ~ 15 ± 1`.

A `pointer` may select an array element by a field: `[key=value]` after a segment picks the first element whose `key` equals `value` (numbers numerically). `server_ability_state` lists stats and ledger entries as arrays, so `/state/stats[stat_id=22]/cur` reads one stat and `/state/ledger[ability_id=637]/expires_in_secs` one effect, wherever they sit in the list.

### Targets, poses and waits

A step should say *what* it acts on and *how the view should look*, not where someone once measured it. Three pieces do that: a `pose` names the target by its spawn tag and describes where to stand and how to aim; `@stand`, `@camera` and `@world_click` take the pose by id; and `wait_server` ends a wait on a server event instead of a sleep. [first-session.toml](uat-specs/first-session.toml) is the worked example: FS-P3 and FS-P4 for the Praxis corpses, FS-S3 to FS-S6 for SGC_W1. The decisions behind it are D-SV1 to D-SV12 in the [lab-spec-vocab ledger](../analysis/lab-spec-vocab/README.md).

```toml
[[row]]
id = "FS-P3"
pose = [
  { id = "frost", tag = "ArmYourself_FrostBody", radius_m = 4.16, bearing_deg = 322.5, dy_m = 0.13, pitch_deg = -28.8, yaw_offset_deg = 25.3, zoom = 250.0, calibrated = "2026-10-10 hand" },
]
setup = [
  { chat = "/gmsetgodmode 1", tier = "G" },
  { tool = "@stand", args = { pose = "frost" } },
  { wait_ms = 1500 },   # nothing waits on the target's introduction yet
]
step = [
  { tool = "@camera", args = { pose = "frost" }, label = "aim" },
  { tool = "@world_click", args = { pose = "frost", button = "right", expect = "window" }, label = "frost" },
]
```

**The `pose` line.** Each pose is one inline table on one line (TOML inline tables cannot span lines), so `lab calibrate` can replace exactly that line and nothing else (D-SV2). Write the keys in this order:

| Key | Meaning |
|---|---|
| `id` | Row-unique, `[a-z0-9-]+`; actions name it with `pose = "<id>"` |
| `tag` | The target's `spawnlist.tag`, exact. Checked against the seed (below) |
| `radius_m` | Horizontal stand-off from the target, 1.0 to 8.0 m |
| `bearing_deg` | Which side of the target to stand on, 0 to under 360 |
| `dy_m` | Stand height above the target's origin, -3.0 to 3.0 (default 0.1) |
| `pitch_deg` | Optional. Camera pitch after facing, in the readout's own `pitch_offset_deg` degrees, -78.75 to 78.75 |
| `yaw_offset_deg` | Optional. Camera yaw after facing, -180 to 180 |
| `zoom` | Optional. Camera distance, 100 to 775 (a new character starts at 250) |
| `aim` | `"entity"` (default) or `"point"` |
| `aim_dy_m` | With `aim = "point"` only, and then required: metres above the target's origin |
| `calibrated` | Provenance, one line: `lab calibrate` writes `"<date> 3/3"`; a hand measurement says so |

**The geometry (D-SV3).** The stand point is the target's *live* server position plus `radius_m` along the bearing: x + r·cos(b), z + r·sin(b), y + `dy_m`, in server metres. Bearing 0 stands you on the target's +x side, 90 on its +z side, 180 on its -x side, 270 on its -z side. So Frost at (-328.30, 73.47, -210.27) with radius 4.16 and bearing 322.5 gives the stand point (-325.0, 73.6, -212.8), the hand-measured point it replaced. Because the target's live position is read each time, a pose survives an NPC that moved.

**`@stand`** teleports to the stand point with `/gmgotoxyz` (2 decimals, tier G), then asks the client for the player's position every 250 ms until the server x/z is within 0.5 m of the point, for up to 5 s (D-SV11). It takes `pose`, or a bare `tag` with `radius_m`, `bearing_deg` and optionally `dy_m`, or a fixed `point = { x, y, z }`. It waits for the *player* only, so a target the client has not introduced yet is not waited for: keep a short sleep after it before addressing the target, as FS-P3 does.

**`@camera { pose }`** faces the target, then sets the pose's `pitch_deg`, `yaw_offset_deg` and `zoom`. `client_camera` applies its arguments in a fixed order (D-SV4): relative counts, zoom, the face, then the absolute pitch, then the absolute yaw. The pitch is a closed loop on the readout's `pitch_offset_deg` (up to 4 looks, 0.5 deg tolerance); use the number the `aim-pitch` clauses grade and never reinterpret its sign. The yaw is a turn from where the face left the camera, closed loop on the camera actor's world yaw, so a positive value turns the view the way positive `yaw_counts` do on a stock camera, and means the same view on a client whose yaw is inverted. Because both are absolute, the view no longer depends on where the previous row left the camera.

**`@world_click { pose }`** (and `@target`, `@entity_find`) clicks the resolved entity by id, or the aim point for `aim = "point"`. A pose adds `rotate_camera = false`, because the stand and camera have already set the view; give `rotate_camera = true` to keep the click tool's own turn, as the uncalibrated SGU rows do.

**Bare `tag` and `face_tag`.** Without a pose, `tag` names the target on `@world_click`, `@target`, `@entity_find` and `@stand`, and `face_tag` names the face target on `@camera`. A bare tag clicks with the click tool's default rotation (on).

**Give one.** Each argument that names a target replaces the others, so validation refuses a pair, and the runner refuses it again for arguments built from `${vars}`:

- `pose` together with `name`, `entity_id` or `point` (and, on `@camera`, `face_name`, `face_entity_id` or `face_point`): "`pose with <key>: give one`".
- `tag` together with `name`, `entity_id`, `point` or `pose`; `face_tag` together with `face_name`, `face_entity_id`, `face_point` or `pose`.
- `pose` only on `@camera`, `@world_click`, `@target`, `@entity_find` and `@stand`; `tag` not on `@camera`; `face_tag` on `@camera` only.

Any other argument next to a pose wins over the pose's value: `pitch_deg`, `yaw_offset_deg`, `zoom`, `rotate_camera`, `aim`, `aim_dy_m`. A `zoom_to` keeps the pose's `zoom` out, because the camera refuses both. `aim_dy_m` with an entity aim is an error, not ignored.

**When to use `aim = "point"`.** Only when the target never answers an entity hover. The Cellblock NID Guard's body is the known case (FS-P4): the click goes `aim_dy_m` (0.28) above its origin, on the torso. A point aim cannot prove the target is under the cursor; it can only prove nothing else is (the avatar, another entity, a HUD window), so prefer the entity aim whenever it hovers.

**How a tag becomes an entity (D-SV1).** The client has no tags; the server does. The runner reads the player's entity id from the client (`client_entity_find` `/player/id`), its space with `server_entity_get`, then `server_entity_query { space_id, tag }`, filters the reply by tag and space again itself (a server older than the tag filter answers with the whole space), and hands the client tool a plain `entity_id` or `point`. `pose`, `tag`, `face_tag`, `aim` and `aim_dy_m` never reach a client tool: the action's `args` in the bundle are what the tool got. Each server read is in the action's `calls` as `{ tool, args, ok }`, with the resolution as `{ resolve: { tag, entity_id, space_id } }` (plus `pose` when the action named one; `-Heal` and lab-golden key on it). Zero matches fail the action with "`no entity with tag <tag> in space <id>`", several with "`tag <tag> matches entities <ids> in space <id>`".

A row whose steps name a pose, `tag` or `face_tag` is BLOCKED without the server lab MCP ("`<tool> by tag needs the server lab MCP (CIMMERIA_LAB_MCP_URL/_TOKEN)`"), and any of those or an `@stand` needs `client_entity_find` routed. An `@stand { point }` needs no server.

**Seed validation (D-SV10).** Every tag a spec names, in a `pose` or in a `tag` / `face_tag` argument, must be a `spawnlist.tag` in `db/resources/Worlds/Seed/`. `cargo test -p cimmeria-lab` checks every committed spec, and `lab_uat_run` checks again when the seed sits beside the specs (a checkout, not an installed copy). An unknown tag fails as "`tag <tag> is not in the seed`", with "did you mean" when only the case differs. Watch the near-twins: `SGCW1_GenHammond` but `SGC_W1_Tealc`. The check proves the tag exists, not that it is the entity you meant or that it is in the player's space.

#### Wait for the server: `wait_server`

```toml
{ wait_server = { log = "Cinematic AoI hold: released", target = "aoi.cinematic_hold", fields = { witness_id = "${player_entity_id}" }, timeout_ms = 30000 }, label = "movie-over" },
```

| Field | Meaning |
|---|---|
| `log` | Required. A case-sensitive substring of the log message |
| `target` | The tracing target, exact. Give it whenever the message text is generic |
| `fields` | Every one must equal the line's field: numbers numerically (`"8"` equals `8`), `${var}`s filled in |
| `timeout_ms` | Default 30000, 1 to 120000 |

The runner reads the server's newest log timestamp *before the action in front of the wait* (a setup action too), so a line that action caused is never missed. The wait then polls `server_log_tail` every 250 ms with `contains`, `target`, `since_ms` and `limit: 500` (the whole ring), and passes on the first newer line with every field. The action's result is `{ matched: { timestamp_ms, message, fields } }` with only the named fields. A timeout fails with "`no server log '<log>' <fields> within <n> ms`"; a failed read fails at once rather than polling an unreachable server, and every server call is bounded by the timeout. Without the server lab MCP the row is BLOCKED. `lab.tool_call` lines (the endpoint's own audit rows, one per call, polls included) never satisfy a wait. The 17 s sleeps for the first-login hold in FS-P2 and FS-S2 are now this wait on the hold's release line.

#### Poll a clause: `timeout_ms`

A `tool` or `server` clause with `timeout_ms` (at most 60000) is re-read every 250 ms while it FAILs, until it passes or the time is up. It replaces "sleep, then read once" for a server write that lands a moment after the step, such as FS-P3's `sgw_mission` step check. UNVERIFIED, BLOCKED and read errors are not retried. A clause read more than once says `polls: <n>` in its detail.

### Packet clauses

A `packet` clause asserts what crossed the wire, as the server decoded it. When any clause in a row has `source = "packet"`, the runner starts a packet tap (`server_packet_tap_start`) on the lab character's session right after the row's `.bug` anchor, and before teardown reads it once (`server_packet_tap_read`) and stops it (`server_packet_tap_stop`). It stops the tap on every path that started one, including a failed setup or step, so a row never leaves a tap buffering. The three calls are recorded on the row as setup and teardown actions, and the full read is the row's `packet_tap` attachment.

| Field | Meaning |
|---|---|
| `message` | The message name as the tap decodes it (the tap row's `msg_name`: the method for an entity call, else the Mercury message name, the dispatch-table name either way); case does not matter. This is not the `wire.in` log field of the same name, which holds only the Mercury message name and puts the method in `method_name` |
| `direction` | `to_client` (the server sent it) or `to_server` (the client sent it) |
| `entity` | Only messages sent for this entity (an outbound row's `target_entity_id`): a number or a `${var}`, usually `"${player_entity_id}"` |
| `match_fields` | Only messages whose decoded fields (or tap columns) equal these, numbers numerically: `{ dead = true }` counts the state-field updates that set the dead bit |
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

**Evidence** (`[[row.evidence]]`): `name`, `tool`, `args`, `at`, `client`. Images become PNG attachments; JSON results become `.json` attachments. Every row that ran also gets `final.png`, and a two-player row gets `final-p2.png` as well.

### Client event clauses and `${cast_id}`

A `client_event` clause asserts what the client itself did, from the `client.ability.*` events the telemetry DLL pushes to the lab ring (ability-mechanics AB-C1 to AB-C5): the press and its gate (`client.ability.press`, `.press_dropped`), the send (`.sent`, `.sent_seq`), what arrived (`.recv`), what the client applied (`.applied`) and what it showed (`.shown`). Any other `client.*` target the DLL pushes works the same way. The fields are the DLL's own; [client-telemetry.md](../architecture/client-telemetry.md#ability-telemetry-clientability) lists them (the receive, apply and show rows are added there by AB-C3 to AB-C5).

| Field | Meaning |
|---|---|
| `event` | The telemetry target, `client.` included (`client.ability.sent`); a glob (`client.ability.*`) is fine. The ring stores it without the prefix |
| `match_fields` | Only events whose fields equal these: `{ method = "onEffectResults", ability_id = 597 }`. Strings are globs, numbers compare numerically, `${var}`s are filled in |
| `since` | Only events after that step started (default: the row start, after the anchor) |
| `min_rows`, `max_rows` | How many matching events; default at least one. `max_rows = 0` asserts the client never did it |
| `field` + `op` + `value` (+ `tolerance`) | Every matching event must satisfy it. `field` is an event field (`target_id`), a store column (`store_seq`, `store_kind`, `store_ts_ms`) or a JSON pointer |
| `timeout_ms` | How long to wait first for `min_rows` events, or for one more than `max_rows` (default 5000) |

```toml
[[row.expect]]
id = "sent-no-target"
text = "the press leaves the client, aimed at nobody (B-15)"
source = "client_event"
event = "client.ability.sent"
match_fields = { ability_id = 597 }
since = "press"
field = "target_id"
op = "eq"
value = 0
```

The runner reads the store through `client_wait_event` with an explicit `since_seq`: it marks the store head at the row start and before every step a clause names in `since`, and never calls `client_events_read`, whose cursor belongs to whoever drives the lab. A clause is UNVERIFIED when no mark could be taken. A PASS that rests on an upper bound (`max_rows`) or on every event (`field`) becomes UNVERIFIED when events may be missing, because the missing ones were never checked. The runner reads the loss signals apart from the clause's own match:

- the store evicted events after the mark (`gap`);
- the bridge ring dropped events before the store saw them (`bridge_dropped`);
- the client throttle suppressed rows of the clause's family (burst 8, then 4 a second).

The throttle decides a press and its answers together and puts the count of suppressed presses only on the *next* `client.ability.press` row, so a dropped `client.ability.sent` leaves no row of its own. A clause on any `client.ability.*` kind is therefore checked against every `client.ability.*` row since its mark. A suppression at the very end of the window, with no row after it, cannot be seen: do not bound a burst of more than 8 presses a second.

**`${cast_id}` and the cast's caster.** After every `@use_ability` press in setup or the steps, the runner finds the cast it became, so SigNoz and server clauses can name it. A `cast_id` is the caster's own `effect_seq`, so it is unique per caster, not across the server: two players can hold the same number in the same minute. A cast is therefore named by the pair (caster, `cast_id`). The runner stores:

| Var | Value |
|---|---|
| `${cast_id}` | the cast's id |
| `${cast_entity_id}` | the caster's entity (the receipt's `source_id`, or the pressing character's session) |
| `${cast_player_id}` | the caster's `player_id`, when the server log tail still holds the cast's `ability_launched` row |
| `${cast_key}` | both halves as one SigNoz fragment: `cast_id = C AND (entity_id = E OR source_id = E OR invoker_id = E)`. Cast rows name their caster under one of those three fields (heals as `source_id`, pulses as `invoker_id`) |

Write SigNoz clauses with the key, never `cast_id` alone: `filter = "event = 'ability_launched' AND ${cast_key}"`. The runner tries, in order:

1. `client_recv`: the client's `client.ability.recv` `onEffectResults` for the pressed ability, after the press. Its `cast_id` is the effect id the server sent, and its `source_id` is the caster.
2. `seq_join`: the press's `client.ability.sent`, its `client.ability.sent_seq` packet range (28-bit, wrapping), the **pressing entity's** `use_ability_recv` row whose `mercury_seq` is in that range (packet seqs are per connection), and that entity's next `ability_launched` for the ability, from `server_log_tail` (the server's DEBUG ring, 500 rows).
3. `press_window`: the `ability_launched` for (the pressing entity, the ability) nearest the press on the server clock (the anchor's offset), within 2 s. The press time is `client_use_ability`'s own `press_ms`, taken just before the key or click went out; else the client's `client.ability.press` row; else the action's start, which is early by the tool's lookup and placement time.

The press's action records which path found it and when the press went out (`calls: [{cast_id, cast_entity_id, cast_player_id, via, press_ms, press_time_from}]`), or every reason none did. Every value also goes into a `_<label>` var for a labelled press. All of them are cleared before each press's attempt, so a press with no cast found leaves none behind, and a SigNoz clause that still names one is written with the literal and says so.

### Two-player rows

A row with `players = 2` drives a second lab client, `p2`, alongside the lab character. Before the row's anchor the runner brings p2 in world, starting its client, logging in and playing its character as needed. It reuses a p2 that is already in world only after p2's client confirms, through `client_player_state`, that it is playing that character. Otherwise, or when the client cannot say, it logs out and re-selects. Every call is recorded as a p2 setup action. Then:

- steps, clauses and evidence with `client = "p2"` run on p2's client. Everything else runs on p1, as in a one-player row. Each client's chat box has its own marks, so a p2 chat clause counts only p2's new lines;
- `@target_player` targets the other player's character by name with real input. A fallback runs on its action's client: it may repeat that client but not name another one. It is `client_target` with `name` set to `${p2_character}` when p1 runs it, and to `${character}` when p2 runs it. Its other `args` (`allow_fallback`, `settle_ms`) pass through. A `targetUnit` fallback reports N3, which costs the row its PASS as usual;
- to read p2's own state, use `{ source = "tool", tool = "@player_state", client = "p2", ... }`.

```toml
[[row]]
id = "M1-2"
title = "Readouts describe the selected player"
expected = "Readouts describe the selected player, not you."
players = 2
setup = [{ chat = ".summon ${p2_character}", tier = "G" }, { wait_ms = 5000 }]
step = [
  { tool = "@entity_find", args = { name = "${p2_character}", exact = true }, label = "see-p2" },
  { tool = "@target_player" },
  { chat = ".info" },
]
[[row.expect]]
id = "p2-visible"
text = "p1's client sees p2 after the summon"
source = "tool"
at = "see-p2"
tool = "@entity_find"
args = { name = "${p2_character}", exact = true }
pointer = "/count"
op = "gte"
value = 1
[[row.expect]]
id = "names-p2"
text = "the readout is about p2"
source = "chat"
contains = "${p2_character}"
```

The runner does not put the two players in one place. Do that in the row's setup: `.summon <name>` always brings the player to the caller's instance and position, which also covers instanced maps such as `Castle_CellBlock`. Then add a visibility check before any targeting step, so a missing p2 FAILs on a clear clause rather than on a target click.

p2's account and character come from `lab-account.p2.json`. The section's `character` and `fresh` settings apply to p1 only. The row is BLOCKED, naming the reason, when that file is missing or names no character, when it names p1's own account or character, ignoring case (the second login would evict the first), when `lab_uat_run` is itself running in instance p2, or when a tool a p2 action or clause needs is not routed on p2. Where p2 comes from depends on the daemon. In a daemon that hosts several instances (`CIMMERIA_LAB_INSTANCES`), the run drives the hosted `p2` instance and holds p2's lease for the length of the run, so another agent can't drive p2 mid-row. A single-instance daemon keeps its own in-process p2. Set-up: [Parallel clients](live-research-lab.md#parallel-clients-up-to-five). Both players need to be in the same world for targeting, and the row's setup moves them there.

### Row fixtures

Without fixtures a spec is a chain: each row starts from whatever the row before it left, so one failure turns every later row into noise, and running one row twenty times means replaying character creation, the login and every earlier row twenty times. A `[row.fixture]` table puts the row's starting state in the row, so `lab uat first-session -Rows FS-P4 -RunsPerLease 20` means what it says.

The runner does not set that state in game. At character select it stages the fixture on the server's lab endpoint ([`server_stage_fixture`](live-research-lab.md#staging-a-row-fixture-server_stage_fixture)), and the server writes it when the character next presses Play, before world entry reads anything. The character enters the world exactly as a returning player with that saved state would: the `player_loaded` restore chains bind the mission dialogs, the space instance is fresh and the camera is level. The decisions behind this are D-FX1 to D-FX10 in the [lab-fixtures ledger](../analysis/lab-fixtures/README.md).

```toml
[section.profiles.praxis]            # one reusable fixture character per lab instance
alignment = "praxis"
archetype = "Soldier"
gender = "male"
first = "Uat"                        # the default

[[row]]
id = "FS-P4"
# ...
[row.fixture]
character = "praxis"                 # a [section.profiles] key
world = "Castle_CellBlock"
missions = [{ id = 622, step = 80623 }, { id = 1360, step = 4037 }]
items = [{ id = 55, count = 0 }]
tutorials_unseen = [5882]
```

**Profiles** (`[section.profiles.<key>]`):

| Field | Meaning |
|---|---|
| the key | 1 to 8 lowercase letters (`praxis`), so two keys never collapse to one character name |
| `alignment`, `archetype`, `gender` | What `lab_create_character` creates: `praxis` or `sgu`, an archetype name such as `Soldier`, `male` or `female`. None may be empty |
| `first` | The first name; default `Uat` |

A profile's character is `<first> Fx<key><instance>`: `Fx`, the key, then up to four letters of the lab instance's label, digits spelled as letters (`0` is `a`, `9` is `j`). The default instance's `praxis` character is `Uat Fxpraxisdefa`, and p2's is `Uat Fxpraxispc`. The runner creates it on first use and keeps it; its slot clearing never deletes a fixture character of the section. The character must be on a GM account, because the server stages fixtures only for those.

**Fixture fields** (`[row.fixture]`):

| Field | Meaning |
|---|---|
| `character` | A `[section.profiles]` key. Required |
| `world` | The character's saved world, a `resources.worlds` name with no whitespace. Without `position` the saved coordinates are kept, so give both when the character may have been saved in another world |
| `position` | `[x, y, z]` in server metres, as `/gmgotoxyz` takes them. Leave it out when the row positions itself in setup (see the rules below) |
| `missions` | `{ id, step }` makes the mission active on that step. `{ id, state = "not_active" }` removes it. `{ id, state = "completed" }` completes it once. `state = "active"` needs a `step`, and a `step` with any other state is an error |
| `items` | `{ id, count, container }`: exactly `count` (0 to 10) of that item design carried afterwards, in `container` `main` (the backpack, the default) or `mission` (the mission bag) |
| `tutorials_unseen` | Tutorial ids the character forgets, so the row sees them shown again |

What the server writes ([`lab_fixture/`](../../crates/base-world-entry/src/base/world_entry/lab_fixture/)), silently: no content events, rewards, Discord posts or tutorials fire (D-FX4).

- **Missions** get the row a relog at that state reads back. Active on a step: that step is current, and every step before it (by `resources.mission_steps.index`) is completed with its objectives. Completed: every step and objective completed, `repeats = 1`. Not active: the row is deleted.
- **Items:** every carried instance of a listed design (containers 1 to 15: the backpack, mission bag, bandolier, equipment and the other carried bags; never buyback, bank, auction or the team and command banks) is deleted, then `count` are granted into the container in stacks of the design's stack size. The design must allow that container: a design with `container_sets` must name it, and one without any may only go to the backpack. Unlisted items are untouched. The item write is one transaction, so when the stacks do not fit nothing changes. When the deletes take the bandolier's readied weapon and another stays on the belt, the readied slot moves to the lowest occupied one.
- **Tutorials:** their `sgw_player_tutorials` rows are deleted, so the next show is a first time again.
- **World and position:** the character's saved location.

Every check (the owner, the world, a finite position, each mission and step against the seed, each item, and a dry run of the item write) runs before the first write, so a refused fixture writes nothing. The server logs the result: INFO `lab fixture applied`, ERROR `lab fixture not applied` with `reason` `precheck_refused` (nothing written) or `write_failed` (a part failed after earlier parts wrote). Either way the character still enters the world, which is why the runner verifies before the row runs.

**Rules for writing a fixture:**

- **A fixture is "relogged at X".** It sets what you list and nothing else. When the route to X grants an item or a mission, list it: FS-P5's fixture lists the pistol (item 55) along with 622's step, because the route to that step hands it over. A step that waits for an item the character no longer has is stranded.
- **Missions advance linearly by step index.** "Active on step S" means every step with a lower `index` is done. Every content-driven mission in the seed advances this way (all 22 were checked); a mission that branches could not be put on one branch.
- **A fixture never equips.** A listed design is deleted from every carried container, the equipment slots and bandolier included, and granted back into `main` or `mission`. Listing an equipped weapon unequips it, and the count check still passes with it in the backpack. When the row needs the weapon in hand, equip it in setup.
- **Position in setup when the row must also chain.** Under fixture mode `auto` (below) a row may continue the chain instead of starting from its fixture, and then the fixture's `position` never applies. A row that must work both ways leaves `position` out and keeps its own positioning in setup (`/gmgotoxyz` or a `@move_to`). That is right in both modes: a freshly entered character has nothing selected for the GM command to move instead.
- **One player, `state = "in_world"` or `"any"`.** Validation rejects a fixture on a `players = 2` row, or on a row that starts at character select or with the client stopped. Each mission, item and tutorial is listed once, with ids above 0.
- **Match `${fixture_character}` at the end of the name.** The runner sets `${fixture_character}` to the last name of the character the row plays: the fixture character, or the chain's character when the row chains. A fixture row also sets `${character}` to it. Write SQL clauses as `player_name ILIKE '% ${fixture_character}'`, not `'%${fixture_character}%'`: instance p1's `Fxpraxispb` is a prefix of p10's `Fxpraxispba`.

**Fixture mode.** `lab_uat_run`'s `fixtures` argument decides when a row's fixture applies (D-FX10):

| Mode | A row with a fixture |
|---|---|
| `auto` (default) | Continues the chain when the section's previous row ran just before it in this run, passed, and left the client in world. Otherwise starts from its fixture |
| `always` | Always starts from its fixture, even after a passing row |
| `never` | Never stages: runs from wherever the client is, like a row without a fixture |

So a full run is the player route, and a row run alone, or after a row that failed, starts clean instead of inheriting the failure. The row's `fixture_mode` var records which path ran: `fixture` or `chain`. From the command line: `lab uat <section> -Fixtures auto|always|never` (no flag means `auto`).

**Parity.** A chained row's fixture still says what the chain should have produced (D-FX4, D-FX10). Before a chained row's setup, the runner snapshots the chain character (`${fixture_character}`, matched by the end of its name): its missions, its carried items (containers 1 to 15) and the tutorials it has seen, through three read-only `server_db_query` statements, and its world through `server_sessions`. The snapshot is written beside the row JSON as `rows/<section>/<row>.snapshot.json` (attachment `snapshot`), and the comparison is the row's `fixture_parity` (`mismatches`, `unspecified`, and `snapshot_error` when no snapshot was taken). Every mission, item count (the carried total and the named container), tutorial and world the fixture names must match: an active mission on its step, a completed one by status only, a `not_active` one with no row or a row in status 0. Missions the chain has that the fixture does not name are `unspecified` (`mission 1360: chain has it completed, fixture does not name it`), a flag that never fails the row. A mismatching snapshot is read again up to three more times, 500 ms apart, in case the previous row's last write was still landing; the last read counts. A mismatch turns a PASS into a FAIL with the reason `fixture parity: mission 622: chain has step 2113, fixture wants 80623` (mismatches joined with `; `, capped at 300 characters); on any other result it is a flag. It is also only a flag (`fixture_parity.advisory`) when the row does not continue a passing spec neighbour, which happens in `-Fixtures never` on a run's first row or after a failed row. A snapshot that fails (no lab endpoint, a query error, a truncated result, or the session list when the fixture names a world) is the reason `fixture parity not checked: ...` and never changes the result; a snapshot file that cannot be written only loses the attachment. `lab_uat_attest` keeps the parity result when it re-grades a row. A row that ran from its own fixture records no parity: the verify step already checked it.

**What a fixture session does.** A fixture row runs in a session of its own (D-FX5):

1. End any running session: `lab_logout` when in world, `lab_client_stop`, then poll `server_sessions` every second until the server stops listing the character, for up to 30 s. `lab_client_stop` kills `SGW.exe` and the server only learns of it from the dropped connection, so a login straight after could find the account still online. A name an earlier session failed to drain is drained first.
2. Start the client and log in to character select.
3. Only when the fixture character does not exist yet: free a slot (`lab_ensure_character_slot`, protecting the lab character and every profile's fixture character), create it from its profile, and give it one bootstrap session: Play, wait out the 17 s first-login hold, finish the dialog, end the session and log in again. That happens once per profile and lab instance, ever.
4. `server_stage_fixture` with the fixture and the character's name.
5. `lab_play_character`. The server applies the fixture as the character enters the world.
6. Verify the staged state at once (FX-07): read back every mission, item count (carried total and the named container) and tutorial with `server_db_query`, and the world from `server_sessions`, retrying every 500 ms for up to 5 s. This runs first because game content can change those rows soon after entry (a tutorial shown, a step advanced).
7. Find `${player_entity_id}` in `server_sessions`, then check the position with `server_entity_get` (within 2 m, the same retry). Fixture positions must be on the ground: the client moves the character, so a point in the air has fallen before the check reads it.
8. The row as usual: anchor, setup, steps, clauses, teardown.
9. End the session as in step 1, on every path, passed or BLOCKED.

Every call is on the row's `actions[]` as a setup action with `kind = "fixture"`, the first-login hold labelled `first_login_hold`, so lab-golden leaves them out of fingerprints. An error ending the session becomes a `session end: ...` flag on the row; flags never change the result.

**BLOCKED reasons.** A fixture that could not be established blocks the row before its setup, and its steps never run. It is never a FAIL: a row that ran from the wrong state proves nothing (D-FX6). Read it as a fixture or environment problem, not a product failure.

| After `fixture not established:` | Cause |
|---|---|
| `fixture needs the server lab MCP (CIMMERIA_LAB_MCP_URL/_TOKEN)` | The runner has no lab endpoint configured |
| `stage: stage refused: <why>` | `server_stage_fixture` refused it ([the refusals](live-research-lab.md#staging-a-row-fixture-server_stage_fixture)), such as `character Fxpraxisdefa is online` or `character Fxpraxisdefa is not on a GM account` |
| `server still lists <name> after 30 s` | The previous session did not drain |
| `player entity: <error>` | No session for the character after Play |
| `<check>: got <v>, want <w>; ...` | Verify found a different state, such as `mission 622 on step 80623: got 2113, want 80623`. Every failing check is listed, capped at 300 characters. Look for `lab fixture not applied` in the server log |
| A lab flow's error | A client tool failed on the way (start, login, create, Play) |

### Golden variance

A section can say what may differ between runs that still agree. The `lab golden` commands and `lab uat -DiffGolden` compare run fingerprints under these tables ([Golden runs](#golden-runs)). A golden stores the variance it was recorded with, so editing these tables changes nothing until you re-bless it. `[section.golden]` applies to every row, and `[row.golden]` extends it: a row's lists add to the section's. Globs use `*` only and match message names unless a key says otherwise.

- `ignore`: messages dropped before comparing. The built-in defaults `avatarUpdate*`, `perfStats` and `requestEntityUpdate` always apply; a spec's `ignore` only adds to them.
- `allow_unordered`: messages moved out of the ordered stream into a sorted set per row, for streams the server does not order.
- `collapse`: consecutive identical messages count once.
- `ignore_windows`: window-name globs left out of the opened and closed sets.
- `key_args`: a table from message name to the decoded fields to keep. An entry replaces the built-in one for that message, a row's entry replaces the section's for the same message, and each entry needs a name and at least one field.
- `late_bind_settle_ms`: when the packet tap bound after the row entered the world, this many milliseconds of messages after its first one are dropped (default 1000).
- `skip` (row only): leaves the row out of every fingerprint, for cleanup rows.

```toml
[section.golden]
allow_unordered = ["onEffectResults"]

[[row]]
id = "FS-99"
# ...
[row.golden]
skip = true
```

Validation rejects `skip` on the section, an empty glob or one with whitespace, a glob that is both ignored and unordered, and a `key_args` entry without a name or fields.

### The capability table

`crates/lab/src/uat/tools.rs` maps each capability to the tool that provides it and the most native tier it can claim: `@world_click` is `client_world_click` at N1, `@inventory` is the read `client_inventory`. It lists the tools on `main`, including the world tools (#1099: `@entity_find`, `@world_click`, `@target`, `@move_to`, `@camera`; and `@target_player`, which the runner expands for [two-player rows](#two-player-rows)) and the combat tools (#1100: `@use_ability`, `@combat_log`, `@die_and_respawn`, `@wait_event`, `@hotbar`), the ability lab capabilities (AB-L3: `@dummy`, `@cooldowns_reset`, `@clear_effects` and the server read `@ability_state`, below), `@window_click` (`client_window_click`: a widget by its real CEGUI name, Lua global or not, such as a frame's `__auto_closebutton__`), the spec-vocabulary pair (`@stand`, a runner action at tier G that teleports to a pose's stand point and waits for the player there, [above](#targets-poses-and-waits); `@hover_probe`, `client_hover_probe` at N1, which hovers an entity or point and scores it without clicking), and the planned ones (UI and items: `@window_read`, `@window_click_row`, `@chat_log`, `@inventory`, `@player_state`, `@item_action`, `@drag_drop`; and `@chat_send`, `@cache_files`). Write planned tools by alias in specs: when one lands under another name, the fix is one line in the table. A tool that reports how it drove the game (`native_level` as a word, `real_input` / `slash_command` / `ui_lua` / `server_shortcut`, as the world tools do, `native_cegui` (N1) / `client_ui_lua` / `native_call` (both N3), as the UI and item tools do, or as `{tier: "N1".."X"}`, as the combat tools do) overrides its table tier when it fell back lower, so a `client_target` that used `targetUnit` counts as N3, and so does a `client_drag_drop` that had to fire the drop itself.

The ability lab capabilities are the AB-L2 dot commands, typed into chat at tier G. The runner builds the line from `args`, types it, and waits up to 5 s for the command's own feedback line; a refusal (`.dummy: ...`) or no reply fails the action, so a teardown that cleared nothing is flagged, not trusted.

| Capability | `args` | Types | Confirms with |
|---|---|---|---|
| `@dummy` | `disposition` (`hostile` default, `friendly`, `clear`), `template_id` | `.dummy friendly 34` | `dummy [<id>] placed`, and stores `${dummy_id}` (a number) |
| `@cooldowns_reset` | `ability_id` (optional) | `.cooldowns reset [id]` | `cooldowns reset ...:` |
| `@clear_effects` | `name` (optional) | targets `name` with real input first, then `.cleareffects` (it acts on the selection, else the caller) | `cleareffects [<id>]`. A target that does not take fails the action with nothing typed |
| `@ability_state` | `entity_id` (default: the lab character) | the server read `server_ability_state`, for `source = "server"` clauses | |

`cargo test -p cimmeria-lab` parses and validates every committed spec, so a typo in a field name or a dangling label fails the build.

## Calibrate a row

A pose written by hand, or one a map change broke, is found again by `lab calibrate`: it sweeps stand-offs and camera angles around the target on the live client, scores each by hover, and writes the best one as a diff against the spec for you to review. It never clicks, so the mission state is the same after every candidate, and it never writes or commits the spec. It is lab use like any other: ask the user first and wait until the lab is free. Calibration may run on the colo for the lab account, one pose at a time, and on any local server (D-SV7).

```powershell
lab calibrate first-session FS-P3 -Pose frost -Prepare FS-01,FS-02,FS-P1,FS-P2
```

1. **Put the client where the row starts.** The section must be `account = "gm"` (the sweep stands with `/gmgotoxyz`), the server lab MCP must be configured, and the client must be in the world in the row's state. `-Prepare <row ids>` runs those rows first (one `lab_uat_run` on the same instance); a prepare row that does not PASS stops with exit 3, naming it.
2. **The sweep.** For each pose a `client_world_click` step of the row names, in step order (or only `-Pose`), it tries the current pose first, then every combination of radius (3.75, 3.0, 4.5 m), bearing (every 30 degrees, starting from the current bearing), yaw offset after the face (+25, -25, 0) and pitch (the pitch the face lands on, -35, -22): 325 candidates over 37 stand points. Each candidate is `@stand` (a teleport only when the point changed), `@camera` with the candidate's pitch, yaw and the pose's zoom (250 when it has none), then `client_hover_probe`.
3. **The score (D-SV6).** A candidate counts only when the aim pixel gives the wanted verdict: the target under the cursor for an entity aim; for a point aim, nothing under it that is not the target (not the avatar, another entity or a HUD window). Its score is `jitter_hits * 1000 + min(avatar_px, 400) + min(edge_px, 200) / 2`: how many of eight points 12 px around the aim pixel agree, then how far the pixel is from the avatar's body and the screen edge. Ties go to the earlier candidate.
4. **Confirmation.** The best candidate is re-stood three times, each from the opposite bearing first, and each probe must be `ok` with at least 6 jitter hits. A failed confirmation moves on to the next best, at most three winners.
5. **The budget.** `-BudgetMinutes` (1 to 30, default 15) bounds the whole call, checked between candidates. A winner already found is still confirmed for up to 2 minutes past it; a pose the budget never reached says `budget: not started`.

**Output.** One line per pose, then the diff unless `-Quiet`:

```text
frost: score 8312, confirmed 3/3, bearing 322.5 -> 330.0; diff <run folder>\first-session.diff
supply: score 7400, confirmed 3/3, unchanged
frost: no proposal: <reason> (best score 5100, confirmed 1/3)
```

`unchanged` means the winner is the pose as written: the diff then only restamps `calibrated`, so the re-measurement still leaves provenance. A line ending `(budget: swept 40 of 108 candidates)` stopped early. `-Json` prints one object under about 400 characters, `{ok, section, row, outcomes: [{pose, ok, score, confirmed, changed, diff, reason}], applied}`, with only the fields that are set and never the candidate list.

**The run folder** is an ordinary UAT run folder under the UAT root (`run.json` included), holding:

- `calibrate.jsonl`: one line per probe, `{pose, i, r, b, yaw, pitch, ok, hits, score}`, plus `confirm: <n>` on a confirmation and `miss: "stand" | "camera" | "probe"` when a candidate never reached a verdict;
- the rewritten copy of the spec file, under its own name;
- `<section>.diff`: a `git apply` diff against `docs/guides/uat-specs/<file>`, carrying every pose the call proposed.

**Review and commit by hand.** Read the diff: only `pose` lines change, and the file is re-parsed to prove nothing else did (D-SV2). `-Apply` runs `git apply` with it in this checkout, and is refused (exit 2) when the spec file has uncommitted changes, staged or not, or is not tracked. That is checked before the sweep and again before applying, and a diff for a file outside `docs/guides/uat-specs/` or naming more than one file is never applied. `-Apply` never stages or commits. Then run the row (calibration proves a hover, not that the click interacts), run `cargo test -p cimmeria-lab` through the [build lane](../../tools/build-lane/lane.ps1), and commit the spec yourself. To recalibrate several poses of one file with `-Apply`, commit between them.

| Exit | Meaning |
|---|---|
| 0 | Every pose has a proposal, or is unchanged |
| 1 | A pose found no candidate that confirmed, or no outcome came back |
| 2 | Usage, or `-Apply` refused or failed (the diff stays in the run folder) |
| 3 | The pre-flight (daemon, a free instance, no `SGW.exe` outside the lab), a prepare row, or the `lab_uat_calibrate` call failed |

The tool behind it is `lab_uat_calibrate` ([live research lab](live-research-lab.md#tools-at-a-glance)); `lab uat -Heal` calls it for a failed row's pose ([above](#run-without-an-agent-lab-uat)). The `first-session` SGU poses (FS-S3 to FS-S6) are computed from the seed's spawn rows, not calibrated, and carry no pitch, yaw or zoom until the campaign's acceptance run (lab-spec-vocab SV-15) calibrates them.

## Spec coverage

Rows authored in [docs/guides/uat-specs/](uat-specs/) (2026-09-29; `ability-mechanics` 2026-10-04), and what a `plan_only` run against today's router says (the `committed_specs_plan_against_main_tools` test pins it):

| Section | Ready now | Blocked, and on what |
|---|---|---|
| `gm-parity` | M1-1, M4-1b | M1-2 (second player, X1), M1-4 (non-GM account, X2) |
| `chat` | 6 (solo half), 9a, 9c | 7-solo (the tell syntax, L11) |
| `pets` | U1, U2b, U12 | U16 (pet-bar window names, L10), U13 (non-GM account) |
| `bank` (fresh character) | 1, 12, 2 (`@world_click`, #1099) | |
| `black-market` | U1, U22 | U0 (needs `owner_approvals: ["bm_seed"]`) |
| `crafting` (fresh Scientist) | 1, 2, 3, 5, 19 | |
| `consumables` | I1, I2 (the item tools are routed) | |
| `cooked-data` | CD3, CD6 | CD1, CD5 (`@cache_files`), CD2 (owner-only files), CD4 (`@cache_files`, `@world_click`) |
| `castle-cellblock` | T01/T02 (makes and deletes its own character) | |
| `ability-mechanics` (fresh Soldier, 2026-10-04) | 25 one-player rows: AB-U1a-c, AB-U3a-c, AB-U5 to AB-U9b, AB-U11, AB-U12, AB-U14 to AB-U19, AB-U20 and AB-U22 (`.dummy caster`), AB-U21a/b, AB-U23 | AB-U1d, AB-U2, AB-U4, AB-U13a/b (second player); AB-U10 (D-AU2), AB-U24 (D-AB03), AB-U25 (AB-E1, AB-11) |
| `first-session` (makes and deletes its own Praxis and SGU Soldiers, 2026-10-10) | All 14: FS-01 boot, FS-02 login and server select, FS-P1 to FS-P5 (Praxis: create, enter Castle_CellBlock, complete 622), FS-S1 to FS-S6 (SGU: create, enter SGC_W1, complete 1559), FS-99 cleanup. Run this section first on a new build. FS-01 to FS-P2 stay a chain (FS-P2 is the fresh-login proof); FS-P3, FS-P4 and FS-P5 gain [fixtures](#row-fixtures) with lab-fixtures FX-08, so each also runs alone on the instance's `Uat Fxpraxis...` character. The SGU rows stay a chain until they are calibrated | |
| `debug-area` (fresh Soldier, 2026-10-04) | 43 rows: DA-U1, DA-U3 to DA-U6, DA-U8 to DA-U43, DA-U45, DA-U46 (every Debug Area station, the outbound stargate, a ring trip, the starter kit, the patch-009 hotbar and a read of the seeded characters) | DA-U2 (non-GM account), DA-U7 (the store's buttons, L9), DA-U44 (a seeded account) |
| `client-limits` (GM lab character, 2026-10-11) | 7 measurement rows: CL-W1 to CL-W7, one per world (Castle_CellBlock, Castle, Harset, Agnos, Ihpet_Crater_Light, Dakara_E1, Tollana), each 90 s and at least three `client.engine.memory` samples. Five clients at once is `-Rows CL-W2 -Leases 5` (CL-M5). Numbers go to [docs/client/limits/](../client/limits/README.md) | |

110 rows are ready (22 before `ability-mechanics`, which adds 23, `debug-area` 43, `first-session` 14, `client-limits` 7, and I2 now that the item tools are routed) and 24 are blocked (11, plus its 10, plus `debug-area`'s 3). Only `first-session`'s Praxis rows have run against a live client so far (item 0 below). The first live run should take them in this order, each proving one more part of the runner:

0. `first-session`, the whole section: boot, login, character creation for both factions, world entry and each faction's first quest, graded on the server's own `sgw_mission` rows. The Praxis rows FS-01 to FS-P5 have run live (2026-10-10): calibrated on the colo, then five clean runs in a row and concurrent runs on two lab instances. The SGU rows' stand-off points are still computed, not calibrated.
1. `gm-parity` M1-1: typed chat, the `.bug` anchor and server clock, `since` chat marks, the ledger block.
2. `chat` 9a and 9c, `black-market` U1 and U22: exact-count clauses, refusals, teardown.
3. `pets` U12, then `chat` 6: setup in G, captures feeding a later command.
4. `cooked-data` CD3 and `pets` U2b: relogs inside a row, timing clauses, SigNoz attestation.
5. `gm-parity` M4-1b: cross-world travel (the chat box across load screens).
6. `bank` 1, 12 and 2 (the first world click), `crafting` 1-19, `castle-cellblock` T01/T02: fresh characters, Lua reads (the stock bindings named there are unproven: a read that errors is UNVERIFIED, and the spec is fixed from what the client shows).
7. `consumables` I1 and `cooked-data` CD6.
8. `debug-area` DA-U1 (the first world-1300 load, which DA-06 also needs), then the plaza rows DA-U3 to DA-U18 and the rest in zone order.

The [lab tooling backlog](../analysis/lab-automation/tooling-backlog.md) lists the tools the blocked rows wait for. When one lands, a spec that names it runs with no runner change. The colo also seeds `lab2` to `lab5` (#1093), and the runner drives a second lab instance for `players = 2` rows ([Two-player rows](#two-player-rows)). `Castle_CellBlock` is instanced per login, so both players move to Castle (world 8) first.

## Golden runs

A row's grade tells you whether its clauses held. A golden tells you whether the run did the same things, in the same order, as five runs everyone agreed were good. You use it after a server change: if every row still passes but the server now sends one more message, answers with a different dialog, or skips a step transition, the diff names the first place it went wrong in one line. The design and decisions are in the [lab-golden ledger](../analysis/lab-golden/README.md) (D-GD1 to D-GD10); the commands and their exit codes are in the operating guide ([`lab golden`](live-research-lab.md#commands), [`lab uat -DiffGolden`](live-research-lab.md#commands)); the tools are [`lab_golden_record` and `lab_golden_diff`](live-research-lab.md#tools-at-a-glance).

No section has a golden yet. The first one, `first-session` FS-01 to FS-P5, is recorded after that spec's migration and recalibration (lab-spec-vocab SV-13 and SV-15) merge, so it is not stale on arrival (GD-10 in the [unified UAT guide](unified-uat.md#first-session-smoke)).

### What a fingerprint holds

A fingerprint is computed offline, by pure code, from a run's evidence bundle (D-GD1). The run only captures: `lab_uat_run` with `fingerprint: true` taps every in-world row and saves entity snapshots and window samples ([The evidence bundle](#the-evidence-bundle)). So an old bundle can be fingerprinted again when the rules change, and none of it needs the lab.

Per row it holds, in this order of comparison:

| Part | What it is |
|---|---|
| Tap coverage and faults | `full`, `late:<action>`, `none` or `unavailable`; and `dropped <n>`, `replaced`, `not stopped` or `read failed` |
| Result | The row's grade (`PASS`, `FAIL`, ...) |
| Clauses | `<clause id>=<verdict>` for every required clause that is not a timing, SigNoz or human clause |
| Transitions | `mission 1360 -> 0`, `step 2113 -> 1`, `objective 3238 -> 1`, from the tap's decoded mission, step and objective updates |
| Events | The ordered client-to-server calls and server-to-client messages, one line each |
| Windows | The row's net opened and closed window sets (what a later sample shows that the first did not, and what any sample showed that the last does not) |
| Unordered | Messages the variance moves out of the ordered stream, sorted |

An event line is the direction, the tap's own message name (D-GD2: the `cimmeria-wire-log` names, not the wireclient decoders), the target of a server message, and its key arguments:

```text
c2s interact target=tag:ArmYourself_FrostBody
s2c onDialogDisplay@self dialog_id=3995 entity_id=tag:ArmYourself_FrostBody mission_id=0
```

Three rules strip what differs on every run:

- **Entity ids become labels.** The lab character is `self`, the second player `p2`, any other player `player`. An NPC is `tag:<seed tag>`, else `spawn:<spawn id>`, else `tpl:<template id>`, from the entity snapshots taken when the tap binds and before it is read. An id in neither snapshot is `?<n>`, numbered by first appearance in the row; `0` is `none`.
- **Key arguments are an allowlist** (`crates/lab/src/uat/golden/key_args.rs`). A listed message keeps only its listed fields, such as `onDialogDisplay`'s `dialog_id`, `entity_id` and `mission_id`; five client calls (`interact`, `dialogButtonChoice`, `moveItem`, `gmGotoXYZ`, `triggerClientHintedGenericRegion`) are decoded from their raw bytes. Any other message is compared by name and target only, so a new decoder or field in wire-log cannot break a golden. Item instance ids are never listed. A spec adds fields with `key_args` ([Golden variance](#golden-variance)).
- **Run variables are written by name.** The run id and the character names (`Px<run id>`) appear in decoded text as `${run_id}`, `${character}` and `${p2_character}`, substituted before the text is cut to 80 characters.

`avatarUpdate*`, `perfStats` and `requestEntityUpdate` are always dropped: their count depends on time spent, not on what the server answered.

### Record a golden

```powershell
lab golden record first-session -Rows FS-01,FS-02,FS-P1,FS-P2,FS-P3,FS-P4,FS-P5 -ServerVersion <sha>
```

1. **Install the lab from the commit you test** ([Install or update the lab](live-research-lab.md#install-or-update-the-lab)) and make sure the lab is free: five runs of the Praxis rows hold one instance for about 25 minutes.
2. **Leave the lab character's packet tap alone.** Don't call `lab_timeline` on it while recording: a read drains the tap, and a run whose tap dropped, was replaced or was unavailable is refused.
3. **Run the command.** `lab golden record` launches the runs itself (default 5, up to 20), one after another on one instance unless `-Leases` splits them, in fingerprint mode. A golden is written only when every row passed in every run, every run has the same spec file hash (which must also be the spec file's hash today), server build and `SGW.exe` hash, and every fingerprint is exactly equal under the variance (D-GD6). A single run, or runs you made earlier, never become golden.
4. **On exit 6 (the runs disagree)**, read the line. If it is timing noise, such as an ambient NPC message or a periodic update, add that message to the row's `[row.golden]` `ignore` or `allow_unordered` with a comment quoting the line, and record again with five fresh runs. Never ignore a dialog, mission, step, objective, item or chat message: those are what the golden is for.
5. **Commit** the spec's variance change and `docs/guides/uat-specs/golden/<section>.json`. The file is pretty-printed JSON, at most 128 KiB and 1 500 ordered events per row, and names its evidence runs by run id and folder (the bundles stay local). `cargo test -p cimmeria-lab` checks every committed golden parses, fits and names a section and rows that exist.

`lab golden show <section>` prints a committed golden's rows, event count, server build, `SGW.exe` hash, recorded time and evidence run ids.

### Diff a run against it

```powershell
lab uat first-session -Rows FS-01,FS-02,FS-P1,FS-P2,FS-P3,FS-P4,FS-P5 -DiffGolden
```

Every passing run is fingerprinted and compared with the committed golden under the variance the golden stored, not the spec's current tables (D-GD5). Each passing run gets `golden: match`, one divergence line, or `error: <why>` when the diff could not be made; a failed run is not compared. A divergence never changes a row's grade: when every run passed but one diverged or could not be compared, the command exits 5 (a failed row keeps 1). Nothing gates `ship.py` or CI on it (D-GD7).

### Read a divergence line

The diff stops at the first part of the first row that differs, in the order of the table above, and says it in at most 300 characters:

| Line | What changed |
|---|---|
| ``row FS-P3: expected `s2c onDialogDisplay@self dialog_id=3995 ...` after `c2s interact target=tag:ArmYourself_FrostBody`, got `...` `` | The event stream: the golden's event, the one before it as context, and what this run had there. `first` replaces `after ...` at the start of the row; `the end of the row` means one side ran out |
| ``row FS-P3: expected transition `step 2113 -> 1` ...``, ``expected clause ...``, ``expected unordered ...`` | The same shape for transitions, clauses and the unordered set |
| ``row FS-P4: expected windows opened `...`, got nothing`` | The net window sets |
| ``row FS-P2: expected result `PASS`, got `FAIL` `` | The grade (only when diffing through the API; `-DiffGolden` skips failed runs) |
| ``row FS-P2: expected tap `full`, got `late:...` `` | The tap bound at a different point |
| ``row FS-P2: tap fault `dropped 12`; the diff is unreliable`` | This run's tap lost messages: rerun it, don't read the rest |
| `row FS-P5: in the golden, not in this run` | The run did not include a row the golden holds |
| `row FS-P1: fingerprinted under a different variance than the golden's; ...` | The run was fingerprinted with other variance tables. `-DiffGolden` always uses the golden's, so you see this only from the API |

Control characters in a value become spaces, and the `after` context shrinks first so `expected` and `got` stay whole.

### Re-bless a golden

Re-bless when the behaviour changed on purpose (a new message, a new dialog), or when you edit a row's variance: a golden keeps the variance it was recorded with, so a spec edit alone changes nothing. Add `-Rebless` to `lab golden record`. A re-bless needs fresh runs (no run id the old golden names as evidence) and keeps every row the old golden held; leaving one out is refused.

## Troubleshooting

| Symptom | Cause and fix |
|---|---|
| Every row BLOCKED "typing chat needs tool ..." | The router is missing an input tool: the supervisor build is older than the flows (#1080) |
| A chat clause FAIL with "the chat box was cleared or overflowed" | More than 150 lines arrived, or a relog cleared the box: put the clause `at` an earlier step |
| A Lua clause UNVERIFIED "Lua read failed" | The stock binding name is wrong or not loaded yet; read it once with `client_lua_eval` and fix the spec |
| `.bug` anchor shows "no bookmark reply seen" | The reply took over 4 s or the account is not a GM; the row still runs, but the server clock offset is missing |
| `lab_uat_run` fails with "The operation timed out." after about a minute | Claude Code's per-request timer for HTTP MCP servers: give the HTTP `cimmeria-lab` entry in `.mcp.json` a `"timeout"` (milliseconds; `1800000` in [`.mcp.json.example`](../../.mcp.json.example)), then reconnect it with `/mcp` (#1243) |
| Every server or packet clause UNVERIFIED "lab-mcp HTTP 422 Unprocessable Entity" | A lab build older than #1243 called `cimmeria-lab-mcp` without the MCP handshake; install a current build and restart the daemon |
| A fixture row BLOCKED "stage refused: character ... is online" | The fixture character's earlier session has not ended on the server, often after a crash or a run cut off mid-row. Wait for `server_sessions` to stop listing it, then rerun |
| A fixture row BLOCKED "stage refused: character ... is not on a GM account" | The instance's lab account is not a GM. Raise its `accesslevel` to 2 or more; fixtures are lab-only (D-FX2) |
| A fixture row BLOCKED "got ..., want ..." | The server did not apply the fixture. Its log says why: `lab fixture not applied` (`reason` `precheck_refused` or `write_failed`) or `lab fixture discarded` (`not_gm`, `no_database`) |
| The run stops mid-section | An MCP call timeout (see the "operation timed out" row above): rerun the remaining rows with the same `run_dir` |
| An action fails "no entity with tag ... in space ..." | The player is in another space than the target (a wrong world, or a previous row that did not bring the player there), the target is not spawned or introduced yet, or the tag is a real seed tag but the wrong one, which the seed check cannot catch. "(the reply was capped)" at the end means a server older than the tag filter cut the space's list off: update the server |
| An action fails "tag ... matches entities ..." | Two spawns share the tag in the player's space (`spawnlist.tag` is not unique). Pick a tag that is unique there |
| `@stand` fails "stand: still n m from (x, y, z) after 5 s" | The teleport was refused (the account is not a GM, or the chat line did not send) or the point is inside geometry and the server put the player elsewhere. Check `dy_m` and the bearing against the map, or recalibrate. "no player position after 5 s" means `client_entity_find` returned no position at all |
| A `wait_server` fails "no server log '...' within n ms" | The event did not happen, or happened with other fields (a `witness_id` of another player): read the server log around the action. The runner re-applies every filter itself, so a server older than the `server_log_tail` filters still works. A failed read is a different error, `server_log_tail: ...`, and fails at once |
| A row BLOCKED "... by tag needs the server lab MCP" or "wait_server needs the server lab MCP" | `CIMMERIA_LAB_MCP_URL` and `_TOKEN` are not set for the runner: pose, tag and `wait_server` rows need the server endpoint |
| `lab calibrate` exits 2 "has uncommitted changes or is not tracked" | `-Apply` refuses a dirty or untracked spec. Commit or revert it, or apply the diff from the run folder by hand |
| `lab calibrate` refuses with "calibration needs a gm section" | The section's `account` is `non-gm`; the sweep stands with `/gmgotoxyz` |
