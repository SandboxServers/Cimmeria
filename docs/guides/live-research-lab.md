---
title: Live Research Lab — the research rulebook
type: how-to
audience: engineers and AI agents doing RE / live verification against a running SGW.exe + cimmeria-server
last_updated: 2026-10-10
companion_docs:
  - ../architecture/live-research-lab.md
  - ../architecture/client-telemetry.md
  - automated-uat.md
  - ../commands.md
  - ../analysis/ability-mechanics/lab-uat-and-telemetry.md
  - ../../tools/lab/install.ps1
  - ../../tools/lab/lab.ps1
  - ../analysis/lab-cli/README.md
  - ../architecture/observability.md
  - re-toolchain-setup.md
  - reverse-engineering-with-claude.md
  - ../operations/colo-deploy.md
---

# Live Research Lab — the research rulebook

The Live Research Lab gives an agent hands, eyes, and a body inside a
running Stargate Worlds session: a client bridge that evaluates Lua and
installs non-freezing hooks on the live `SGW.exe`, a supervisor that owns
the process and merges timelines, and a token-gated in-server endpoint
that reads live entity/witness/packet state. The design is the ADR at
[../architecture/live-research-lab.md](../architecture/live-research-lab.md);
this guide is the **operating manual and the rules of the road**.

The one rule everything else serves:

> **Do not infer client behavior when the running client can answer the
> question.**

If a probe can answer in minutes, a decompilation-only inference is a
draft, not a finding.

## The rulebook

These seven rules expand ADR §7. They apply to every session, local or
colo.

### 1. Ask the running game first

Static analysis produces a hypothesis. A probe produces an answer. If
`client_lua_eval` or a logging hook can confirm the behavior in minutes,
run it before you publish anything derived from Ghidra alone. Inference is
allowed — it is just labeled `inferred` until a probe upgrades it.

### 2. Static finds the *where*, runtime proves the *what*

Ghidra (or x64dbg for single-step work) locates the address, the Lua
global, the message handler. The lab confirms what actually happens there
at runtime. The two are complementary, not competing: never skip the
static step (you need the address), never stop at it (you need the proof).

### 3. Every finding cites its probe

A finding is not done until it records, together:

- the **probe** — the exact Lua chunk or the hook capture spec (address,
  registers/args captured, hit limit, sample rate);
- the **dev-session id** — the id every client and server event was
  tagged with, so the run is findable in SigNoz;
- the **client build** — the SGW.exe build the probe ran against.

A finding without a replayable probe is marked `inferred`. A finding with
one can be re-run by anyone. This is the difference between "we think" and
"we checked".

### 4. Non-freezing hooks only

Logging hooks, never breakpoints. The client's Mercury channel dies if the
main thread stalls past the heartbeat window, and a dead channel means the
server disconnects you — you lose the session you were measuring. Every
lab hook is a log-and-continue trampoline that cannot stall the Tick
drain. Reach for the x64dbg MCP **only** when single-stepping is genuinely
unavoidable, and expect the disconnect when you do.

### 5. Writes and native calls are experiments, not fixes

`client_mem_write` and `client_call_native` exist to *learn* — to prove
that flipping a byte or calling a function produces the effect you
predicted. Nothing that works this way ships as a memory poke. Anything
worth keeping gets re-derived as a launcher patch (client side) or a
server change (server side), with its own test. The lab is the workbench,
not the product.

### 6. On the colo, touch only the lab character and what it spawns

The colo runs with other, non-telemetry players connected. Anything that
reaches beyond your own lab character — `reloadmap`, `respawnall`, a
shutdown, a world-wide content reload — needs the owner's explicit say-so
*in that session*. Spawning an NPC next to your lab character is fine;
anything a stranger three zones away would notice is not. The token does
not enforce this; it is a discipline. (The audit trail does record it —
see "Audit" below.)

### 7. Authored from scratch

AteraLoader and AtreaRL are reference material for how a bridge *can* be
built. They are not code to extend or vendor. Everything in the lab is our
own, so the trust boundary is one we control end to end.

## Before you drive the client: the lab lease

One workstation runs one lab client per instance, and several Claude
sessions may want it. Driving it takes the
[lab lease](#the-lab-lease), which the supervisor enforces:

1. **Take the lease first.** `lab_lease_acquire {owner, purpose}` returns a
   `lease_id`; pass it to every tool that drives the client. A refusal names
   who holds the lab, what for and since when: wait, or ask that session to
   `lab_lease_release`. Take it over (`force: true` with a `reason`) only
   when the holder is gone; the takeover is logged and the holder's next
   call is told who took it. `lab_lease_status` shows the holder at any
   time, and reading tools (screenshots, UI readers, status) need no lease.
   When the daemon hosts several clients, the lease is per client: pass
   `instance` (such as `lab2`) to `lab_lease_acquire`
   ([Parallel clients](#drive-your-own-client)).
2. **Release it when you finish.** An idle lease expires after its
   `ttl_s` (default 600 s) anyway, and with no lease held the watchdog
   leaves a dead client down instead of relaunching it.
3. **Run the shared daemon.** The lease covers every session only when
   they all reach the same supervisor, the
   [shared daemon](#the-shared-daemon-cimmeria-lab---http). A stdio
   `cimmeria-lab` enforces the lease for its own session only, and its
   watchdog relaunches its client whoever killed it (seen 2026-10-04, when
   a client killed from one session came straight back under another
   session's supervisor). While stdio supervisors still run on the
   machine, find who owns a running `SGW.exe` before you touch it. The
   bridge listens inside the client on the instance's bridge port (8770 by
   default), and the supervisor that owns the client holds an open
   connection to it; the supervisor's parent chain names the session's
   `claude.exe`:

   ```powershell
   $sgw = (Get-Process SGW).Id
   $port = (Get-NetTCPConnection -OwningProcess $sgw -State Listen).LocalPort
   Get-NetTCPConnection -RemotePort $port -State Established |
     ForEach-Object { Get-CimInstance Win32_Process -Filter "ProcessId=$($_.OwningProcess)" } |
     Select-Object ProcessId, ParentProcessId, Name
   ```

   [`tools/lab/install.ps1`](../../tools/lab/install.ps1) prints the same
   chain when it refuses to install over a running client. Ask that
   session to call `lab_client_stop` rather than killing the client
   yourself.

> **The folder lock is obsolete.** Before the lease, sessions agreed to
> create `%LOCALAPPDATA%\cimmeria-lab\live.lock` as a directory before
> driving. Nothing enforced it. Do not create it any more, and delete a
> leftover one; the lease replaces it.

## How to run an experiment

The loop, concretely (ADR §4):

1. **Find a candidate** in Ghidra: an address, a Lua global, a message
   handler. Note the client build.
2. **Place a probe** with `client_hook_install` (a capture spec at the
   address) or `client_lua_eval` (a chunk). No rebuild, no relaunch.
3. **Cause the behavior.** Drive it from the real client body: a slash
   command via `client_console`, a Lua call into the UI, or a server-side
   `server_console_exec` — e.g. spawn a mob beside the lab character.
4. **Read what happened.** `lab_timeline` for the merged client+server
   window (below), `lab_screenshot` for the eyes, `server_packet_tap_read`
   / `server_witnesses` / `server_entity_get` for the server's view.
5. **Write it up** with the probe, the dev-session id, and the client
   build (rule 3). Then re-derive anything worth keeping as a patch or a
   server change (rule 5).

### Reading the merged timeline

`lab_timeline` interleaves local client events with server packet-tap
rows on **one clock**. Client events (the bridge heartbeat today; see the
note below) arrive on the dev-box clock; packet-tap rows arrive on the server clock. The
supervisor estimates the offset between the two from the packet-tap round
trip and projects the client events onto the server clock, so a client
observation and the packet that caused it line up.

Call it with the server session whose tap you want merged in:

```jsonc
// tool: lab_timeline
{
  "session_id": "<the lab character's session>",  // omit for client-only
  "window_ms": 30000,                              // lookback; default 60000
  "limit": 1000                                    // tap-row cap; default 1000
}
```

The result carries the estimated `offset` (with its RTT and method), the
resolved `window`, and the ordered `events`. Each event says whether it is
`client` or `server` sourced, its `server_ts_ms` (the merge key), the
original `client_ts_ms` for client events, and a `kind`
(`bridge.heartbeat`, `mercury.<name>`, …).

`lab_timeline` is the **low-latency local read**. SigNoz remains the
durable copy under the dev-session id — use SigNoz for anything you need
to keep or share; use `lab_timeline` for the fast inner loop.

> **What it merges today.** The client side of the timeline is still the
> bridge heartbeat ring: enough to answer "did the client's main thread
> keep ticking while the server sent packet X". The rich client events
> (CME events, `net.out`, entity lifecycle, Lua errors, and the
> `ability.*` rows below) do exist now: the supervisor drains them into
> its event store, and `client_events_read` and `client_wait_event` read
> them. `lab_timeline` does not merge that store yet: its seam
> (`drain_event_ring` in `crates/lab/src/timeline/client_events.rs`)
> returns nothing. Until it is wired, read client events with
> `client_wait_event` and line them up with the tap by hand. The clock
> offset is a coarse estimate from the packet-tap round trip (there is no
> dedicated server ping tool yet).

### Did the client accept what the server sent?

Two client events answer this without a probe (2026-09-28, see
[client-telemetry.md](../architecture/client-telemetry.md#gameplay-seams-what-the-client-accepted-and-what-its-ui-complained-about)):

- `client.cme.event` with `kind = net_in` names every inbound entity
  method the client routed (`event = Event_NetIn_onDialogDisplay`, …).
  `client.dispatch.method_dropped` names the ones it discarded. If the
  server's packet tap shows the method sent and neither event appears, it
  was lost below the dispatcher.
- `client.ui.cegui_log` at `error` carries the text of CEGUI errors,
  including `ScriptException`s from failed UI Lua binding calls.

Both reach SigNoz (`service.name = 'cimmeria-client'` and
`cimmeria.session_kind = 'lab'`, the event name in `client_target`) and, with the bridge, the local ring
(`client_events_read`, kinds `cme.event` and `cegui.log`). Both are
throttled per name: a hot name gets 8 in a burst then 4 a second, and the
next event that gets through carries `suppressed`. The backlog of further
client seams is
[docs/analysis/lab-automation/tooling-backlog.md](../analysis/lab-automation/tooling-backlog.md).

### The free Lua-VM check (SigNoz Q1)

Before you build any autologin probe, answer the open question from
the #685 spike — **is the Lua VM alive at the login screen?** — for free, with
no new probe. The client already emits a `client.lua.newstate` event when
the Lua state is created, and it ships to SigNoz under your dev-session id.
Query it:

- **Signal:** Logs (or Traces).
- **Filter:** `service.name = 'cimmeria-client'` AND
  `session_id = <your session>` AND `client_target = 'client.lua.newstate'`.
  `lab_client_start` returns the session id under `telemetry.session_id`.
- **Read:** the timestamp of the first `client.lua.newstate` relative to
  the login-screen render tells you whether the VM exists before or only
  after character select. If it fires at the login screen, Lua-driven
  autologin is viable; if not, autologin falls back to synthesized input
  until the VM appears.

This is the pattern for the whole lab: the cheapest probe is often a query
against telemetry you are already emitting.

## Tools at a glance

Supervisor (`cimmeria-lab` on the dev box: the [shared daemon](#the-shared-daemon-cimmeria-lab---http), or stdio per session). Tools that drive the client need a `lease_id` ([the lab lease](#the-lab-lease)):

| Tool | Purpose |
|---|---|
| `lab_lease_acquire` / `_renew` / `_release` / `_status` | Take, extend, give back and inspect the one-driver lab lease. A refusal names the holder; `force` with a `reason` takes over. |
| `lab_ensure_in_world` | One call from any state to in the world as a character: start, wait for the window and bridge, log in, play (create when `create` is given), finish the intro dialog, virtual focus. Returns at once when already there. See [Fewer calls](#fewer-calls-composites-and-compact-results). |
| `client_batch` | Ordered read and probe steps in one call (`lua`, `mem_read`, `call_native`, `wait`, `player_state`, `window_text`) with `$id` references to earlier results. |
| `client_ui_sequence` | One scripted UI step in one call: clicks, keys, typing, drags, window waits; reports the least native level used. |
| `lab_client_start` / `_stop` / `_restart` | Own the SGW.exe lifecycle. `_stop` waits up to 10 s until the process and its window are gone, so a start right after it is not refused as "outside the lab". `exited: false` means the wait ran out (logged as a warning); the next start will refuse that client. |
| `lab_client_status` | PID, uptime, heartbeat age, login state, crashes. |
| `lab_login` | Log in with the client's own input (Escape through the intro movies, type the account and password, pick the server) and stop at character select. Credentials default to `lab-account.json`. See [Client flows](#client-flows). |
| `lab_characters` / `lab_create_character` / `lab_delete_character` / `lab_ensure_character_slot` | Character select: list, create, delete by name, keep free slots under the 8-character cap. |
| `lab_play_character` / `lab_finish_dialog` / `lab_logout` | Enter the world (cutscene skipped), finish the open dialog with the green checkmark, `/logout` back to character select. |
| `client_ui_state` | One read: visible top-level windows, open dialog (title, text, buttons), prompts, mission tracker, chat tail. |
| `client_wait_for` | Poll a Lua boolean expression until it holds or times out (`met: false` on timeout). |
| `client_wait_event` | Wait for a client event matching a predicate (kind, name, entity id, text, fields; globs) or a window becoming visible, through a named persistent cursor. `arm: true` marks now. A timeout is `met: false`. See [Abilities, combat and event waits](#abilities-combat-and-event-waits). |
| `client_hotbar` | The action bar: each button's action (ability or item id, name, quantity), cooldown, and both key bindings with the lab key that presses them. |
| `client_use_ability` | Fire an ability by id or name like a player (bound key, button click, or the Ability window), then report what came back: sent, cast started, effect applied, refused, feedback text, cooldown. |
| `client_combat_log` | The floating-combat-text feed (every `UnitCombat` event: ability, hit type, source, target, stat changes) with a read cursor and a dealt/taken summary. |
| `client_die_and_respawn` | Optional GM setup, wait for the defeat window, read its respawners, click Release (or let it time out), verify alive, position and world. |
| `client_entity_table` | Walk the client's BigWorld entity maps: per entity id, vtable, enter count, rendered, `isReady()`; limbo and pending enter counts. |
| `lab_screenshot_region` / `lab_pixel_probe` | Crop of the capture as an image; count pixels in an RGB box (a nameplate colour, a HUD element). |
| `lab_screenshot` | The client window's game area (client rect, the UI's pixel space) → MCP image. Refuses a minimised window or an all-black frame. |
| `lab_crash_report` | Last minidump, last N commands, quarantined command. |
| `lab_timeline` | Merged client+server window (above). |
| `client_lua_eval` / `client_module_info` / `client_mem_read` | Probe tools proxied to the bridge. |
| `client_ui_click` / `client_cursor_move` | Click a named UI window (`Name` or `Parent/Child`) like a player: cursor onto its centre (native `injectMousePosition`, a real CEGUI `MouseMove`), real button messages. |
| `client_input_key` / `client_type_text` | Key presses and typing as `WM_KEYDOWN`/`WM_KEYUP` (the game translates them; Shift is virtual). Types letters, digits, space, `-_/.`, so `/logout` and `.`-console lines work through chat. |
| `client_input_mouse` | DirectInput relative motion (mouse-look) and button clicks at the UI cursor. |
| `client_input_focus` / `client_input_status` / `client_input_release` | Virtual focus (the game keeps reading input in the background), hook counters, let go of everything. |
| `client_window_read` | Read an open window: title, buttons with enabled state, text, list rows. `kind` adds the module's state for vault, trainer, crafting, pet, organization, mail, loot, DHD, dialog/blurb, greet, vendor, trade, character. See [UI readers and item tools](#ui-readers-and-item-tools). |
| `client_window_click` | Click a named widget, the widget showing a text, or a list row, with a real click at its screen point. |
| `client_chat_log` | Chat, feedback and Server Message lines with channel, speaker, colour and tabs, from the lab's chat ring through the event store, read through a named cursor. |
| `client_inventory` | Every loaded container, the bandolier's ammo and cash; `snapshot` / `diff_against` for before-and-after checks. |
| `client_player_state` | Position, world, level, experience, every stat, effects, the active weapon's ammo, the target. |
| `client_item_action` | Use, equip, unequip (a right-click on the slot), double-click, Loot All, loot one row; slash-command fallback. |
| `client_drag_drop` | A drag between slots or onto a named window through the client's own CEGUI injectors; `split` is the stock Ctrl-drag (one off the stack). Verified by an inventory diff. |
| `client_entity_find` | Entities the client knows, by id, name, mob id (the client's template id), hostility or distance: name, level, hostility, rendered, targetable, position (client and server coordinates), distance, screen point. Read-only. See [World tools](#world-tools). |
| `client_world_click` / `client_target` | Click an entity or world point in the 3D view with real input (camera turned onto it if needed, mouse-over checked for occluders), then report the target and windows it changed. `client_target` left-clicks and requires `Unit.Target` to become the entity. |
| `client_move_to` | Walk to a point, an entity or through waypoints with `W` and mouse-look, closed loop on the player's position; stuck detection, arrival radius, timeout. |
| `client_camera` | Mouse-look yaw/pitch, wheel zoom, face an entity or point. |

Server endpoint (`cimmeria-lab-mcp`, in-server, token-gated HTTP —
WireGuard-only on the colo): `server_console_*`, `server_sessions`,
`server_entity_*`, `server_witnesses`, `server_ability_state`, `server_packet_tap_*`,
`server_log_tail`, `server_content_reload`, `server_db_query`. See ADR
§3.5 for the full set; `docs/operations/colo-deploy.md` for the port.
`lab_uat_run` drives the packet tap itself for a row with `packet`
clauses ([automated-uat.md](automated-uat.md#packet-clauses)): one tap
per row from the anchor to teardown, always stopped.

### Names next to IDs in the server tools

`server_entity_get`, `server_entity_query`, `server_witnesses` and `server_sessions` pair every ID with its name, per Rule 6 of [instrumentation-discipline.md](../architecture/instrumentation-discipline.md#rule-6--every-id-field-is-paired-with-its-name). A name that does not resolve is left out of the reply, never `null`, `""` or `"unknown"`: a `template_id` with no `template_name` is a seed hole worth reporting.

| Tool | ID → name keys |
|---|---|
| `server_entity_get`, `server_entity_query` | `entity_id` → `entity_name` (character name; for an NPC its `npc_name`, else the text of its `name_id` or its template's); `template_id` → `template_name`; `space_id` → `world`; `archetype_id` → `archetype_name`; `current_target_id` → `current_target_name` (a live target in the same space) |
| `server_witnesses` | The entity's `entity_name`, `template_id` + `template_name` and `world`. `witnessed_by` and `witnesses` are lists of `{ entity_id, entity_name, template_id, template_name }`, not bare IDs |
| `server_sessions` | `entity_id` → `entity_name`, `player_id` → `player_name`, `account_id` → `account_name`, and the zone as `world` |

The cell fills the names it holds live (a character or NPC name, the space's world, the target's name); the lab endpoint fills the seed names from the NameBook (`cimmeria-names`) on the way out. The older keys stay for existing callers: `name` and `world_name` on a snapshot, `name` and `zone` on a session row. The snapshot's `spawn_id`, `class_id`, `faction` and per-stat `stat_id` have no name table and stay bare.

`client_entity_table` and `client_inventory` are answered by the lab daemon from the client's memory and UI Lua; the server never sees those replies, so it does not name them. `client_inventory` already carries the client's item names; `client_entity_table` reports only IDs (use `client_entity_find` or `server_entity_get` for the name).

Before and after for an NPC (`server_entity_get`, trimmed, illustrative IDs):

```json
// before
{ "entity_id": 900, "space_id": 3, "world_name": "Castle_CellBlock", "name": null,
  "template_id": 7001, "current_target_id": 100 }
// after
{ "entity_id": 900, "entity_name": "Jaffa Guard", "space_id": 3, "world": "Castle_CellBlock",
  "world_name": "Castle_CellBlock", "name": null, "template_id": 7001,
  "template_name": "Jaffa_Guard_T1", "current_target_id": 100, "current_target_name": "Tealc" }
```

## Driving the client with its own input

The input tools press nothing through Lua: Lua only reads where a widget is. What the live client showed (2026-09-29, cursor and drag findings 2026-10-10):

- Keys and typing are window messages. A posted `WM_KEYDOWN`/`WM_KEYUP` reaches the game; a bare posted `WM_CHAR` is ignored, because the game turns keys into characters itself with `GetKeyboardState` + `ToUnicodeEx`. The bridge makes Shift virtual by answering those two calls.
- Mouse buttons are window messages, applied at CEGUI's cursor position, not at the message's coordinates.
- The game feeds CEGUI's cursor only when DirectInput reports mouse motion: its input pump then calls `CEGUI::System::injectMousePosition`. A posted `WM_MOUSEMOVE` never reaches CEGUI, Lua's `MouseCursor:setPosition` moves the pointer without a `MouseMove` event, and the client's Lua has no `CEGUI.System` binding. So the supervisor moves the cursor by calling `injectMousePosition` natively on the game's main thread (`call_native`), which gives CEGUI a real `MouseMove` (hover, drag thresholds, minigame input), and mirrors the point into a virtual `GetCursorPos`. If the native call fails it falls back to `setPosition` and says so (`cursor_via: lua_set_position`, `client_ui_lua`). Addresses and evidence: [CEGUI mouse input feed](../reverse-engineering/findings/cegui-mouse-input-feed.md).
- A UI drag is driven with native CEGUI input: `injectMousePosition` onto the source, `injectMouseButtonDown(0)`, one `injectMousePosition` step per frame to the target, `injectMouseButtonUp(0)`. These are the calls the game's own input path makes, so those steps count as N1 (`native_cegui`). But CEGUI has never resolved a drop target in the live client, so every live drag so far landed through the explicit drop (`notifyDragDropItemDropped`, `native_call`), which makes the drag as a whole N3. A drag that moves nothing is no pass at any level (`effect_ok: false`).
- The DirectInput keyboard is created but never read. The mouse is read while the viewport has it captured (mouse-look), and only while the game thinks it is focused: virtual focus answers `GetForegroundWindow`, `GetFocus`, `GetActiveWindow`, and lets a background `Acquire` succeed.
- Launch skips the intro movies with Escape; on a new character Escape also skips the arrival cutscene, and dialogs are paged with Next to the green checkmark (`Dialog_DoneButton`).
- `lab_client_start` refuses while an `SGW.exe` the lab does not own is running, and while its own instance's client runs. A second lab client is allowed only as a named instance ([Parallel clients](#parallel-clients-up-to-five)). It injects `cimmeria-client-patches.dll` first when `CIMMERIA_LAB_PATCHES_DLL` is set, as the launcher does.

## Parallel clients (up to five)

Trade, duels, squads and teams, mail between players, player-to-player visibility and chat need two players, and parallel testing needs several clients that don't wait for each other. One workstation runs up to five lab clients at once, one per seeded lab account (`lab`, `lab2` to `lab5`). Each client is a **lab instance**: its own account, session file, bridge port, crash markers, logs and Firesky folder. One shared daemon hosts every instance, and each instance has its own lease, so five agents can each drive their own client. Evidence and measurements: [multi-client-lab.md](../reverse-engineering/findings/multi-client-lab.md).

**Why each client gets its own Firesky folder.** A running `SGW.exe` holds all 22 cooked-data archives (`Documents\My Games\Firesky\SGWGame\Cache.en-US\*.pak`) open for writing with read-only sharing for its whole life. A second client on the same folder can't open one, reports cooked version 0 for every category, and the server answers each of its logins with a full resync of about 59,000 entries, which pins its main thread for a minute or more. So the supervisor gives every client its own folder.

### Set it up

1. In `%LOCALAPPDATA%\cimmeria-lab\labd.env`, list the instances the daemon hosts (`lab env set CIMMERIA_LAB_INSTANCES default,p2,p3,p4,p5` writes it with a backup):

   ```text
   CIMMERIA_LAB_INSTANCES=default,p2,p3,p4,p5
   ```

   `default` is the unnamed instance on `lab-account.json`. A shorter list (`default,p2`) hosts fewer clients. Instance *i* in the list (counting from 0) bridges on `CIMMERIA_LAB_BRIDGE_PORT` + *i*, so 8770 to 8774 by default. In this mode the daemon ignores `CIMMERIA_LAB_BRIDGE`, and `CIMMERIA_LAB_TOKEN` applies to the first instance only. The daemon refuses to start on more than five instances, a name listed twice, or a bad name (1 to 16 letters, digits, `-` or `_`). Unset, the daemon hosts the one instance `CIMMERIA_LAB_INSTANCE` names (none: the default), as before.
2. Write the account files for the extra instances:

   ```powershell
   lab instances init                  # or: pwsh tools/lab/instances.ps1 init
   ```

   It copies `Binaries\sessions\lab-account.json` to `lab-account.p2.json` to `lab-account.p5.json`, with username `lab2` to `lab5` and character `Labtwo` to `Labfive`. Every other field, the password included, is copied unchanged and never printed. Existing files are kept unless you pass `-Force`, and `-Count 3` stops at `p3`. `lab instances status` prints each instance's account, character, whether its profile is seeded, and whether its client runs.
3. Restart the daemon so it reads `labd.env`: `lab restart` (or `pwsh tools/lab/daemon.ps1 restart`).
4. Reconnect the lab MCP server in **every** Claude session (`/mcp`). With more than one instance hosted, every tool gains an optional `instance` argument, and a session that connected before the restart still has the old tool list.

### Drive your own client

Take the lease on the instance you were given, then pass its `lease_id` to every call. The lease routes the call, so you don't need to repeat `instance`:

```text
lab_lease_acquire { owner: "...", purpose: "...", instance: "lab2" }
  -> { lease_id: "...", instance: "p2", account: "lab2", ... }
lab_ensure_in_world { lease_id: "..." }        # runs on p2
```

Each call picks its instance in this order:

1. the `instance` argument: a label (`p2`) or a lab account name (`lab2`), ignoring case. A label wins over another instance's account name. An instance whose account file can't be read can be named only by its label;
2. else the instance that issued the call's `lease_id` (this covers `lab_lease_renew` and `lab_lease_release` too);
3. else the first instance in `CIMMERIA_LAB_INSTANCES`.

A call with neither runs on the first instance. That matters for read-only tools, which need no lease: `lab_screenshot` without `instance` captures the first client, not yours. A lease is valid only on its own instance. Naming one instance and passing another instance's lease is refused, and every refusal in a multi-instance daemon ends with the instance that refused it, such as `(instance p3)`.

`lab_lease_status` without `instance` lists every instance: its label, account, lease (holder, purpose, since, expiry and recent history) and client pid. With `instance` it reports that one instance, labelled with its instance and account. It never shows a lease id.

In `labd.log`, every routed tool call runs in a `lab_call` span, each watchdog in a `lab_watchdog` span, and each lease sweeper in a `lab_instance` span, all with an `instance` field. Filter on it to follow one client.

### What each instance gets

| | Default instance | Named instance, such as `p2` |
|---|---|---|
| Session file | `sessions\current-session.json` | `sessions\instances\p2\current-session.json` (the DLL finds it through `CIMMERIA_LAB_SESSION_FILE`) |
| Bridge port | `CIMMERIA_LAB_BRIDGE_PORT` (8770) | that port + its position in `CIMMERIA_LAB_INSTANCES` |
| Credentials | `sessions\lab-account.json` | `sessions\lab-account.p2.json`, never the default file |
| Crash marker, minidumps | `sessions\` | `sessions\instances\p2\` |
| DLL logs | `cimmeria-client-*.log` | `cimmeria-client-*-p2.log` |
| Firesky folder | `%LOCALAPPDATA%\cimmeria-lab\instances\default\profile` | `%LOCALAPPDATA%\cimmeria-lab\instances\p2\profile` |
| Lease | its own | its own |

All paths are under the install's `Binaries\`, except the Firesky folder (see below).

### The per-instance profile

Every instance, the default one included, launches the game with `USERPROFILE` set to `<root>\<label>\profile`. The root is `%LOCALAPPDATA%\cimmeria-lab\instances`, or the absolute path in `CIMMERIA_LAB_PROFILE_ROOT` (`labd.env` or the environment). It must lie outside the game install: `SGW.exe` refuses a user folder inside its own install folder (`Failed to create user directory ... Force quitting`), so a profile under `Binaries\` fails at boot. Outside the install any length works, with or without spaces. Profiles seeded by an earlier build under `<install>\Binaries\sessions\instances\<label>\profile` are not used; delete them (each instance reseeds), or move one to the new root to keep its warm cache.

The client finds My Documents in one place, `SHGetFolderPathW(CSIDL_PERSONAL)`, and Windows resolves the default `%USERPROFILE%\Documents` with the calling process's own `USERPROFILE`, so the client's whole `My Games\Firesky\SGWGame` folder moves into the profile. Its one other folder lookup, Local AppData, moves there too.

The first launch seeds the profile's `SGWGame` from the real one: the top-level files (`SavedSystemOptions.xml`, `WindowStates.xml`, ...) and the `Config`, `Content` and `Cache.en-US` folders. The per-account folders (saved vars), `Logs`, `CrashDumps`, `Stats` and every other folder are left out. A warm `Cache.en-US` means the instance's first login is a routine one, not a full resync. After that the seed never runs again, so the instance keeps the cache its own logins bring up to date. The copy goes to `SGWGame.seeding` and is renamed when complete, so a seed cut short is redone on the next launch. With no real `SGWGame` folder (the game never ran on this Windows account), the profile starts empty and the server fills its cache at the first login.

A change you make to the real `Config\*.ini` doesn't reach an instance that is already seeded. To reseed one, stop its client and delete its `profile` folder.

`labd.log` records the outcome at every launch under `lab.instance`: `user_dir_ready` with `seeded` (`copied`, `already_there` or `no_source`), or a `user_dir_shared` warning with its `reason`.

- **`no_profile_root`**: neither `CIMMERIA_LAB_PROFILE_ROOT` nor `LOCALAPPDATA` is set. The client uses the shared folder.
- **`profile_root_inside_install`**: the root lies inside the install folder. Set `CIMMERIA_LAB_PROFILE_ROOT` to a folder outside it. The client uses the shared folder.
- **`profile_root_not_absolute`**: the root is relative, so it would resolve inside `Binaries\`. Use an absolute path. The client uses the shared folder.
- **`CIMMERIA_LAB_SHARED_USER_DIR=1`** (also `true` or `yes`) in `labd.env` turns this off and puts every lab client back on the real, shared folder (`reason = opted_out`). Two clients then lock each other's cache again.
- **OneDrive and redirected Documents.** The redirect works only where the `Personal` shell folder (`HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders`) is `%USERPROFILE%`-relative, the Windows default. If OneDrive Known Folder Move or a policy points Documents at an absolute path, every launch logs `reason = documents_not_redirectable` and uses the shared folder, and so does a failed registry read. A seed that fails (`reason = seed_failed`) falls back the same way. On such a machine run one lab client at a time. The durable fix, a `SHGetFolderPathW` hook in the lab DLL, is on the [tooling backlog](../analysis/lab-automation/tooling-backlog.md#backlog-lab-bridge-and-supervisor-for-the-session-that-owns-them).

### Accounts, characters, the cap and the windows

**Accounts.** A second login on the same account evicts the first client (`duplicate_login`), so every instance needs its own account. The seed has `lab` plus `lab2` to `lab5` (account ids 10 to 14, same password as the other seed accounts); keep all five in the Discord `muted_accounts` list.

**Characters.** Characters belong to an account, so each instance plays the `character` in its own account file (`instances.ps1 init` writes `Labtwo` to `Labfive`). If an account has no such character yet, create it with `lab_ensure_in_world` and `create`.

**The cap.** `CIMMERIA_LAB_MAX_CLIENTS` caps the lab clients. It defaults to the number of hosted instances (at least 2) and is clamped to 1 to 5, the five seeded accounts. The start guard still refuses when an `SGW.exe` the lab does not own is running.

**Booting several at once.** With other clients running, a new client's bridge took about 25 s to come up on 2026-10-10. The watchdog leaves a client that has never answered its bridge alone for 90 s after launch ([the watchdog](#the-watchdog-hangs-world-loads-and-the-recovery-cap)).

**Focus.** A client whose window is not in the foreground runs at below-normal priority with a 5 ms sleep per tick (`FEngineLoop::Tick`). Turn on `client_input_focus` (virtual focus) on every instance: it answers `GetForegroundWindow` per process, so none is throttled and each keeps reading its own lab input. Real keyboard and mouse still go to the window in front, so don't type while a scenario runs.

**Windows.** Every client opens at the same place and size, stacked. Arranging them is up to you; no lab tool moves a window. Screenshots use `PrintWindow` per window, so an overlapped window still captures.

### Two-player UAT rows

`lab_uat_run` drives p2 itself for a `players = 2` row ([automated-uat.md, Two-player rows](automated-uat.md#two-player-rows)). In a daemon that hosts several instances, the run drives the hosted `p2` instance and holds p2's lease for the length of the run. In a single-instance daemon it makes an in-process supervisor for the second instance on the first such row, with that instance's session file, credentials, logs and bridge port, and keeps it for the life of the server, so p2's client stays up between runs. You need:

- `sessions\lab-account.p2.json` with its own account (`lab2`) and a `character`. Without it, two-player rows are BLOCKED and the reason names the file;
- in a single-instance daemon only: `CIMMERIA_LAB_UAT_P2` to use another instance name (default `p2`), and `CIMMERIA_LAB_UAT_P2_BRIDGE_PORT` to use another bridge port (default: this instance's port + 1, so 8771).

Don't also run a separate stdio `cimmeria-lab` with `CIMMERIA_LAB_INSTANCE=p2` while the daemon drives p2. Both would own p2's session file, and whichever starts second has its client start refused. Virtual focus is turned on for each client by the runner's chat macro.

### When to use `wireclient` instead

A second player that only has to exist and answer (a duel partner, a body to be visible, a trade or squad counterpart driven with `cell_method`/`base_method`) needs no window at all: use `sparbot` or `GameSession` from `crates/wireclient` ([wireclient.md](../architecture/wireclient.md)). It has no throttling and no client cache, and needs its own account too. Use a full client when that player's UI is part of what is being tested.

## The display: screensaver and D3D

**Windowed, 16:9.** The supervisor launches the client with `-windowed ResX=1280 ResY=720` (UE3's command-line overrides, which SGW.exe parses). `CIMMERIA_LAB_WINDOW` in `labd.env` sets another size (`1600x900`) or `off` (no arguments; the client's own settings decide). The stock `SystemOptions.xml` defaults `windowedMode` to false, so a profile that never saved its options opened a borderless window at the desktop resolution (5120x1440 on the lab box, 2026-10-10), and `lab_screenshot` captured it black. The lab's own client profile (`Documents\My Games\<game>\SGWGame`) also carries a `SavedSystemOptions.xml` with `windowedMode` true, so a client started outside the supervisor opens windowed too.

Lab input goes through the client's hooked DirectInput, which never resets Windows' idle timer, so an unattended run reaches the screensaver after about ten minutes. While a screensaver owns the display, Direct3D 9 reports no adapter (`D3DERR_NOTAVAILABLE` from `GetDeviceCaps`): a client launched then dies on a "GetDeviceCaps failed" message box and an R6025 box, each with a Windows error sound, and the watchdog relaunches it until its cap. The supervisor prevents this:

- While any `SGW.exe` runs, a supervisor thread holds `ES_DISPLAY_REQUIRED` (released when none runs), so the screensaver doesn't start.
- Before every launch (including watchdog relaunches) it probes Direct3D. If the display is unavailable and a screensaver that is not password-protected is running, it dismisses it and probes again; otherwise the launch is refused with the reason (screensaver still running, password-protected screensaver, or display off/locked) instead of starting a client that can only show an error box.

## The watchdog: hangs, world loads and the recovery cap

The supervisor polls the bridge heartbeat (the client's Tick-drain counter) once a second. Every bridge call, the heartbeat included, is answered on the client's main thread, so while that thread is blocked each poll times out after 5 s with `dispatch timeout`. Two rules kill the client:

- five failed polls in a row (`MAX_HEARTBEAT_FAILS`, about 30 s of stall), unless another bridge call completed within the last 5 s (`heartbeat::BUSY_GRACE_MS`);
- polls that answer but show the counter stopped for 8 s (`HEARTBEAT_STALE_AFTER`).

A world load blocks the main thread too: loads of Ihpet_Crater_Light (world 1300) took 19-42 s on a busy machine, with a further hitch after `mapLoaded`, and were killed mid-load. So before either rule kills, the watchdog checks the **load grace** (`supervisor::stall_grace`). It reads the main thread's CPU time from outside the process (`supervisor::main_thread`, the process's first-created thread), because nothing can be asked of the client while it is blocked. A thread that used at least 2 % of the interval since the last poll is busy, which is what a load looks like; a deadlocked thread or one in a crash dialog uses none. Busy is sticky for the stall: once one sample inside it shows work, the stall stays in grace until the Tick counter advances or the cap, so a disk-bound stretch of a load doesn't get it killed. The first two samples after the last Tick advance don't count, because they still cover frames the thread rendered before it stalled. A busy stall waits up to 120 s, counted from the last Tick advance. A stall that never showed work dies at the rules above, at the old time, and a busy one dies at the cap. `CIMMERIA_LAB_LOAD_GRACE_SECS` changes the cap (in `labd.env` for the daemon; `0` turns the grace off; at most 3600).

What `labd.log` shows:

| Line (`WARN`) | Meaning |
|---|---|
| `load grace in effect` | a kill rule tripped but the stall has shown work; `stalled_ms`, `grace_ms`, `fails`, and `main_thread_cpu_ms` over `interval_ms` say how far in, and `busy_now` whether this last interval was busy too |
| `client main thread busy past the load grace; terminating` | `load_in_progress=true`: a load, or a busy loop, outlasted the cap |
| `client hung (main thread stalled and idle); terminating` | `load_in_progress=false`; `rule` is `heartbeat unreachable` or `heartbeat stale` |
| `client process gone` | the process exited on its own |

A spinning main thread also reads as busy, and so does a deadlock that strikes mid-load: both die at the cap instead of after about 30 s.

A client still booting gets a **boot grace** too (`stall_grace::BOOT_GRACE`, 90 s). Until its bridge has answered once, a failed poll within 90 s of the launch doesn't count toward either rule; the watchdog logs `bridge not up yet; boot grace in effect` at debug instead. With several clients booting at once a bridge took about 25 s to come up, and the old rules killed it as `heartbeat unreachable` at about 17 s (2026-10-10). A client that dies during boot is still caught at once (the process check runs first), the grace ends at the first answer, and a client that never answers is judged by the normal rules after 90 s.

After a kill the watchdog relaunches the client and logs back in, but only while someone holds the lease, and at most three crashes in ten minutes (`recovery cap reached (3 crashes / 10 min); not relaunching`). The count lives in the supervisor's memory, so it resets ten minutes after the oldest of those crashes, or when the supervisor restarts (`pwsh tools/lab/daemon.ps1 restart`). `lab_client_start` launches a client whatever the count.

## Fewer calls: composites and compact results

A lab-driving agent re-sends its whole context on every turn, so the cost of a session grows with the number of calls, and each result stays in that context for every later turn. Measured 2026-10-10: a four-call probe (lease, status, player state, release) by the Haiku `lab-driver` agent ended at 37.3k tokens of context, 33.9k of it there before the first result (the system prompt and the agent's 27 tool schemas). So the lab offers fewer, larger calls, and keeps results small.

**`lab_ensure_in_world {server?, shard?, character?, focus = true, create?}`** observes the client and runs the existing flows until it is in the world as `character` (default `lab-account.json`'s): `lab_client_start` when nothing runs, a wait for the window and the bridge heartbeat (up to 120 s), `lab_login` (server row `shard`, else `server`), `lab_logout` when in the world as someone else, `lab_play_character` (with `create: {alignment, archetype, gender, first?}`, a missing character is made first, `character` being its last name), `lab_finish_dialog` once for an intro dialog, then virtual focus. Already there, it returns after one observation with `already: true`. The result is `{in_world, character, world_id, pos, steps_ms}`; a failure names the step. `stop_at: "running"` stops once the window and bridge are up, on any screen; `stop_at: "character_select"` stops at character select (logging out of the world first if needed) and returns `{at, characters}`.

**`client_batch {steps, stop_on_error = true}`** runs read and probe steps in order and returns `{steps: {id: value | {error}}, ms, stopped_at?}`:

| `op` | Arguments | Value |
|---|---|---|
| `lua` | `chunk` | its return values (one: a scalar; none: `true`) |
| `mem_read` | `addr`, `len?` (default 4), `as` = `hex` / `u8` / `u16` / `u32` / `i32` / `f32` / `f64` (default `u32` for a 4-byte read, else `hex`) | the decoded value, an array when `len` holds several |
| `call_native` | `addr`, `conv?`, `args?`, `ret?` = `u32` / `i32` / `f32` / `f64` / `void` / `hex` | the return value |
| `wait` | `frames` (bridge heartbeat ticks) or `ms` (up to 30 s) | none |
| `player_state` | `fields?` | `client_player_state` without stats and effects, projected |
| `window_text` | `window`, `children?` | the window's text and visibility, and its visible children's names and texts |

A string argument that is exactly `$id`, `$id.key` or `$id+0x270` (`-4`, decimal or hex) is an earlier step's value, plus the offset: read a pointer, then `{"op": "mem_read", "addr": "$ptr+0x270", "as": "f32"}`. `${id}` inside a longer string (a Lua chunk) is replaced by the value's text. In `call_native` arguments a JSON float (`1.5`, `2.0`) is passed as its single-precision bits, `{"f32": x}` says so explicitly, and `{"f64": x}` passes a double as two words, low first. `call_native` goes through the same journaled, exception-guarded bridge call as `client_call_native` and is never replayed after a crash. A step without an `id` is named by its position.

**`client_ui_sequence {actions, stop_on_error = true}`** runs `{do: click, window, button?}`, `{do: key, key, action?}`, `{do: type, text, into?}`, `{do: drag, from, to, split?}` (ends `{container, slot}` or `{window}`), `{do: wait_window, window, gone?, timeout_ms?}` and `{do: wait, ms}` through the same paths as the single tools, and stamps the least native level used (`native_level`, `native_tier`, `native_pass`).

A composite is admitted under one lease like any guarded tool; every bridge call and posted input inside it renews the lease, and its idle waits renew it every 5 s.

**Compact results.** What an MCP client receives is compacted at the server's edge: one-line JSON; null, empty-string, empty-list and empty-object fields left out (`false` stays); floats rounded to 2 decimals; lists capped at 50 items with `<key>_total` and `truncated: true`; and the per-step native trail (`native_steps`, `trail`) left out, while `native_level`, `native_tier` / `tier` and `native_pass` / `counts_as_native_pass` stay. The cursor reads (`client_events_read`, `client_chat_log`, `client_combat_log`, `client_wait_event`) are not capped, because their cursor has already moved past what they return; their own `max` bounds them. The probe tools (`client_batch`, `client_call_native`, `client_mem_read`, `client_lua_eval`, `client_events_read`, `client_wait_event`) keep their floats exact, because a measurement rounded to 2 decimals is a different measurement. Inside a list, an empty item stays in place; a capped list with no key of its own ends with `{"truncated_total": n}`. Any tool takes `verbose: true` for the full result and `fields: ["position", "world_id"]` (or `"a.b"`) to keep only some keys; the heaviest tools list both in their schemas. A tool that declares one of these names itself keeps it: `client_wait_event`'s `fields` is its equality filter. A field that matches nothing comes back in `fields_missing`, with the result's keys in `fields_available`. At the top level, empty lists and objects stay (`windows: []` means "none open"); below it they are left out. `lab_uat_run` calls the tools in-process and grades the full results, never the compacted ones.

**Images as files.** An image block (`lab_screenshot`, `lab_screenshot_region`) is saved to `%LOCALAPPDATA%\cimmeria-lab\screenshots\<tool>-<time>.png` and the result says `image saved: <path>`; `image: true` (or `verbose: true`) returns it inline. An inline 1280x720 capture costs an agent over a thousand tokens on every later turn, and the session that briefed the agent can open the file.

**Lean schemas.** `tools/list` strips what the schema generator adds but a caller never needs (`"default": null`, `["T", "null"]` types, integer formats, `minimum: 0`, `$schema`, null-wrapping `anyOf`): about 12% of every schema, which an agent pays on every turn.

**The camera is native.** The world tools turn and zoom the camera through its own handlers (`ASGWCamera_Player` vtable thunks), found by scanning the level's actors for the player controller, so mouse-look no longer depends on DirectInput (#1243). A turn reports `native_level: native_camera` (N1). Pitch turns are clamped to ±78.75°, because the handler does not clamp the stored offset. `client_camera` takes `zoom_to` (distance 100 to 775) and returns `view_before` / `view_after` (`zoom`, `yaw_offset_deg`, `pitch_offset_deg`, `gain`). Findings: `docs/reverse-engineering/findings/cegui-mouse-input-feed.md` §10.

**Click retries.** When every point on a `client_world_click` target is covered or off screen, the tool changes the view and tries again before failing: pitch up twice, yaw each way, then zoom in. The yaw and pitch steps sum to zero, and the result lists them under `view_retries`. `rotate_camera: false` turns this off.

**Off-screen windows.** A window position saved at another resolution can leave a visible window off screen, where every click in it misses. Before a slot action, the lab moves the host window (inventory, character, vault) fully on screen and reports `moved_on_screen`. This is layout setup, not the action under test, so it stays out of the native trail.

## Client flows

To run whole unified-UAT rows (steps, checks, evidence and ledger text) rather than single flows, use `lab_uat_run`: [automated-uat.md](automated-uat.md).

The `lab_*` flow tools turn the scripts agents kept rewriting (log in, make a fresh character, play it, click through the intro dialog, log out) into single calls. Each is supervisor-side orchestration over the input tools above: every button press is a real click or key, and Lua only reads (visibility, widget text, the character list). The one Lua-driven step is picking a server row by name, because list rows are not named windows; the Select button is still clicked.

For the common case, getting in the world as the lab character, `lab_ensure_in_world` does all of this in one call ([Fewer calls](#fewer-calls-composites-and-compact-results)). `lab_login` itself now waits up to 90 s for a fresh client's window before its first step; it used to fail at `focus` after 0 ms.

A typical run on a fresh character:

1. `lab_client_start`, then `lab_login` (ends at character select with the list).
2. `lab_ensure_character_slot {protect: ["Labone"]}` (keeps a slot free; the lab account's character is always protected).
3. `lab_create_character {first, last, alignment, archetype, gender}`.
4. `lab_play_character {name: <last>}` (reports whether a dialog is open).
5. `lab_finish_dialog`, then your probes (`client_ui_state`, `client_entity_table`, `lab_pixel_probe`).
6. `lab_logout`.

Every flow returns `elapsed_ms` and a `steps` list with per-step timings. A failure is an MCP error whose text names the flow, the step and the widget or condition, and whose `data` carries the steps completed so far and the client state at that moment (visible screens and any prompt text, so a bad password or a taken name reads as the client's own message).

What the flows guard against:

- **Deleting the wrong character.** Delete and Play act on the *selected* slot, so the flow checks the client's `CharSelectMod.selectedCharacterIndex` and name after the click, and confirms a delete only when the prompt text names the character.
- **Escape opening the game menu.** `lab_play_character` presses Escape only while a movie plays or from the moment character select goes away until the world HUD shows, one press per 1.5 s poll, up to the timeout. The HUD test is `SelfStatusWin` visible and nothing else: on the first live run a new character's intro dialog (and the minimap) read visible while the Bink arrival cutscene still played, and `SelfStatusWin` appeared only after an Escape. Once the HUD is up the flow stops pressing. With `skip_cutscene: false` it never presses Escape.
- **A busy client after Create.** Right after Create the client's main thread is busy, and a bridge call can time out in its queue (`dispatch timeout`; the first live run failed there although the character was made). `lab_create_character` retries such timeouts until its 30 s budget, and once one has happened it also reads the character list, so a character that is already in the list counts as created. Any other bridge error still fails the step.
- **Closing a dialog the wrong way.** `lab_finish_dialog` pages with Next until Done (the green checkmark) shows and never uses the close X, which sends choice -1. `accept: true` presses Accept on an offer with no Done.

Typing covers letters, digits, space and `-_/.`; a password with other characters is refused before anything is typed. Names in character creation must be letters only.

`client_entity_table` reads the `GameEntityManager` singleton (VA `0x01EF244C` plus the ASLR slide) and walks its three `std::map`s with one memory read per tree node and one per entity. Hundreds of small reads once starved the watchdog's heartbeat and got a healthy client killed; the watchdog now forgives a missed heartbeat while other bridge calls are completing (`heartbeat::BUSY_GRACE_MS`), and a crash relaunch logs back in with `lab_login` and plays the `lab-account.json` character.

Proven on the first live run (colo, 2026-10-04): `lab_create_character` made a level-1 Soldier that showed at character select, and `lab_play_character` entered the world on it. That run found the two problems fixed since (the create step's bridge timeout and the in-world test that accepted the intro dialog); the fixes themselves have not run live yet.

Not yet proven on the live client (the prototype scripts these port were): the EULA path, the server-row selection by name, the `SelfStatusWin` world-HUD test for a returning character and for a new one after the Escapes, `client_ui_state`'s root-window and chat sections, and `isReady()` through the vtable.

## World tools

The world tools drive the 3D view the way a player does, so an automated UAT step exercises the client's own picking, targeting and movement code:

| Tool | Arguments | Result |
|---|---|---|
| `client_entity_find` | `entity_id`, `name` (substring, `exact`), `mob_id`, `hostility` (an `AggressionLevel` name), `max_distance_m`, `rendered_only`, `include_player`, `limit` (10), `project` (true) | Nearest first: `id`, `name`, `level`, `hostility`, `is_friend`, `mob_id`, `rendered`, `targetable`, `position` (`client` UE3 units, `server` metres, `yaw_deg`), `distance_m`, `screen`, `on_screen` |
| `client_target` | `entity_id` or `name`; `rotate_camera` (true), `force`, `allow_fallback`, `settle_ms` (1200) | Left-click; passes when `Unit.Target` becomes the entity. `result` has the target before and after, windows opened and closed; `hover_verified`; `camera` when it had to turn |
| `client_world_click` | as above plus `point`, `button` (`right`), `expect` (`target`, `window`, `any` (default), `nothing`) | Same shape; `expect: nothing` reports what changed without failing (for "nothing should happen" steps) |
| `client_move_to` | `point`, `entity_id` or `name`; `waypoints`; `arrival_m` (1.5, or 2.5 for an entity); `timeout_ms` (60000) | `arrived`, `start`, `end`, per-leg outcome and unstick attempts, the learned mouse-look gain, a path sampled every second |
| `client_camera` | `yaw_counts`, `pitch_counts`, `zoom_notches`; `face_entity_id`, `face_name` or `face_point`; `max_steps` (10) | Camera actor pose before and after; for a face, whether the target ended up centred |

Points are server coordinates (BigWorld metres, Y up, as `.location` and `server_entity_get` print them) unless the point says `"space": "client"`; the client uses UE3 units with Z up, `client = (z, x, y) * 100`.

**Native level.** Every result carries `native_level` (`read`, `real_input` or `ui_lua`), its UAT tier (`N1`, `N3`) and `counts_as_native_pass`. The only fallback is `client_target {allow_fallback: true}`: when the click does not change the target it calls the stock `targetUnit()` and reports `ui_lua` / N3, so a runner can refuse to count it. A failure is an MCP error naming the tool, the step (`resolve_target`, `on_screen`, `hover`, `observe`, `leg_N`, ...), the entity or point, the evidence at that moment (mouse-over at each height tried, target and windows before and after, position and distance left) and the steps done so far with timings.

How they work:

- **Positions** come from memory, not Lua: each entity's actor (`Entity + 0x08`) holds UE3 `AActor::Location` at `+0xDC` and the `FRotator` at `+0xE8`, which is exactly what the stock `unitPosition` and `unitOrientation` read (`FUN_00aeb960`, `BW__unknown_00e685c0`). One 0x18-byte read per actor, so the walk loop polls the player's position every 100 ms without touching Lua.
- **Names, level, hostility, mob id** come from the stock `unit*` Lua functions. Lua units are *slots*, not entity ids: `GameEntityManager + 0x130` is a `std::map<int, int>` from slot (`Unit.Target`, `Unit.MouseOver`, `Unit.Pet1` = 10, `Unit.Dialog` = 17, ...) to entity id. `client_entity_find` points private slots (7700 and up, which no stock code uses) at the nearest 48 entities with the client's own slot writer (`FUN_00c67bd0`, a journaled native call), then reads all of them in one Lua call. The slot writer raises `Event_UI_UnitMappingChanged`; the only stock handler ignores slots it does not track.
- **Projection** uses the game's own `view:worldToPixel`, which only exists inside `Events.PreRender`. The tools re-subscribe `SCTWin`'s PreRender to `LabWorld.pre`, which calls `SCTMod.onPreRender` first (combat text keeps working) and then projects the queued points. `MinimapWin` is the fallback host. If the frame counter stops moving (a UI reload), the chain is re-installed once.
- **Clicking** places the CEGUI cursor on the projected point (plus a `WM_MOUSEMOVE` when the mouse-over does not follow), reads `Unit.MouseOver` from the slot map, tries the body centre, chest, legs and head, and fails as occluded when another entity answers at every height. Then it presses and releases the real button and polls the target slot and the visible windows.
- **Walking** holds `W` and turns with DirectInput mouse-look. The mouse-look gain and its sign are learned from each motion (`steer::TurnModel`), and so is whether the pawn turns while standing (if not, it turns while walking). No progress for 2.5 s while walking is a snag: jump, strafe right (`D`), strafe left (`A`), then fail. More than 30 m in one tick fails as a teleport. Every exit releases `W`, `A` and `D`.
- **Facing** turns until the target projects within a quarter of the half-width of the centre. While it is behind the camera, it turns by the bearing from the camera actor (see below), or a quarter turn when that is unknown. It turns yaw, and pitches only when the target projects outside the vertical margin (40 px from the top or bottom edge).

### Clicking a body on the floor (measured 2026-10-10)

The first-session spec's Castle_CellBlock corpses fixed these numbers on the live client (1280x720). The spec header has the full calibration table.

- **The avatar eats clicks.** The camera orbits behind the avatar, so a target straight ahead or centred by a face sits behind the avatar's legs or head and under the HUD ring. The hover then answers the player's own entity id. Stand 4-4.5 m off and to the side of the target, never straight behind it.
- **Pitch.** A new character starts level: `pitch_offset_deg` 0, zoom 250. `pitch_counts: 200` tilts it down by 21.97 deg (gain 20, about 9.1 counts per degree; positive is down). A face that follows re-centres the target vertically when it sits outside the margin: from +22 deg, facing Frost lands on -28.8 deg every time, and that is the pitch every passing click ran at. Pitch is relative and a face can change it, so assert `view_after.pitch_offset_deg` after the last camera move rather than assume it.
- **Yaw.** A teleport keeps whatever facing the player had, so face the target first, then turn `yaw_counts: 230` (+25.3 deg). The target then projects left of the avatar (screen x about 430). Click with `rotate_camera: false`, or the click turns the camera back onto the avatar.
- **Bodies that never hover.** Some corpses (the Cellblock NID Guard) answer no entity hover at their origin. A `point` click 0.25-0.3 m above the origin hits the torso and interacts.
- **`camera_before` / `camera_after.pose` read zeros** on the live client; use `view_before` / `view_after`.

### Not yet verified on the live client

These tools were written and tested against a simulated client only (the lab lock was not available); the Ghidra facts above are static. Still true on 2026-10-04: no live run has happened. Start that run with the [AB-L0 smoke](#first-live-check-the-ab-l0-smoke), which installs fresh binaries and proves the supervisor, the server endpoint and the new ability hooks; then check, in this order:

1. `client_entity_find {include_player: true}`: the player's `position.client` equals `unitPosition(Unit.Player)` from `client_lua_eval`, and `yaw_deg` matches `unitOrientation(Unit.Player) * 360` (a fraction of a turn, per `FUN_00aeb9b0`). Compare `position.server` with `.location` to confirm the swizzle.
2. The private-slot pin: debug-hub NPC names and levels read correctly; no Lua error or UI glitch after `Event_UI_UnitMappingChanged` for slots 7700+. Check `call_native` returns cleanly for the thiscall slot writer.
3. Projection: `on_screen` and `screen` match where the NPC is in `lab_screenshot`; `SCTWin` combat text still shows after the chain is installed.
4. Mouse-over: does `Unit.MouseOver` follow the CEGUI cursor alone, or only after the posted `WM_MOUSEMOVE`? `client_target`'s `hover` step records which one worked.
5. `client_target` on a debug-hub NPC changes the target frame; `client_world_click` (right) opens its dialog or greet window.
6. Mouse-look: does DirectInput motion turn the camera without a mouse button held? If a button must be held, the tools need a `look_button` option (not implemented). Record the learned `counts_per_rad` and update `steer::DEFAULT_COUNTS_PER_RAD`.
7. The camera chain `[[[[g_pGLevel + 0x50] + 0x3C]] + 0x35C]`: is that actor the player controller (its yaw follows mouse-look) or the pawn? `client_camera` reports it as `camera_before` / `camera_after`.
8. `client_move_to` a few metres across the stasis room: the character walks with `W`, turns toward the point, stops inside the radius, and `W` is released. Then a point behind a wall to see the unstick sequence and the `leg_0` failure.
9. Movement keys: `A`/`D` strafe (per `SGWInput.ini`) rather than turn.

## Abilities, combat and event waits

These tools drive combat the way a player does and say how they did it. Every result carries a `native_level`: `N1` real input (a key press or a click through the lab's hooked input), `N2` a slash command, `N3` a call into the stock UI's own Lua (what a button handler runs, minus the input), `G` GM setup typed into chat, `X` a server shortcut. A step driven at N3 or below is not a native pass for a UAT row.

**`client_hotbar`** reads `ActionButtonMod.buttons` (ids 1 to 100, windows `ActionButtons_<id>Button`), `getActionInfo` for each bound action, and `getBindingKey('ActionButton<id>', 1|2)`. The hotbar lives only in the client's Lua profile; the server keeps no copy.

**`client_use_ability {ability_id | name}`** resolves the ability against the hotbar and the Ability window's training trees, then fires it:

1. On the hotbar: presses the button's bound key (the binding's virtual-key code mapped to a lab key), or clicks the button when the key is one the lab cannot post (`press: key | click` forces one). N1.
2. Not on the hotbar, `place: true`: puts it on the first visible empty button with the calls the drop handler makes (`getUnusedAction`, `ActionProfileMod.setButtonCurrentAction`, `setActionToAbility`), reported as N3, then presses it (N1). Posted mouse moves cannot start a CEGUI drag; the native drag `client_drag_drop` uses (above) could, but the hotbar placement has not been moved onto it yet. The placement stays in the player's profile.
3. Not on the hotbar (default `fallback: window`): opens the Ability window with its bound key (N3 `AbilityMod.onToggleAbilityWin` when unbound), selects the tree tab, clicks `Ability_Button<i>` (N1), and closes the window again. An ability outside the trees (a GM `.giveability` grant) has no window button: use `place: true` or `fallback: lua`.
4. `fallback: lua`: `useAbility(id, Unit.Target)`, N3.

No slash command for abilities is known, so there is no N2 path. The result is read from the events that follow the press, for up to `observe_ms` (default 2500), from both generations of client telemetry: the `cme.event` / `net.out` names and the ability trace's `ability.*` rows ([client-telemetry.md § Ability telemetry](../architecture/client-telemetry.md#ability-telemetry-clientability)).

| Signal | Events |
|---|---|
| sent | `net.out useAbility*`, `ability.sent` |
| refused by the client | `ability.press_dropped` (its `reason` is in `result.press_dropped`) |
| cooldown or warmup timer | `onTimerUpdate` (`cme.event` or `ability.recv`), `ability.applied` `kind = cooldown` |
| cast started | `onSequence` |
| effect applied | `onEffectResults`, `ability.applied` `kind = stat`, `stat_base`, `effect_bar_add` or `effect_bar_refresh`, `ability.shown` combat text, a combat-text record |
| refused by the server | `onErrorCode` |
| feedback | speaker-less chat lines, `ability.shown` `feedback_line` |

`result.verdict` is one of `effect_applied`, `refused`, `refused_client_side`, `cast_started` (a sequence or a timer, nothing landed yet), `refused_with_feedback`, `sent_no_reply`, `nothing_observed`. One message seen by several sources (its CME name, its `ability.recv` decode, its `ability.applied` row) counts once. Timers, stat changes and CME names carry no ability id, so a timer or a stat change from another source inside the window (regeneration, a damage-over-time) counts too; effect results, combat text, combat records and dropped presses name the ability, and another ability's are skipped. The hotbar button's cooldown after the press is included. With `place: true` the first hotbar read includes the empty buttons, which a fresh character's bar is made of.

**`client_combat_log`** wraps `SCTMod.onUnitCombat`, the stock handler for `Events.UnitCombat`. The wrapper records the raw event (ability id and name, `HitType`, source and target unit names and whether each is the player, mortal, every stat change with value and result code) into a ring in `_G.CimmeriaLab`, then calls the original, so the SCT verbosity option cannot hide anything and the player sees no change. It re-subscribes `SCTWin`'s `Events.UnitCombat` to the same stock name after wrapping, in case the event system caches the resolved function; it never touches `SCTWin`'s `Events.PreRender` subscription, which the world tools own (a window holds one subscription per event). A second wrapper on `ChatMod.onMessageReceived` feeds `chat.line` events the same way. Capture starts at the first pump in the world (any `client_combat_log`, `client_wait_event` or `client_use_ability` call); an interface reload is noticed (the ring's epoch changes) and the wrappers are reinstalled.

**`client_die_and_respawn`** is for the defeat and respawn rows. Getting killed is setup: `setup_health: n` types `/gmsethealth n 0` (G). That GM command only writes the stat; it does not run the death sequence, so the flow waits for the defeat window (`PlayerDefeatWin`, opened by the server's `onBeginAidWait`), which only a real lethal hit opens. It then reads the respawners and countdown, clicks `PlayerDefeat_Release` (N1; `respawn: auto` lets the countdown release, `none` stops at the window; picking a non-default `respawner` selects its list row through the list's own call, N3), and verifies the window closed, health above zero, and the position and world before, at death and after.

**Events and cursors.** The bridge ring behind `events_read` is drain-and-clear, so two readers would steal from each other. The supervisor is its only drainer: each pull lands in a bounded store (8192 events) with a seq that never repeats in one lab process. Readers keep cursors instead of consuming:

- `client_wait_event` starts after `since_seq`, else its named cursor (default `wait`), else the newest event (a fresh cursor never matches stale history). A met wait moves the cursor to its last match, so the next wait sees only later events, including ones that arrived between the two calls. A timeout leaves the cursor, so a later wait with another predicate still sees what this one scanned past. The safe pattern is arm, act, wait: `client_wait_event {arm: true}`, press or click, then `client_wait_event {name: "*onEffectResults"}`.
- `client_combat_log` has its own cursor (`combat_log`) and returns `next_since_seq`.
- `client_events_read` keeps a cursor too (`events_read`), so it still returns each event once, now with its seq.
- A reader whose cursor fell behind the oldest kept event gets `gap: true`. Events the bridge ring dropped while full are counted as `dropped`, and `cme.event` names past the throttle (8 burst, then 4 per second) carry a `suppressed` count.

Event kinds: `cme.event` (field `event`, e.g. `Event_NetIn_onEffectResults`; `kind` `net_in`, `action`, ...), `net.out` (`method`, `entity_id`), `entity.*`, `cegui.log` (`message`), `lua.error`, `lua.print`, `hook.hit`, the native ability rows `ability.press`, `ability.press_dropped`, `ability.sent`, `ability.sent_seq`, `ability.recv`, `ability.applied`, `ability.shown` and `sequence.dropped` ([Ability telemetry in the lab ring](#ability-telemetry-in-the-lab-ring)) from the bridge; `combat.hit` and `chat.line` (`text`, `channel`, `channel_name`, `speaker`) from the Lua rings. Predicates are case-insensitive globs: `name` matches `event`, `method`, `name`, `ability_name` or `channel_name`; `text` is a substring (or a glob with `*`/`?`) of `text`, `message` or `line`; `fields` compares field by field.

### Not yet verified on the live client (combat tools)

These tools were written against the stock UI Lua and tested against a fake bridge. The first live smoke run (colo, 2026-10-04) proved:

- `client_hotbar {include_empty: true}` on a fresh level-1 Soldier lists all 100 buttons, `ActionButtons_<n>Button`, button 1 visible, every one empty (`action_id` 0, no action type).
- `client_use_ability` sends `net.out useAbility` for a Heal Focus press, and the ability trace's rows reach the lab store within 2 s: `ability.applied` `kind = cooldown` for the warmup and cooldown timers (`timer_type` 1 and 2, both matching the server's two `onTimerUpdate` sends) and `ability.applied` `kind = stat`.

That run also found two `client_use_ability` bugs, fixed since and not yet rerun live: `place: true` read the bar without its empty buttons and so found none to place on, and the verdict ignored the `ability.*` rows (`sent_no_reply` for that press).

Still to check, in this order, after the [AB-L0 smoke](#first-live-check-the-ab-l0-smoke). Steps 1, 4 and 5 can be read against the native [`client.ability.*` rows](#ability-telemetry-in-the-lab-ring) for the same press: a `combat.hit` record should have an `ability.shown` row with `kind = combat_text` beside it, and every `net.out useAbility` an `ability.sent` with the same method.

1. `client_combat_log` once in the world: `capture.status` is `installed`, then `ok` on the next call. Fire one shot: a `combat.hit` record arrives and the floating combat text still shows. No record means the event system kept the old handler; `installed_no_resubscribe` means the re-subscribe call failed.
2. `chat.line`: an ability refusal (no target, out of range) arrives with an empty `speaker`, and a player's `/say` has one. `client_use_ability` treats speaker-less lines as feedback.
3. `client_hotbar`: `getBindingKey` returns `key` as a virtual-key code (49 for `1`), and whether it carries modifier fields.
4. `client_use_ability` on a bar ability with `press: key` and then `press: click`: both send `net.out useAbility`. Check that the Heal Focus press now reads `effect_applied`, that a damaging ability's `onSequence` and `onEffectResults` arrive (as `cme.event` names and as `ability.recv` rows), that an untaught ability gives `ability.press_dropped` and `refused_client_side`, and that the button's cooldown reads back.
5. The Ability-window path: `getBindingKey('ToggleAbility', 1)` resolves (else the window opens through the N3 toggle), the tab click switches `AbilityMod.currentTab`, and the `Ability_Button<i>` click casts. Then `place: true` on the empty bar: the ability lands on button 1 and stays there after a relog.
6. `client_die_and_respawn`: `/gmsethealth 1 0` leaves the player alive (it does not kill), a lethal hit opens `PlayerDefeatWin` with the respawner list, Release respawns, and `respawn: auto` releases on the countdown.
7. After an interface reload (anything that rebuilds the UI Lua state), the next pump reports `lua_epoch_changed` and combat capture resumes.

## Ability telemetry in the lab ring

Since 2026-10-04 the telemetry DLL that every player runs follows one cast through the client natively, with no lab Lua: the press, whether the client sent it, what it sent, what came back, what the stock handlers applied and what the UI drew. The events go to SigNoz (`service.name = 'cimmeria-client'`, the target in `client_target`) for every player. In a DLL built `--features lab-bridge` each one is also pushed onto the bridge ring, so the lab reads it within one pump, without a SigNoz round trip. The ring kind is the target without `client.`. This section is how the lab reads them; the full field lists, hook addresses and decoders are in [client-telemetry.md, Ability telemetry](../architecture/client-telemetry.md#ability-telemetry-clientability).

| Target (ring kind) | What it says | Fields you match on most |
|---|---|---|
| `client.ability.press` (`ability.press`) | The press chain knows the ability, or knows the press failed | `press_id`, `source` (`hotbar` or `lua`), `slot`, `ability_id`, `target_id`, `self_cast`, `pending_expired`, `suppressed` |
| `client.ability.press_dropped` (`ability.press_dropped`) | The client threw the press away itself, before the wire | `press_id`, `ability_id`, `reason` (`bad_args`, `no_action`, `pet_missing`, `not_known`, `pet_state_flag`, `pet_ability_flag`, `not_connected`, `class_mismatch`), `drop_site` |
| `client.ability.sent` (`ability.sent`) | The router sent an allowlisted method (`useAbility`, `useAbilityOnGroundTarget`, the pet sends, `confirmationResponse`, `trainAbility`, `resetMyAbilities`, the `gmDebug*` methods) | `send_id`, `press_id`, `press_to_sent_ms`, `method`, `ability_id`, `target_id`, `client_target_id` |
| `client.ability.sent_seq` (`ability.sent_seq`) | The packets that carried the send | `send_id`, `press_id`, `mercury_seq_first`, `mercury_seq_last`, `packets` |
| `client.ability.recv` (`ability.recv`) | An ability method arrived, payload decoded | `method` (`onSequence`, `onTimerUpdate`, `onEffectResults`, `onStateFieldUpdate`, `onStatUpdate`, `onStatBaseUpdate`, feedback-channel `onPlayerCommunication`, `onKnownAbilitiesUpdate`, `onErrorCode`, `onAbilityTreeInfo`), `entity_id`, `path`, `cast_id`, `send_id`, `send_reply`, `sent_to_recv_ms`, `decode_error` |
| `client.ability.applied` (`ability.applied`) | What the stock handler did with it | `kind` (`effect_bar_add`, `effect_bar_refresh`, `effect_bar_clear`, `effect_bar_ignored`, `cooldown`, `stat`, `stat_base`, `state_flag`), `entity_id`, `effect_id` or `ability_id`, `ui`, `outcome`, `recv_to_applied_ms` |
| `client.ability.shown` (`ability.shown`) | The UI Lua handler that drew it, and whether it failed | `kind` (`combat_text`, `combat_chat_line`, `feedback_line`, `effect_bar_ui`, `sequence_played`), `handler`, `status`, `ability_id`, `text`, `cast_id`, `interrupt` |
| `client.sequence.dropped` (`sequence.dropped`) | The `SequenceManager` threw a sequence away (now also at net-in) | `path`, `stage`, `sequence_id`, `entity_id`, `cast_id` |

`client.ability.timing` (the per-stage latency histograms) is built by the uploader's governor and goes to SigNoz only; the intervals themselves ride on the `sent`, `recv` and `applied` rows above.

**Reading them.**

- `client_wait_event` is the tool to use. `kind` takes a glob (`ability.*`); `name` matches `method`, so `{kind: "ability.recv", name: "onEffectResults"}` waits for the results; `entity_id` matches `entity_id`, `source_id` or `target_id`; and `fields` compares field by field. The `kind` field that `applied` and `shown` rows carry is a field, not the store kind: write `{kind: "ability.applied", fields: {kind: "cooldown"}}`. Arm first, then press, then wait:

  ```jsonc
  // client_wait_event
  { "arm": true }
  // client_use_ability
  { "ability_id": 597, "press": "key" }
  // client_wait_event: did the press leave the client, and with which target?
  { "kind": "ability.sent", "fields": { "ability_id": 597 }, "timeout_ms": 3000 }
  // client_wait_event: the answer, joined to the send by press_id
  { "kind": "ability.recv", "name": "onEffectResults", "timeout_ms": 3000 }
  ```

- `client_events_read` takes no filter: it returns every new event once, with its store seq, through its own cursor. That cursor belongs to whoever drives the lab, which is why the UAT runner never calls it.
- A UAT `client_event` clause names the target with its prefix (`event = "client.ability.sent"`) and the runner strips it. The runner reads through `client_wait_event` with an explicit `since_seq`, so it shares no cursor with you. Clause fields: [automated-uat.md, Client event clauses](automated-uat.md#client-event-clauses-and-cast_id).
- `client_use_ability` reads both the CME names (`net.out useAbility*`, `onEffectResults`, ...) and the `ability.*` rows for its verdict ([the signal table](#abilities-combat-and-event-waits)); the [first live check](#first-live-check-the-ab-l0-smoke) compares the two.

**The throttle.** Every `client.ability.*` name gets a burst of 8, then 4 a second, and the next event of that name that gets through carries the count it dropped as `suppressed`. A press and its answer are kept or dropped together: rows with a `press_id` (`press_dropped`, `sent`, `sent_seq`) follow the decision made for their `press` row, and suppressed presses are counted only on the *next* `press` row. So a dropped `sent` leaves no row of its own, and a suppression at the very end of a window, with no press after it, cannot be seen. `recv`, `applied` and `shown` have their own buckets per method or kind, split by the local player (`self`) and everyone else (`other`), so NPC traffic cannot starve your own rows. Two more loss signals are separate from the throttle: `dropped` in a read (the bridge ring of 4096 was full) and `gap` (the supervisor's store of 8192 evicted past your cursor). Do not bound counts over a burst of more than 8 presses a second.

**The kill switch.** `CIMMERIA_CLIENT_HOOKS_DISABLE` leaves named inline hooks uninstalled: a comma-separated list of the `hook` names that `client.hooks.inline.installed` reports, where a trailing `*` matches a prefix and `all` matches every inline hook. `ability_*` leaves out the 23 ability hooks (12 for presses and sends, 11 for what the handlers applied), and `sequence_net_in` the `onSequence` net-in drop. The `recv` and `shown` rows ride on hooks that existed before (the `onEntityMethod` detour and the `lua_pcall` / `lua_call` detours), so `ability_*` does not stop them. The DLL reads the variable once, when it installs its hooks. The client inherits the supervisor's environment, so put the variable in the `env` block of the `cimmeria-lab` MCP entry, reconnect the MCP server, then `lab_client_restart`. If a client misbehaves on the wire after login, try `ability_nub_send,ability_channel_send,ability_seq_next` first: an extra frame around a Mercury send is the kind of change that once broke `processOrderedPacket` ([client-telemetry.md](../architecture/client-telemetry.md#ability-telemetry-clientability)).

### Not yet verified on the live client (ability hooks)

Every ability anchor was read statically from the QA `SGW.exe`, and the decoders are tested against synthetic wire bytes and the checked-in definitions. The first live smoke run (colo, 2026-10-04) loaded them: `ability.press`, `ability.sent`, `ability.sent_seq` and `ability.applied` rows (cooldown timers of type 1 and 2, a stat update) were observed for a Heal Focus press. `ability.recv` was broken in that run and is being fixed in #1203; `ability.shown` and `ability.press_dropped` were not exercised. Still to check, with one hotbar press each:

1. `client.hooks.inline.installed` lists the 12 press-and-send hooks (`ability_use_action`, `ability_use_ability`, `ability_slot`, `ability_lookup`, `ability_send_builder`, `ability_pet_action`, `ability_pet_send`, `ability_start_entity_message`, `ability_start_proxy_message`, `ability_channel_send`, `ability_nub_send`, `ability_seq_next`), the 11 apply hooks (`ability_effect_*`, `ability_cooldown_*`, `ability_stat_*`) and `sequence_net_in`.
2. A hotbar press of a known ability gives `ability.press` then `ability.sent` with the same `press_id`, and an `ability.sent_seq` whose range contains the server's `use_ability_recv` `mercury_seq` (28-bit wrap: the join rule is in client-telemetry.md).
3. `client_target_id` on `ability.sent` equals the server's `setTargetID` (the field is `client_target_inferred` until this holds).
4. A press of an ability the client was never taught gives `ability.press_dropped` with `reason = not_known`; a non-GM character's `/gmdebugcombat` gives `class_mismatch`.
5. `ability.recv` `onEffectResults` carries the server's `cast_id`, an `ability.applied` `cooldown` row has `outcome = applied`, and an `ability.shown` `combat_text` row arrives beside the lab's own `combat.hit` record.
6. The client stays connected and in sync through a minute of presses. If not, use the kill switch above and record which hook it was.

### First live check: the AB-L0 smoke

AB-L0 of the [ability-mechanics lab plan](../analysis/ability-mechanics/lab-uat-and-telemetry.md#part-4-lab-tools-and-the-uat-run-ab-l-ab-r) is the first thing any live run does. It needs no code unless it fails:

1. [Install from `main`](#install-or-update-the-lab) and restart the daemon (`lab restart`; with a stdio supervisor, reconnect the MCP server), then take the [lab lease](#before-you-drive-the-client-the-lab-lease). Restart first: the restart closes the lab clients and refuses while a client is leased, and the lease would not survive it anyway. Record the commit from `installed-from.txt`.
2. Point the lab at the colo: the colo row in `lab-account.json`, and `CIMMERIA_LAB_MCP_URL` / `CIMMERIA_LAB_MCP_TOKEN` at its WireGuard-only endpoint ([colo-deploy.md](../operations/colo-deploy.md)).
3. `server_sessions` answers. A 403 "Host header is not allowed" is gap G6 in the plan: the endpoint's allowed-hosts setting.
4. `lab_uat_run { plan_only: true }` matches the [spec coverage table](automated-uat.md#spec-coverage): no row BLOCKED on a missing tool that the table says is routed.
5. One hotbar press, checked against the list above. Write the SHAs and the result into the plan's ledger.

## Ability lab commands

The ability rows need a target that holds still, a way to reset cooldowns and effects between presses, and a server readout to compare with the client. These commands do that from the lab character's chat (`client_type_text`, or the UAT runner's `chat` setup lines, tier G). They are setup and readback, never the graded press. Details, refusals and limits: [commands.md](../commands.md).

| Command | Use it for | UAT capability |
|---|---|---|
| `.dummy [hostile\|friendly\|clear] [templateId]` | A target 3 m in front of you: 1,000,000 Health, no AI, no respawn; template 34 unless you name one. At most 4 each, gone after 10 minutes, on logout or on `.dummy clear` | `@dummy` (stores `${dummy_id}`) |
| `.dummy caster <abilityId> [intervalSecs]` | The same hostile dummy, casting that ability at you every interval (default 8 s) through the real NPC launch, so its warmup can be interrupted and its effects cleansed. The ability UAT uses `.dummy caster 1354` | `@dummy` |
| `.effects [target]` | Your selected target's ability state (else yours) in chat: Health, Focus, state field, warmup, cooldowns, pulses, ledger entries, held flags. Read-only | `@ability_state` reads the full form, `server_ability_state` |
| `.cooldowns [reset [id]]` | List your cooldowns, or clear them all or one; the client gets the clear timer, so the hotbar sweep stops too | `@cooldowns_reset` |
| `.cleareffects [target]` | Strip every timed and pulsing effect from your selected target (else you), reason `cleansed` | `@clear_effects` |
| `/gmgiveability <id>`, `/gmgiveallabilities`, `/gmresetabilities` | Teach one ability, the archetype's whole tree, or reset to the starters; no training points | `chat` setup line |
| `/gmsetgodmode <0\|1>` | Take no Health or Focus damage from hits and DoTs (they show "Absorbed"); yourself only, off at relog | `chat` setup line |
| `/gmsetmobabilityset <setId>` | Give your selected mob an NPC ability set until it respawns | `chat` setup line |
| `/gmdebugcombat`, `/gmdebugcombatverbose`, `/gmdebugability <id>`, `/gmdebugabilityonmob <id>`, `/gmdebugheal` | Combat and ability debug lines, delivered as feedback-channel chat lines that start `[CD #<cast id>]`. The client sends them only from a GM character ([native-combat-debug.md](../reverse-engineering/findings/native-combat-debug.md)) | `chat` setup line; read with `client_chat_log` or an `ability.recv` `onPlayerCommunication` row |

Every state change also logs one `abilities.gm` row, so SigNoz shows who changed what.

**Colo rule 6.** These commands act on you, your selection or your own dummies. On the colo, keep it that way: place your own dummies and clear only your own (`.dummy clear` cannot remove anyone else's); select only your lab character or your own dummy before `.cleareffects`; use `/gmsetmobabilityset` only on a mob you spawned; and leave other players' mobs and characters alone, even for a read. `.cooldowns` never touches another player.

## The ability UAT section

The ability-mechanics rows are [docs/guides/uat-specs/abilities.toml](uat-specs/abilities.toml), section `ability-mechanics` (AB-U1 to AB-U25 of the [unified UAT guide](unified-uat.md#ability-mechanics)). It runs on the colo with the GM lab account and a fresh Soldier per run. Each row places its ability on the bar in setup (N3), resets its cooldown, then makes the graded press with the bound hotbar key (N1); its clauses check the client's own `client.ability.*` rows, the server's `server_ability_state`, the packet tap and SigNoz.

**Before you run it:** the lab lease (pass its `lease_id`, or let the run take its own), the AB-L0 smoke, `CIMMERIA_LAB_MCP_URL` and `CIMMERIA_LAB_MCP_TOKEN` set (without them every `server` and `packet` clause is UNVERIFIED), and, for the two-player rows, `lab-account.p2.json` naming `lab2`, which must be a GM account for p2's own `/gm*` lines ([Parallel clients](#parallel-clients-up-to-five)).

**Run it in batches.** Long rows can outlast an MCP call's timeout, so take about five rows at a time on one `run_dir`:

```jsonc
// lab_uat_run: see what can run first
{ "sections": ["ability-mechanics"], "plan_only": true }
// then the first batch; pass the returned run_dir to every later batch
{ "sections": ["ability-mechanics"], "rows": ["AB-U1a", "AB-U1b", "AB-U1c", "AB-U3a", "AB-U3b"],
  "server_version": "<service.version>" }
```

**Attest SigNoz.** The runner has no SigNoz client: every SigNoz clause comes back PENDING with its filter. The filters name the cast by `${cast_key}`, which the runner fills in after each press: the cast id together with its caster (`cast_id = C AND (entity_id = E OR source_id = E OR invoker_id = E)`), because a cast id is unique per caster, not per server. Run each filter with the SigNoz MCP, then `lab_uat_attest` the rows; finish with `lab_uat_report { ledger: true }`. How the runner finds the cast (`client_recv`, `seq_join`, `press_window`): [automated-uat.md](automated-uat.md#client-event-clauses-and-cast_id).

**The server tools it uses.** `server_ability_state` (through `@ability_state`, defaulting to the lab character) for every `server` clause; `server_packet_tap_*` for the `packet` clauses (one tap per row, anchor to teardown); `server_log_tail` for the `seq_join` and `press_window` cast lookups; `server_db_query` where a row compares damage with the seed; `server_sessions` to find the character's session.

**What can run.** 25 one-player rows are ready. The five two-player rows (AB-U1d, AB-U2, AB-U4, AB-U13a/b) run once p2 is set up and are BLOCKED, with the reason, until then; AB-U10 (`.qr`, D-AU2), AB-U24 and AB-U25 are BLOCKED on open decisions. The current list is the `ability-mechanics` row of the [spec coverage table](automated-uat.md#spec-coverage). `.qr` does not exist, so a miss on a graded hostile press fails a damage or debuff row: check `qr_rolled` for the row's cast in SigNoz before filing it. A FAIL becomes a fix packet in the plan's ledger.

## UI readers and item tools

These tools serve automated UAT: read what the client shows, and act on items the way a player does. Readers use the stock UI's own Lua bindings (`getItemIDForSlot`, `getUnitStat`, `getEffectInfo`, `getLootInfo`, the CEGUI window tree); actions put the UI cursor on a widget's screen rectangle and send real button and key messages through the input path above. Container, stat and channel ids are read from the client's `Container`, `Stat` and `UIChannel` tables at run time, never hard-coded.

**Native level.** Every result carries `native_level`, `native_tier` (the UAT matrix labels the world and combat tools use) and a `native_steps` list. The levels, most native first: `real_input` (N1, key and mouse messages), `native_cegui` (N1, the client's own CEGUI input injectors called natively: the calls its input pump makes, minus the DirectInput read in front of them), `slash_command` (N2, a line typed into chat, which the client parses and sends itself), `client_ui_lua` (N3, a call into the stock UI's Lua), `native_call` (N3, a native call that stands in for a decision the UI should have made itself, such as firing a drop the client did not resolve). The overall level is the least native step; `native_pass` is true only when every step was N1, so a UAT runner can refuse to count a pass that fell back. Readers report `native_tier: "read"` and `mode: "read"`. A failure is an MCP error naming the tool, the widget or step, and the elapsed time.

| Tool | How it drives or reads | Fallback (reported) |
|---|---|---|
| `client_window_read` | Walks the window's subtree: name, type, visibility, enabled state, text, screen rectangle, list rows (Listbox and MultiColumnList, with selection). `kind` adds the module's state: loot items (`getLootInfo`), trainer abilities with id, cost and trainable (`TrainerMod`), mail headers, vendor stock (`getVendorItemInfo`), greet topics, crafting permission per craft type, vault items, pet, DHD active. | — |
| `client_window_click` | Real click at a named widget, at the widget showing a text, or at a list row (the row's point comes from the list's `getItemAtPoint`, else from item heights), then reads the row's selection back. | Row not selected after the click: `setItemSelectState` (`client_ui_lua`). Scrolling a row into view is `ensureItemIsVisible` (`client_ui_lua`). |
| `client_chat_log` | Pumps the lab's one chat capture, the `chat.line` ring (see [Abilities, combat and event waits](#abilities-combat-and-event-waits)), into the event store and reads it through its own cursor (`chat_log:<name>`), so it never takes lines from `client_wait_event`. Adds the channel name (`UIChannel`), speaker-flag names, and the colour and tabs the chat window uses for the channel. | — |
| `client_inventory` | Every container the client has loaded, the bandolier's ammo per slot, cash. `snapshot` / `diff_against` give slot changes, per-item quantity deltas and the cash delta. | — |
| `client_player_state` | Position, facing (`unitOrientation` is a 0..1 turn fraction; `heading_deg` too), world, level, experience, every stat, effects, the active weapon's ammo, the target. | — |
| `client_item_action` | `use` / `equip` / `unequip` / `rightclick`: a right-click on the item's slot, which the stock UI turns into `contextSensitiveUseItem`. The inventory or character window is opened with its bound key (`getBindingKey`) and the right tab and the All filter are clicked first. `loot_all` clicks Loot All; `loot_slot` pages the loot window and double-clicks the row. | Window toggle unbound: the window's toggle handler (`client_ui_lua`). Slot beyond the 40 visible: the scrollbar (`client_ui_lua`). Slot cannot be put on screen: `/useitem`, `/equip`, `/lootitem` (`slash_command`). |
| `client_drag_drop` | Native CEGUI input (`native_cegui`): `injectMousePosition` onto the source slot, `injectMouseButtonDown(0)`, the cursor walked to the target one `injectMousePosition` per frame (a bridge round trip between steps; at least 3 steps, since the move that crosses the threshold only starts the drag), `injectMouseButtonUp(0)`. The source must be a CEGUI `DragContainer` (checked by vtable before anything is pressed). `split` holds Ctrl: the stock inventory pulls one item off the stack on a Ctrl-drag; its Shift-drag split is an unimplemented `TODO`. A press CEGUI does not take is refused. Verified by an inventory diff: `drag_started` (the source container's own dragging byte, `+0x23e`), `drop_target_resolved` (the container's `d_dropTarget` before release), `drop_notified`, `moved`, `snap_back`, and the raw `diff`. With slot ends `moved` means the source slot's item reached the target slot (`moved_check: slots`); with a named-window end any inventory change counts (`any_change`). Nothing moved: `effect_ok: false` and `native_pass: false`, and the UAT runner fails the action. | No drop target at the end of a drag of the source container itself (every live drag so far): fires the target window's `DragDropItemDropped` (`notifyDragDropItemDropped`) before release, so the stock handlers send `moveItem` (`native_call`, N3). A drag `getDragInfo` reports for some other item never triggers it. `allow_fallback: false` skips it and the drag snaps back. |

Quirks to keep in mind:

- The chat log only has lines shown after the chat ring was first installed (by any pump); read `client_ui_state`'s chat tail for older ones. Centre-screen splash text does not go through the chat handler and is not in the log.
- Items in the Mission and Crafting tabs share `InventoryWin` with Main: a drag between two tabs of the same window is refused, because both ends cannot be on screen at once.
- Vault slots can be read and dragged only while the vault window is open at a banker.
- `d_dropTarget` stays null over an inventory slot in the live client (2026-10-10), so a drag only lands through the explicit drop. Why is open: the leading suspect is that no window from the slot up to the GUI sheet has `DragDropTarget` set ([findings, section 8](../reverse-engineering/findings/cegui-mouse-input-feed.md#8-follow-up-d_droptarget-stays-0-after-a-live-injected-drag-2026-10-10)). When that is fixed, drags report `drop_target_resolved: true` and N1 with no change to the tool.

**Not yet proven on the live client.** These tools were built and unit-tested against fixtures and a Lua 5.1 mock of the bindings (every reader chunk loads and runs under Lua 5.1), without a live client; still true on 2026-10-04. After the [AB-L0 smoke](#first-live-check-the-ab-l0-smoke), the first live session should check:

1. `client_chat_log`: a typed `/say` line appears once, with `channel: "Say"`, the chat window's colour and tab; a `/tell` error arrives as a `Feedback` line; the chat window still shows every line; a second `client_chat_log` with the same cursor returns nothing new, while `client_wait_event {kind: "chat.line"}` still sees the line.
2. `client_window_read {kind: "loot"}` and `{kind: "trainer"}` at the debug hub's crate and trainer: items and abilities match the windows.
3. `client_window_click` on a `Trainer_Choices` row: `method: "item_at_point"` and `selected: true` with no `fallback`.
4. `client_item_action {action: "use"}` on a consumable: the inventory window opens with its bound key (`getBindingKey('ToggleInventory', 1)` returns a key), the right-click consumes one (diff `delta: -1`), `native_level: "real_input"`.
5. `client_item_action {action: "equip"}` and `unequip`: the item moves between `Main` and its equipment container.
6. `client_drag_drop` between two `Main` slots: `drag_started` true and `moved` true, with `drop_target_resolved` and `drop_notified` saying which drop landed it; with `split: true`, one item moves (Ctrl reaches CEGUI's button state as 9).
7. `client_item_action {action: "loot_all"}` on the loot crate: loot count drops to 0 and the items appear in the diff.
8. `client_player_state`: `position` matches `unitPosition`, `stats.Health` and the active weapon's ammo match the HUD.

## Trust, audit, and the colo

- **Lab telemetry reaches SigNoz under a real token.** Each
  `lab_client_start` mints a dev-session token with `session_kind = lab`
  from the server the client logs into: the login URL of the server row
  (the `server` argument, else `lab-account.json`'s `server`) in the
  install's `LoginInternal.lua`, whose login port serves the mint route,
  as it does for the launcher. With one row, that row is used whatever
  the name. `CIMMERIA_LAB_SERVER_URL` overrides the server. The token and
  the server's upload endpoint go into `current-session.json`
  (`CIMMERIA_LAB_UPLOAD_ENDPOINT` overrides the endpoint). The client's
  events then land in `service.name = 'cimmeria-client'` tagged
  `cimmeria.session_kind = 'lab'`. **A failed mint stops the launch**,
  and the error says why (a server without
  `CIMMERIA_TELEMETRY_HMAC_SECRET` answers 500).
  `CIMMERIA_LAB_TELEMETRY=optional` launches without uploads instead, and
  the start result's `telemetry` block reports the reason. Until
  2026-10-03 the default was `http://127.0.0.1:8443`, a local admin API,
  so a lab client playing on the colo launched with telemetry silently
  off. The telemetry token is not the bridge token, which never
  leaves the machine. The minted token is cached next to the session file
  (`lab-telemetry-grant.json`, one per lab instance) and reused while it
  has at least 30 minutes left, so relaunches don't mint again: the server
  allows 30 mints per install id per window, and a relaunch-heavy repro
  campaign that minted every launch ran out and uploaded nothing.
- **Activation is double-gated.** The bridge code exists only in a
  telemetry DLL built `--features lab-bridge` (off by default), and even
  then starts only when `current-session.json` carries a `lab` block that
  only the supervisor writes. A DLL handed to anyone else physically lacks
  the bridge.
- **No server→client command relay exists.** The agent drives the client
  from the same box as the client. Other players' clients are unreachable
  by construction.
- **Fail-closed server endpoint.** `cimmeria-lab-mcp` starts only when
  both `CIMMERIA_LAB_MCP_BIND` and `CIMMERIA_LAB_MCP_TOKEN` are set, with
  a token of at least 32 bytes. Absent env = endpoint absent.
- **Audit is the log trail.** Every server-endpoint tool call emits one
  `info` event on `lab.tool_call` (tool, arguments, caller, outcome),
  which reaches SigNoz through the existing exporter. On the colo, that
  log line is the whole audit trail per the owner's decision — which is
  why rule 6 matters.

## Setup

The lab is wired into `.mcp.json` alongside Ghidra and x64dbg — see
[re-toolchain-setup.md](re-toolchain-setup.md) for the entries and the env
vars, and [reverse-engineering-with-claude.md](reverse-engineering-with-claude.md)
for where the lab sits in the RE workflow (the "ask the running game"
path). Lab-account credentials live in
`<install>/Binaries/sessions/lab-account.json`, gitignored. `lab_login`
types them with native key presses, which cover letters, digits, space and
`-_/.` only: an account name or password with any other character (`!`,
`@`, ...) fails `lab_login` and the post-crash relogin, so give the lab
account a password inside that set.

`lab_client_start` needs the i686 `sgw-start32.exe` helper beside
`cimmeria-lab.exe` (or at `CIMMERIA_LAB_START32`). The supervisor is 64-bit
and cannot inject into the 32-bit client itself; the helper starts
`SGW.exe` suspended, injects the bridge DLL at the game's bitness and
resumes it (#985). The install script below builds and places it.

### Install or update the lab

The lab runs from installed copies, not from a target dir, so a build of
`main` changes nothing until you install it. Nothing refreshes them on
their own: on 2026-10-04 the installed DLL still dated from 2026-09-29.
Use [`lab install -From <worktree>`](#the-lab-command): it runs
[`tools/lab/install.ps1`](../../tools/lab/install.ps1) from that worktree,
which builds the four pieces and installs them, then refreshes the `lab`
command's own copy:

| Built (through the build lane) | Installed to |
|---|---|
| `cimmeria-lab` (release, host) | `%LOCALAPPDATA%\cimmeria-lab\bin\cimmeria-lab.exe` |
| `cimmeria-client-telemetry --features lab-bridge` (release, i686) | `%LOCALAPPDATA%\cimmeria-lab\bin\cimmeria_client_telemetry.dll`, and `<CIMMERIA_LAB_INSTALL_DIR>\Binaries\cimmeria-client-telemetry.dll`, the one the supervisor injects unless `CIMMERIA_LAB_DLL` names another |
| `cimmeria-start32` (release, i686) | `%LOCALAPPDATA%\cimmeria-lab\bin\sgw-start32.exe` |
| `cimmeria-client-patches` (release, i686) | `%LOCALAPPDATA%\cimmeria-lab\bin\cimmeria_client_patches.dll` |

1. Take the [lab lease](#before-you-drive-the-client-the-lab-lease).
   The script refuses while any `SGW.exe` runs and prints its PID and the
   supervisor that owns it; ask that session to `lab_client_stop`.
2. Install from the worktree you want:

   ```powershell
   lab install -From C:\src\Cimmeria\.claude\worktrees\lab-fix
   ```

   `lab install` passes `-InstallDir` from `labd.env`'s
   `CIMMERIA_LAB_INSTALL_DIR` and `-SkipBuild` when you give it. To see the
   plan first, or to install without the `lab` command, run the script
   itself from that checkout:

   ```powershell
   pwsh tools/lab/install.ps1 -DryRun
   pwsh tools/lab/install.ps1                     # or -Worktree <path>
   ```

   The script builds through `tools/build-lane/lane.ps1`; no bash is
   involved. `-InstallDir` names the SGW install when neither
   `CIMMERIA_LAB_INSTALL_DIR` nor the main checkout's `.mcp.json` does.
   `-SkipBuild` installs what the target dir already holds. The script
   finds the outputs in the target dir the lane uses
   (`$CIMMERIA_TARGET_ROOT\<worktree>\` on a Dev Drive, else
   `<worktree>\target\`), keeps each replaced file as
   `<name>.<yyyymmdd>.old`, skips a file whose content has not changed,
   and writes the source commit to `bin\installed-from.txt`.
3. Restart the supervisor. With the
   [shared daemon](#the-shared-daemon-cimmeria-lab---http),
   `lab restart` (or `pwsh tools/lab/daemon.ps1 restart`) copies the new
   `bin\cimmeria-lab.exe` and every session uses it from its next call.
   A stdio supervisor is still the old
   process (the script renamed its exe; it did not stop it). Run
   `/mcp reconnect cimmeria-lab`. If the server is still connected and
   the reconnect keeps the old process, stop that `cimmeria-lab.exe`
   first; the reconnect then spawns the new one. The script lists the
   running supervisors' PIDs. Stop only your own session's: another
   session's supervisor is that session's lab.
4. Verify: `lab doctor` passes its binary check (the daemon runs the exe
   you installed), `lab_uat_run { plan_only: true }` lists the tools the
   new build routes, and `lab_client_start` launches with the new DLL.

To roll back, rename the `.old` copies back over the installed files and
reconnect again.

## The `lab` command

`lab` is one command in front of the lab scripts and the
[shared daemon](#the-shared-daemon-cimmeria-lab---http): its status, its
log, `labd.env`, installs, instances and clients. It runs an installed copy
of the scripts, never a checkout's, so a checkout on an old branch can't
drive the lab (D-LC4 in the
[campaign ledger](../analysis/lab-cli/README.md)). It is PowerShell 7 only.

### Set up the `lab` command

Run setup once, from a checkout on a current `main`:

```powershell
pwsh tools/lab/cli/setup.ps1
```

It does three things:

1. Copies the CLI into `%LOCALAPPDATA%\cimmeria-lab\cli\`: `lab.ps1`, the
   commands in `cli\*.ps1`, and `daemon.ps1`, `instances.ps1` and
   `labd-lib.ps1` beside `lab.ps1`. Tests are not copied, and a command
   file the checkout no longer has is removed. `cli\cli\VERSION`, the
   checkout's short commit, is written last, so a copy that failed part way
   has no `VERSION`.
2. Writes the shim `%LOCALAPPDATA%\cimmeria-lab\bin\lab.cmd`, which runs
   `pwsh -NoProfile -File <that copy>\lab.ps1` with your arguments. The
   shim holds the real path.
3. Asks `Add ...\cimmeria-lab\bin to your user PATH? [y/N]` when the
   folder isn't on it yet. `-Yes` adds it without asking, and `-NoPath`
   leaves the `PATH` alone. The edit goes through `HKCU\Environment`
   directly, so a `REG_EXPAND_SZ` `PATH` keeps its type, and setup
   broadcasts the change. Open a new terminal to see it.

Run `setup` again to refresh the copy. From the installed copy (`lab setup`
on the `PATH`) there is no checkout to copy from, so it needs one:
`lab setup -From C:\src\Cimmeria`. `lab install` refreshes the copy too, from
the worktree it builds.

### Commands

Every command prints its own help in its `.SYNOPSIS`; `lab help` lists the
first line of each.

| Command | What it does | Exit codes |
|---|---|---|
| `lab help` | lists the commands | 0 |
| `lab status` | daemon pid, uptime and version, then one row per instance: account, client pid, bridge port, lease (owner, purpose, time left; never an id) and whether its profile is seeded | 0; 1 when the daemon is down (it still lists the `labd.env` instances with the seed column) |
| `lab version` | the copy's `VERSION` (`dev` when run from a checkout) and the daemon version | 0 |
| `lab start`, `lab stop [-Force]`, `lab restart [-Force]` | the `CimmeriaLabDaemon` task, through `daemon.ps1`; `stop` and `restart` also close the lab clients (see [A new build](#run-it)) | `daemon.ps1`'s: 3 when a leased client refused the stop |
| `lab logs [-Lines 50] [-Instance p2] [-Level warn] [-Follow]` | the last lines of `labd.log`, filtered by instance and minimum level; a continuation line (a panic body, a backtrace) goes with its entry; bearer and 64-hex tokens print as `<redacted>` | 0; 1 when there is no log |
| `lab env [get KEY \| set KEY VALUE \| unset KEY]` | reads or edits `labd.env`; see below | 0; 1 for `get` of an unset key or a missing `labd.env`; 2 for a bad verb, key or value |
| `lab install -From <worktree> [-SkipBuild]` | builds and installs the lab from that worktree, then refreshes the CLI copy with the worktree's own installer | 0; non-zero when the build, the install or the copy fails |
| `lab setup [-From <checkout>] [-NoPath] [-Yes]` | see [Set up the `lab` command](#set-up-the-lab-command) | 0; non-zero when the copy fails |
| `lab instances [status \| init ...]` | `instances.ps1` from the installed copy (see [Parallel clients](#parallel-clients-up-to-five)) | its own |
| `lab clients stop [<instance> \| all] [-Force]` | closes lab clients; see below | 0 stopped or nothing to stop; 1 daemon down; 2 usage; 3 a leased client was refused; 4 a client could not be stopped |
| `lab doctor` | one `PASS`, `WARN` or `FAIL` line per check; see below | 0; 1 when any check fails |
| `lab uat <section> [-Rows ...] [-Leases 1-5] [-RunsPerLease 1-20] [-Instance p2] [-PlanOnly] [-Json] [-Quiet]` | runs a UAT spec through `lab_uat_run` with no agent; see below | 0 every run passed; 1 a run failed; 2 usage; 3 the pre-flight or plan failed (nothing driven); 4 a lane stopped on its brake |

An unknown command exits 2. Examples:

```powershell
lab status
lab logs -Instance p2 -Level warn -Lines 20
lab logs -Follow
lab env get CIMMERIA_LAB_INSTANCES
lab env set CIMMERIA_LAB_INSTANCES default,p2,p3
lab install -From C:\src\Cimmeria\.claude\worktrees\lab-fix
lab instances status
lab clients stop p2
lab doctor
```

**`lab uat`.** It runs spec rows (`docs/guides/uat-specs/`) on the lab with no agent: one `lab_uat_run` call per run, over the daemon's MCP endpoint.

- **Lanes.** `-Leases N` starts N lanes at once, one per hosted instance in the daemon's order (`default`, `p2`, ...), skipping instances another session holds. Each lane runs the rows `-RunsPerLease` times, back to back.
- **Leases.** Every run takes and releases its own lease.
- **Pacing.** Lanes start `-StaggerSeconds` apart (default 20). Run ids are time based, and two clients booting at once is where logins go wrong.
- **Pre-flight.** Before anything is driven: the daemon answers, the instances are free, no `SGW.exe` runs outside the lab, and a plan-only pass of the rows is ready.
- **Pre-flight before the daemon.** Argument errors exit 2 whatever state the daemon is in. A requested row the section doesn't have is a usage error too: row ids are exact and case sensitive.
- **Brake.** A lane stops after `-MaxConsecutiveFailures` failed runs in a row (default 3, 0 never). A lane that ran every run is not counted as stopped early. The other lanes carry on.
- **Timeouts.** Each run times out after `-RunTimeoutMinutes` (default 15). The lane then waits up to 10 minutes for that instance's lease to clear before its next run; a lease that never clears ends the lane.
- **Ctrl+C** starts no new run, and the summary is still written from the runs that finished. A run already under way finishes on the daemon and releases its lease.

Output:

- **Progress:** one line per run.
- **Summary:** the pass rate per row and per lane, and failures grouped by their first failing row. Known issues are tagged (`#1341` for FS-P2's "Corporal Frost is known"), so a new failure stands out.
- **`-Json`** prints only one compact JSON object, for agents and scripts. A clean 7-row, 10-run batch is about 340 characters; failure groups are capped at 8 and their reasons at 200 characters. Failures print one too: `{"ok":false,"exit":N,"error":...}`. `-Quiet` prints one line.
- **Files.** Every batch writes `batch.jsonl` (one line per run, as each finishes), then `batch.json` and `summary.md`, under `uat-runs\batch-<time>\`, next to the runs' own evidence folders (`CIMMERIA_LAB_UAT_DIR` when set). Lease ids are redacted from reasons.

```powershell
lab uat first-session -Rows FS-01,FS-02,FS-P1,FS-P2,FS-P3,FS-P4,FS-P5 -PlanOnly
lab uat first-session -Rows FS-01,FS-02,FS-P1,FS-P2,FS-P3,FS-P4,FS-P5 -Leases 2 -RunsPerLease 5
lab uat first-session -Instance p2 -Rows FS-01,FS-02,FS-P1 -Json
```

The `first-session` SGU rows are not calibrated yet, so name the Praxis rows as above.

**`lab env`.** With no verb it prints every line of `labd.env`. Values print
masked: a key containing `TOKEN`, `SECRET`, `PASSWORD`, `KEY`, `AUTH` or
`CREDENTIAL` shows `<redacted>`, so do bearer and 64-hex tokens, a URL's
user, password and host show as `<host>`, and a line that is neither
`KEY=VALUE` nor a comment shows `<unparsed line>`. The masking is display
only: `set` and `unset` write values back unchanged. Keys are upper case
(`^[A-Z][A-Z0-9_]*$`) and match existing lines ignoring case, as the daemon
reads them. `set` replaces the key's line in place or appends it; `unset`
removes it; comments and order stay. Before writing, `labd.env` is copied
to `labd.env.bak-<yyyyMMdd-HHmmss>`, and the new file replaces the old one
in a single move. The daemon reads `labd.env` only when it starts, so a
change needs `lab restart`. Two value rules:

- a value that starts with `-` must be written `-Value:<value>`, or
  PowerShell reads it as a parameter name: `lab env set SOME_FLAGS -Value:-x`;
- a value with leading or trailing spaces is refused, because `labd.env`
  lines are trimmed when read.

**`lab clients stop`.** It stops only clients whose instance holds no lease,
because a leased client belongs to the agent driving it (D-LC5). A leased one
is refused, naming the holder and purpose. `-Force` stops it anyway, but
only with a named instance: `lab clients stop all -Force` exits 2 before
asking the daemon anything. The holder loses the client, and its watchdog
may relaunch it. The pid comes from the daemon's `/status`, and only a
process named `SGW` is touched: the command asks its window to close and
force-stops it after 8 seconds.

**`lab doctor`.** Read-only; the token is only ever reported as set or not.

| Check | Fails as |
|---|---|
| The scheduled task `CimmeriaLabDaemon` exists | FAIL |
| The daemon answers `/status` | FAIL |
| `CIMMERIA_LAB_DAEMON_TOKEN` is set | FAIL |
| `labd.env` has `CIMMERIA_LAB_INSTALL_DIR`, and `Binaries\SGW.exe` is under it | FAIL |
| The profile root is absolute and outside the install dir | FAIL |
| Each instance's account file exists | WARN (`lab instances init`) |
| Each instance's profile is seeded | WARN |
| No `SGW.exe` runs that `/status` doesn't list | WARN, naming the pids |
| `bin\cimmeria-lab.exe` and the daemon's `labd\cimmeria-lab.exe` have the same SHA-256 | WARN (`lab restart`) |
| The copy's `VERSION` has the same `tools/lab` as `origin/main`, compared by content (`git diff --quiet <VERSION> origin/main -- tools/lab`); only inside a Cimmeria checkout | WARN (`lab setup`) |

The last check compares content, not ancestry: PRs are squash-merged, so
the commit of a worktree you installed from never becomes an ancestor of
`origin/main`, even after it merges. It uses your last fetch.

**Where things live**, all under `%LOCALAPPDATA%\cimmeria-lab\`:

| Path | What it is |
|---|---|
| `bin\lab.cmd` | the shim on your `PATH` |
| `cli\lab.ps1`, `cli\cli\*.ps1` | the installed dispatcher and commands |
| `cli\daemon.ps1`, `cli\instances.ps1`, `cli\labd-lib.ps1` | the scripts the commands run |
| `cli\cli\VERSION` | the commit the copy came from |

The dispatcher is [`tools/lab/lab.ps1`](../../tools/lab/lab.ps1). A command
is a file `tools/lab/cli/<name>.ps1`, so adding one never edits the
dispatcher; `common.ps1`, `test-*.ps1` and `*-lib.ps1` are not commands.
Named flags reach the command as flags, and a command that ends without
`exit` reads as success. The tests are `tools/lab/cli/test-*.ps1`, run with
`pwsh -NoProfile -File`. `CIMMERIA_LAB_HOME` points the CLI at another lab
home for those tests only: `daemon.ps1` ignores it, so with it set,
`lab status` would read a different `labd.pid` than the one `lab restart`
acts on.

## The shared daemon (`cimmeria-lab --http`)

By default every Claude session starts its own stdio `cimmeria-lab`, and
each one runs its own heartbeat watchdog against the same `SGW.exe`. Three
sessions meant three watchdogs: one relaunched a client another session
had just closed, and a new build reached a session only when it
reconnected. The daemon is one `cimmeria-lab` for the whole machine, so
there is one supervisor, one watchdog and one owner of the client. Every
session reaches it over MCP streamable HTTP. Stdio stays the default, so
existing `.mcp.json` entries keep working until you switch.

What the daemon refuses:

| Check | Refusal |
|---|---|
| `--http` is not a loopback address (`127.0.0.1`, `::1`) | exit 2 at start |
| `CIMMERIA_LAB_DAEMON_TOKEN` unset or under 32 bytes | exit 2 at start |
| another daemon holds the `Local\cimmeria-labd` mutex, or the port is taken | exit 3 at start; the log names the holder's pid from `labd.pid` |
| a request without `Authorization: Bearer <token>`, to `/mcp` or `/status` | `401`, before any MCP session |
| a `Host` header other than `localhost`, `127.0.0.1` or `::1` (DNS rebinding) | `403` on `/mcp` |
| any `Origin` header (no browser has a reason to call it) | `403` on `/mcp` |

### Run it

```powershell
pwsh tools/lab/daemon.ps1 install   # copy the exe, make the token, import env, register + start the task
pwsh tools/lab/daemon.ps1 status    # task state, pid, port, token present, last log lines
pwsh tools/lab/daemon.ps1 restart   # pick up a newer build (see below)
pwsh tools/lab/daemon.ps1 stop
pwsh tools/lab/daemon.ps1 start
pwsh tools/lab/daemon.ps1 uninstall
```

`lab start`, `lab stop` and `lab restart` run the same three commands.
`lab status` and `lab doctor` read the daemon's `GET /status`, a
read-only summary behind the same bearer token as `/mcp` (see
[The `lab` command](#the-lab-command)).

`install` registers the per-user scheduled task `CimmeriaLabDaemon`: it
starts at logon, in your interactive session (the client needs the
desktop), and restarts after a crash. Its files are in
`%LOCALAPPDATA%\cimmeria-lab`:

| File | What it is |
|---|---|
| `labd\cimmeria-lab.exe` | the daemon's own copy of the exe, so a rebuild or `tools/lab/install.ps1` is never blocked by the running daemon |
| `labd.env` | the supervisor's environment, `KEY=VALUE` per line, imported once from the `env` block of the stdio `cimmeria-lab` entry in `.mcp.json` (`install -Force` re-imports) |
| `labd.log` | the daemon log; rotates at 10 MiB, five old files kept (`labd.1.log` ...) |
| `labd-task.log` | the task wrapper's start and exit lines |
| `labd.pid` | pid and address of the running daemon |

The token is the user environment variable `CIMMERIA_LAB_DAEMON_TOKEN`,
which `install` generates when it is missing. Claude Code gets it from
`tools/lab/labd-headers.ps1`, the entry's `headersHelper`, which reads the
user environment directly, so the token never lands in `.mcp.json` and a
fresh token works without restarting Claude Code:

```json
"cimmeria-lab": {
  "type": "http",
  "url": "http://127.0.0.1:8779/mcp",
  "headersHelper": "pwsh -NoProfile -File <CIMMERIA_ROOT>\\tools\\lab\\labd-headers.ps1",
  "timeout": 1800000
}
```

**Keep the `timeout`.** For an HTTP server, Claude Code waits for the
first response byte for the larger of 60 seconds and the server's tool
timeout. Without the per-server `timeout` (milliseconds), a `lab_uat_run`
row that takes longer than a minute fails with "The operation timed out."
while the row is still running (#1243). Calls running past two minutes
move to a background task in Claude Code; the limit still applies. Source: Claude Code's MCP documentation, <https://code.claude.com/docs/en/mcp>.

`.mcp.json.example` carries this entry as `cimmeria-lab-http`. To switch,
rename it to `cimmeria-lab` and delete the stdio `cimmeria-lab` entry.

**A new build.** `restart` copies
`%LOCALAPPDATA%\cimmeria-lab\bin\cimmeria-lab.exe` (what
`tools/lab/install.ps1` installs), else the repo's
`target\debug\cimmeria-lab.exe`, over the daemon's copy when it is newer
(`-Exe <path>` names one explicitly), then starts the task. Every session
gets the new build on its next call; a session whose MCP connection broke
reconnects with `/mcp`.

**Clients across a stop.** A new daemon does not adopt the clients the old
one launched. Before F-LC1 they ran on unlisted, with their bridges
unreachable. So `stop`, `restart`, `install` and `uninstall` read the
running daemon's `/status` first, stop the daemon, and then close every
client it listed (window close, force-stop after 8 seconds, only a process
named `SGW`). Leases live in the daemon's memory and end with it. If any
client is leased, the stop is refused before anything is touched: it names
the holder and purpose and exits 3. `-Force` closes the leased clients too.
If the live daemon answers `401` (the token is not its token), its leases
are unknown, so the stop is refused with exit 4; `-Force` stops the daemon
and leaves the clients open. When `/status` can't be read for another reason
(an older daemon), the stop warns and leaves the clients open; `lab doctor`
then lists them as unlisted `SGW.exe`. The clients are closed only once the
daemon that listed them has exited, so its watchdog can't relaunch them.

### The lab lease

One agent drives the lab client at a time, and the supervisor enforces it.
Take the lease before you drive, pass its id to every tool that drives, and
release it when you are done:

```text
lab_lease_acquire {owner: "session-name", purpose: "ability UAT AB-3"}
  -> {lease_id: "lease-...", expires_at, ttl_s: 600}
client_ui_click {window: "Login_LoginButton", lease_id: "lease-..."}
lab_lease_release {lease_id: "lease-..."}
```

| Rule | What happens |
|---|---|
| A tool that drives the client is called without `lease_id`, or with one that is not the current lease | refused: the message says to call `lab_lease_acquire`, names the holder if there is one, and says when and how a stale lease ended (released, expired, or taken over by whom and why) |
| `lab_lease_acquire` while someone holds the lease | refused, naming the holder, their purpose and since when |
| `lab_lease_acquire {force: true, reason}` | takes the lease over; logged at `WARN`; the new lease and `lab_lease_status` record the previous holder |
| any guarded call with the current lease | renews the lease for its `ttl_s` (a touch) |
| the lease is taken over, released or expires while a guarded tool is still running | the tool stops before its next bridge call, key or click press, or process launch, with an error starting `lease revoked`; key and button releases still go through, so nothing is left held |
| no call for `ttl_s` seconds (default 600, 30 to 3600) | the lease expires and is logged; `lab_lease_renew` extends it explicitly |
| the client dies while nobody holds a lease | the watchdog leaves it down and logs `watchdog_idle_no_lease`; with a lease it relaunches and logs back in as before, and stops if the last lease goes during the relaunch |

`lab_lease_status` shows the holder, purpose, since, expiry and the last few
leases with how they ended. It never shows a lease id: the id is what lets
a session drive, so only the acquirer gets it.

**Which tools need the lease.** A tool needs it when it changes the client,
drives its input or UI, runs Lua or native code the caller chooses, or
moves a shared read cursor. The list is `OPEN` / `LEASED` in
`crates/lab/src/lease/policy.rs`; a tool missing from both is guarded, and
a test fails until every routed tool is classified. Guarded tools list
`lease_id` as a required argument in `tools/list`.

| Open (no lease) | Leased |
|---|---|
| `lab_lease_*`, `lab_client_status`, `lab_crash_report`, `lab_timeline`, `lab_uat_report` | `lab_client_start` / `stop` / `restart` |
| `lab_screenshot`, `lab_screenshot_region`, `lab_pixel_probe` | `client_lua_eval`, `client_wait_for` (its predicate is Lua), `client_mem_write`, `client_call_native`, `client_console`, `client_hook_install` / `remove` |
| `client_module_info`, `client_mem_read`, `client_hook_list`, `client_input_status` | `client_events_read`, `client_wait_event`, `client_chat_log`, `client_combat_log` (shared cursors) |
| `client_entity_table`, `client_ui_state`, `client_window_read`, `client_player_state`, `client_hotbar`, `lab_characters` | `client_entity_find` (it pins the shared unit slots and the one projection slot), `client_inventory` (its `snapshot` writes a shared table); every input, click, drag, world, combat and item tool; the `lab_*` login, character, play, dialog and logout flows; the composites `lab_ensure_in_world`, `client_batch` (caller-chosen Lua and native calls) and `client_ui_sequence`; `lab_uat_attest`; `lab_uat_run` (see below) |

The four cursor reads are leased because they share one event store: two
sessions reading through the same named cursor take events from each other,
and the chat and combat logs install their client-side capture on first
use. A session that only watches uses screenshots, the UI readers,
`lab_timeline` and SigNoz.

**`lab_uat_run` and the lease.** Pass your `lease_id` and the run drives
under your lease. Omit it and the run takes a lease of its own (owner
`lab_uat_run`, purpose naming the sections and rows), which is refused
while someone else holds the lab and released when the run ends,
whatever the outcome. A keep-alive renews the lease every third of its
ttl for the whole run, so `wait_ms` steps (which call no tool) never let
it lapse. The moment the lease is taken over or released, the run stops:
the row being driven is cut off at its next await and BLOCKED, every
remaining row is BLOCKED without being driven (reason `lease revoked`),
and the run lets go of every held key and button. `plan_only: true`
drives nothing and needs no lease.

**Stdio mode has the same rules.** A stdio supervisor enforces the lease
too, so a tool behaves the same whichever transport reaches it. With one
stdio supervisor per session the lease only covers that session's own
supervisor, which is why the daemon is the supported setup.

**Telemetry.** Each acquire, renew, release, expire and forced takeover is
one event on target `lab.lease` (field `event`, plus `owner` and
`purpose`) in `labd.log`. The watchdog's refusal to relaunch is
`event = "watchdog_idle_no_lease"` on the same target. The touch on every
guarded call is `debug` only.

### Cut over from stdio supervisors to the daemon

Do this once per machine, at a quiet moment: it ends every session's own
supervisor. Nothing here touches the server or the game install.

1. **Quiesce.** Make sure no session is mid-run: `lab_lease_status` on any
   session (stdio supervisors each have their own book, so ask around), and
   look for `SGW.exe` with the owner query in
   [Before you drive the client](#before-you-drive-the-client-the-lab-lease).
   Have the owning session `lab_client_stop` its client.
2. **Install the build.** `pwsh tools/lab/install.ps1` from the checkout
   you want (it builds and installs to `%LOCALAPPDATA%\cimmeria-lab\bin\`).
   The daemon needs PR 1 to 3 of the shared-daemon work, so install from a
   `main` that has them.
3. **Install the daemon task.** `pwsh tools/lab/daemon.ps1 install`. It
   copies `bin\cimmeria-lab.exe` to `labd\`, generates
   `CIMMERIA_LAB_DAEMON_TOKEN` if it is missing, imports `labd.env` from
   the `env` block of the stdio `cimmeria-lab` entry in the repo's
   `.mcp.json`, registers `CimmeriaLabDaemon` and starts it. Check
   `labd.env`: it is the daemon's whole environment
   (`CIMMERIA_LAB_INSTALL_DIR`, `CIMMERIA_LAB_START32`,
   `CIMMERIA_LAB_PATCHES_DLL`, `CIMMERIA_LAB_SERVER_URL`,
   `CIMMERIA_LAB_MCP_URL` and `CIMMERIA_LAB_MCP_TOKEN`), and paths into a
   target dir should point at `bin\` instead. Edit it, then
   `daemon.ps1 restart`.
4. **Check it runs.** `pwsh tools/lab/daemon.ps1 status`: task `Running`,
   a live pid, the port listening, the token set, and a
   `lab daemon listening` line in the log tail.
5. **Switch `.mcp.json`.** Replace the stdio `cimmeria-lab` entry with the
   http one (`.mcp.json.example` has it as `cimmeria-lab-http`; rename it
   to `cimmeria-lab` so tool names stay `mcp__cimmeria-lab__*`):

   ```json
   "cimmeria-lab": {
     "type": "http",
     "url": "http://127.0.0.1:8779/mcp",
     "headersHelper": "pwsh -NoProfile -File <CIMMERIA_ROOT>\\tools\\lab\\labd-headers.ps1",
     "timeout": 1800000
   }
   ```

   Remove `cimmeria-lab-p2` too unless you drive the second client by
   hand. Two-player UAT rows drive p2 from inside the daemon (under the
   same lease); a stdio p2 supervisor is a second owner of that client,
   with its own watchdog and no view of the daemon's lease.
6. **Stop the stdio supervisors.** In every open session run `/mcp`
   reconnect for `cimmeria-lab` (it now reaches the daemon). Then stop any
   `cimmeria-lab.exe` still running from a target dir or `bin\`; only the
   daemon's copy in `labd\` should be left:

   ```powershell
   Get-Process cimmeria-lab | Select-Object Id, Path
   ```

7. **Verify.** From any session: `lab_lease_status` answers (`held:
   false`); `lab_uat_run {plan_only: true}` lists the routed tools and
   needs no lease; `lab_lease_acquire` from one session is refused from a
   second. Delete a leftover `%LOCALAPPDATA%\cimmeria-lab\live.lock`.

**Roll back:** restore the stdio entry in `.mcp.json`, reconnect, and
`pwsh tools/lab/daemon.ps1 uninstall`.

**New builds after the cut-over:** `pwsh tools/lab/install.ps1`, then
`pwsh tools/lab/daemon.ps1 restart` (the restart picks up the newer
`bin\cimmeria-lab.exe`). Sessions reconnect on their next call, or with
`/mcp`.
