---
title: Live Research Lab — the research rulebook
type: how-to
audience: engineers and AI agents doing RE / live verification against a running SGW.exe + cimmeria-server
last_updated: 2026-09-29
companion_docs:
  - ../architecture/live-research-lab.md
  - ../architecture/client-telemetry.md
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
rows on **one clock**. Client events (bridge heartbeat today; the full
hook-hit / Lua-print / Mercury-dispatch ring once #686 lands) arrive on
the dev-box clock; packet-tap rows arrive on the server clock. The
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

> **Today vs. later.** The client side of the timeline is currently the
> bridge heartbeat ring — enough to answer "did the client's main thread
> keep ticking while the server sent packet X". The rich client-event ring
> (`client_events_read`, #686) was stopped by the owner; when it lands it
> plugs into the same offset/merge machinery with no downstream change.
> Until then, `lab_timeline` is useful but heartbeat-only on the client
> side, and the clock offset is a coarse estimate from the packet-tap
> round trip (there is no dedicated server ping tool yet).

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

Before you build any autologin probe, answer the open question from the
#685 spike — **is the Lua VM alive at the login screen?** — for free, with
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

Supervisor (`cimmeria-lab`, stdio MCP on the dev box):

| Tool | Purpose |
|---|---|
| `lab_client_start` / `_stop` / `_restart` | Own the SGW.exe lifecycle. |
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
| `lab_screenshot` | Window capture by PID → MCP image. |
| `lab_crash_report` | Last minidump, last N commands, quarantined command. |
| `lab_timeline` | Merged client+server window (above). |
| `client_lua_eval` / `client_module_info` / `client_mem_read` | Probe tools proxied to the bridge. |
| `client_ui_click` / `client_cursor_move` | Click a named UI window (`Name` or `Parent/Child`) like a player: cursor onto its centre, real button messages. |
| `client_input_key` / `client_type_text` | Key presses and typing as `WM_KEYDOWN`/`WM_KEYUP` (the game translates them; Shift is virtual). Types letters, digits, space, `-_/.`, so `/logout` and `.`-console lines work through chat. |
| `client_input_mouse` | DirectInput relative motion (mouse-look) and button clicks at the UI cursor. |
| `client_input_focus` / `client_input_status` / `client_input_release` | Virtual focus (the game keeps reading input in the background), hook counters, let go of everything. |
| `client_window_read` | Read an open window: title, buttons with enabled state, text, list rows. `kind` adds the module's state for vault, trainer, crafting, pet, organization, mail, loot, DHD, dialog/blurb, greet, vendor, trade, character. See [UI readers and item tools](#ui-readers-and-item-tools). |
| `client_window_click` | Click a named widget, the widget showing a text, or a list row, with a real click at its screen point. |
| `client_chat_log` | Chat, feedback and Server Message lines with channel, speaker, colour and tabs, from the lab's chat ring through the event store, read through a named cursor. |
| `client_inventory` | Every loaded container, the bandolier's ammo and cash; `snapshot` / `diff_against` for before-and-after checks. |
| `client_player_state` | Position, world, level, experience, every stat, effects, the active weapon's ammo, the target. |
| `client_item_action` | Use, equip, unequip (a right-click on the slot), double-click, Loot All, loot one row; slash-command fallback. |
| `client_drag_drop` | A real drag between slots or onto a named window; `split` is the stock Ctrl-drag (one off the stack). Verified by an inventory diff. |
| `client_entity_find` | Entities the client knows, by id, name, mob id (the client's template id), hostility or distance: name, level, hostility, rendered, targetable, position (client and server coordinates), distance, screen point. Read-only. See [World tools](#world-tools). |
| `client_world_click` / `client_target` | Click an entity or world point in the 3D view with real input (camera turned onto it if needed, mouse-over checked for occluders), then report the target and windows it changed. `client_target` left-clicks and requires `Unit.Target` to become the entity. |
| `client_move_to` | Walk to a point, an entity or through waypoints with `W` and mouse-look, closed loop on the player's position; stuck detection, arrival radius, timeout. |
| `client_camera` | Mouse-look yaw/pitch, wheel zoom, face an entity or point. |

Server endpoint (`cimmeria-lab-mcp`, in-server, token-gated HTTP —
WireGuard-only on the colo): `server_console_*`, `server_sessions`,
`server_entity_*`, `server_witnesses`, `server_packet_tap_*`,
`server_log_tail`, `server_content_reload`, `server_db_query`. See ADR
§3.5 for the full set; `docs/operations/colo-deploy.md` for the port.

## Driving the client with its own input

The input tools press nothing through Lua: Lua only reads where a widget is and places the UI cursor. What the live client showed (2026-09-29):

- Keys and typing are window messages. A posted `WM_KEYDOWN`/`WM_KEYUP` reaches the game; a bare posted `WM_CHAR` is ignored, because the game turns keys into characters itself with `GetKeyboardState` + `ToUnicodeEx`. The bridge makes Shift virtual by answering those two calls.
- Mouse buttons are window messages, applied at CEGUI's cursor position, not at the message's coordinates. The cursor does not follow posted mouse moves or DirectInput motion, so the supervisor places it through CEGUI's own cursor and mirrors it into a virtual `GetCursorPos`.
- The DirectInput keyboard is created but never read. The mouse is read while the viewport has it captured (mouse-look), and only while the game thinks it is focused: virtual focus answers `GetForegroundWindow`, `GetFocus`, `GetActiveWindow`, and lets a background `Acquire` succeed.
- Launch skips the intro movies with Escape; on a new character Escape also skips the arrival cutscene, and dialogs are paged with Next to the green checkmark (`Dialog_DoneButton`).
- `lab_client_start` refuses while an `SGW.exe` the lab does not own is running, and while its own instance's client runs. A second lab client is allowed only as a named instance ([Two clients](#two-clients-two-player-scenarios)). It injects `cimmeria-client-patches.dll` first when `CIMMERIA_LAB_PATCHES_DLL` is set, as the launcher does.

## Two clients: two-player scenarios

Trade, duels, squads and teams, mail between players, player-to-player visibility and chat need two players. Two clients on one workstation work when each has its own **lab instance**; without that they collide on the session file, the bridge port, the credentials, the crash markers and the logs (evidence and the client-side findings: [multi-client-lab.md](../reverse-engineering/findings/multi-client-lab.md)).

**Setup.** Run one `cimmeria-lab` per client, each its own MCP server entry with its own environment. The default entry is the first player; the second adds `CIMMERIA_LAB_INSTANCE` and its own port:

```json
"cimmeria-lab-p2": {
  "type": "stdio",
  "command": "<CIMMERIA_ROOT>\\target\\debug\\cimmeria-lab.exe",
  "env": {
    "CIMMERIA_LAB_INSTANCE": "p2",
    "CIMMERIA_LAB_BRIDGE_PORT": "8771",
    "CIMMERIA_LAB_INSTALL_DIR": "<SGW_INSTALL_DIR>",
    "CIMMERIA_LAB_START32": "...",
    "CIMMERIA_LAB_PATCHES_DLL": "..."
  }
}
```

Its tools show up under the second server's name, so an agent addresses a player by the tool prefix. A named instance gets:

| | Default instance | `CIMMERIA_LAB_INSTANCE=p2` |
|---|---|---|
| Session file | `sessions\current-session.json` | `sessions\instances\p2\current-session.json` (the DLL finds it through `CIMMERIA_LAB_SESSION_FILE`) |
| Bridge port | 8770 | `CIMMERIA_LAB_BRIDGE_PORT` (give each instance its own; `CIMMERIA_LAB_BRIDGE` follows it by default) |
| Credentials | `sessions\lab-account.json` | `sessions\lab-account.p2.json`, never the default file |
| Crash marker, minidumps | `sessions\` | `sessions\instances\p2\` |
| DLL logs | `cimmeria-client-*.log` | `cimmeria-client-*-p2.log` |

`CIMMERIA_LAB_MAX_CLIENTS` caps the clients (default 2, ceiling 4). The start guard still refuses when an `SGW.exe` the lab does not own is running.

**Two accounts.** A second login on the same account evicts the first client (`duplicate_login`), so the second instance needs its own account and `lab-account.p2.json`. The seed has `lab` plus `lab2` to `lab5` (account ids 10 to 14, same password as the other seed accounts); give each instance its own account in its `lab-account.<instance>.json`, and keep all five in the Discord `muted_accounts` list.

**Focus.** A client whose window is not in the foreground runs at below-normal priority with a 5 ms sleep per tick (`FEngineLoop::Tick`). Turn on `client_input_focus` (virtual focus) for both instances: it answers `GetForegroundWindow` per process, so neither is throttled and each keeps reading its own lab input. Real keyboard and mouse still go to the window in front, so do not type while a scenario runs. Both windows open at the same place and size; screenshots use `PrintWindow` per window, so an overlapped window still captures.

**When to use `wireclient` instead.** A second player that only has to exist and answer (a duel partner, a body to be visible, a trade or squad counterpart driven with `cell_method`/`base_method`) needs no window at all: use `sparbot` or `GameSession` from `crates/wireclient` ([wireclient.md](../architecture/wireclient.md)). It has no throttling and no shared client cache, and needs its own account too. Use a second full client when the second player's UI is part of what is being tested.

## The display: screensaver and D3D

Lab input goes through the client's hooked DirectInput, which never resets Windows' idle timer, so an unattended run reaches the screensaver after about ten minutes. While a screensaver owns the display, Direct3D 9 reports no adapter (`D3DERR_NOTAVAILABLE` from `GetDeviceCaps`): a client launched then dies on a "GetDeviceCaps failed" message box and an R6025 box, each with a Windows error sound, and the watchdog relaunches it until its cap. The supervisor prevents this:

- While any `SGW.exe` runs, a supervisor thread holds `ES_DISPLAY_REQUIRED` (released when none runs), so the screensaver doesn't start.
- Before every launch (including watchdog relaunches) it probes Direct3D. If the display is unavailable and a screensaver that is not password-protected is running, it dismisses it and probes again; otherwise the launch is refused with the reason (screensaver still running, password-protected screensaver, or display off/locked) instead of starting a client that can only show an error box.

## Client flows

To run whole unified-UAT rows (steps, checks, evidence and ledger text) rather than single flows, use `lab_uat_run`: [automated-uat.md](automated-uat.md).

The `lab_*` flow tools turn the scripts agents kept rewriting (log in, make a fresh character, play it, click through the intro dialog, log out) into single calls. Each is supervisor-side orchestration over the input tools above: every button press is a real click or key, and Lua only reads (visibility, widget text, the character list). The one Lua-driven step is picking a server row by name, because list rows are not named windows; the Select button is still clicked.

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
- **Escape opening the game menu.** `lab_play_character` presses Escape only while a movie plays or while the map loads under a cutscene (character select gone, no HUD yet), plus three presses once a new character's intro dialog is up (the arrival cutscene keeps playing under it). With `skip_cutscene: false` it never presses Escape.
- **Closing a dialog the wrong way.** `lab_finish_dialog` pages with Next until Done (the green checkmark) shows and never uses the close X, which sends choice -1. `accept: true` presses Accept on an offer with no Done.

Typing covers letters, digits, space and `-_/.`; a password with other characters is refused before anything is typed. Names in character creation must be letters only.

`client_entity_table` reads the `GameEntityManager` singleton (VA `0x01EF244C` plus the ASLR slide) and walks its three `std::map`s with one memory read per tree node and one per entity. Hundreds of small reads once starved the watchdog's heartbeat and got a healthy client killed; the watchdog now forgives a missed heartbeat while other bridge calls are completing (`heartbeat::BUSY_GRACE_MS`), and a crash relaunch logs back in with `lab_login` and plays the `lab-account.json` character.

Not yet proven on the live client (the prototype scripts these port were): the EULA path, the server-row selection by name, the `SelfStatusWin`/`MinimapWin` world-HUD test for a returning character (the prototype only played new characters, whose intro dialog marks the world as loaded), `client_ui_state`'s root-window and chat sections, and `isReady()` through the vtable.

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
- **Facing** turns until the target projects within a quarter of the half-width of the centre. While it is behind the camera, it turns by the bearing from the camera actor (see below), or a quarter turn when that is unknown.

### Not yet verified on the live client

These tools were written and tested against a simulated client only (the lab lock was not available); the Ghidra facts above are static. The first live run should check, in this order:

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
2. Not on the hotbar, `place: true`: puts it on the first visible empty button with the calls the drop handler makes (`getUnusedAction`, `ActionProfileMod.setButtonCurrentAction`, `setActionToAbility`), reported as N3 because a CEGUI drag cannot be started from posted mouse moves yet, then presses it (N1). The placement stays in the player's profile.
3. Not on the hotbar (default `fallback: window`): opens the Ability window with its bound key (N3 `AbilityMod.onToggleAbilityWin` when unbound), selects the tree tab, clicks `Ability_Button<i>` (N1), and closes the window again. An ability outside the trees (a GM `.giveability` grant) has no window button: use `place: true` or `fallback: lua`.
4. `fallback: lua`: `useAbility(id, Unit.Target)`, N3.

No slash command for abilities is known, so there is no N2 path. The result is read from the events that follow the press, for up to `observe_ms` (default 2500): `net.out useAbility*` (sent), `onTimerUpdate` (cooldown or warmup timer), `onSequence` (cast started), `onEffectResults` or a combat-text record (effect applied), `onErrorCode` (refused), and feedback chat lines. `result.verdict` is one of `effect_applied`, `refused`, `cast_started`, `refused_with_feedback`, `sent_no_reply`, `refused_client_side`, `nothing_observed`. CME events carry names, not payloads, so a timer or effect from another source inside the window counts too; the combat records name the ability. The hotbar button's cooldown after the press is included.

**`client_combat_log`** wraps `SCTMod.onUnitCombat`, the stock handler for `Events.UnitCombat`. The wrapper records the raw event (ability id and name, `HitType`, source and target unit names and whether each is the player, mortal, every stat change with value and result code) into a ring in `_G.CimmeriaLab`, then calls the original, so the SCT verbosity option cannot hide anything and the player sees no change. It re-subscribes `SCTWin`'s `Events.UnitCombat` to the same stock name after wrapping, in case the event system caches the resolved function; it never touches `SCTWin`'s `Events.PreRender` subscription, which the world tools own (a window holds one subscription per event). A second wrapper on `ChatMod.onMessageReceived` feeds `chat.line` events the same way. Capture starts at the first pump in the world (any `client_combat_log`, `client_wait_event` or `client_use_ability` call); an interface reload is noticed (the ring's epoch changes) and the wrappers are reinstalled.

**`client_die_and_respawn`** is for the defeat and respawn rows. Getting killed is setup: `setup_health: n` types `/gmsethealth n 0` (G). That GM command only writes the stat; it does not run the death sequence, so the flow waits for the defeat window (`PlayerDefeatWin`, opened by the server's `onBeginAidWait`), which only a real lethal hit opens. It then reads the respawners and countdown, clicks `PlayerDefeat_Release` (N1; `respawn: auto` lets the countdown release, `none` stops at the window; picking a non-default `respawner` selects its list row through the list's own call, N3), and verifies the window closed, health above zero, and the position and world before, at death and after.

**Events and cursors.** The bridge ring behind `events_read` is drain-and-clear, so two readers would steal from each other. The supervisor is its only drainer: each pull lands in a bounded store (8192 events) with a seq that never repeats in one lab process. Readers keep cursors instead of consuming:

- `client_wait_event` starts after `since_seq`, else its named cursor (default `wait`), else the newest event (a fresh cursor never matches stale history). A met wait moves the cursor to its last match, so the next wait sees only later events, including ones that arrived between the two calls. A timeout leaves the cursor, so a later wait with another predicate still sees what this one scanned past. The safe pattern is arm, act, wait: `client_wait_event {arm: true}`, press or click, then `client_wait_event {name: "*onEffectResults"}`.
- `client_combat_log` has its own cursor (`combat_log`) and returns `next_since_seq`.
- `client_events_read` keeps a cursor too (`events_read`), so it still returns each event once, now with its seq.
- A reader whose cursor fell behind the oldest kept event gets `gap: true`. Events the bridge ring dropped while full are counted as `dropped`, and `cme.event` names past the throttle (8 burst, then 4 per second) carry a `suppressed` count.

Event kinds: `cme.event` (field `event`, e.g. `Event_NetIn_onEffectResults`; `kind` `net_in`, `action`, ...), `net.out` (`method`, `entity_id`), `entity.*`, `cegui.log` (`message`), `lua.error`, `lua.print`, `hook.hit` from the bridge; `combat.hit` and `chat.line` (`text`, `channel`, `channel_name`, `speaker`) from the Lua rings. Predicates are case-insensitive globs: `name` matches `event`, `method`, `name`, `ability_name` or `channel_name`; `text` is a substring (or a glob with `*`/`?`) of `text`, `message` or `line`; `fields` compares field by field.

### Not yet verified on the live client (combat tools)

These tools were written against the stock UI Lua and tested against a fake bridge only (the owner deferred live testing). The first live run should check, in this order:

1. `client_combat_log` once in the world: `capture.status` is `installed`, then `ok` on the next call. Fire one shot: a `combat.hit` record arrives and the floating combat text still shows. No record means the event system kept the old handler; `installed_no_resubscribe` means the re-subscribe call failed.
2. `chat.line`: an ability refusal (no target, out of range) arrives with an empty `speaker`, and a player's `/say` has one. `client_use_ability` treats speaker-less lines as feedback.
3. `client_hotbar`: `getBindingKey` returns `key` as a virtual-key code (49 for `1`), and whether it carries modifier fields.
4. `client_use_ability` on a bar ability with `press: key` and then `press: click`: both send `net.out useAbility`. Check the CME names it waits for appear as `Event_NetIn_onSequence`, `onEffectResults`, `onTimerUpdate` and `onErrorCode`, and that the button's cooldown reads back.
5. The Ability-window path: `getBindingKey('ToggleAbility', 1)` resolves (else the window opens through the N3 toggle), the tab click switches `AbilityMod.currentTab`, and the `Ability_Button<i>` click casts. Then `place: true`: the ability lands on the first empty visible button and stays there after a relog.
6. `client_die_and_respawn`: `/gmsethealth 1 0` leaves the player alive (it does not kill), a lethal hit opens `PlayerDefeatWin` with the respawner list, Release respawns, and `respawn: auto` releases on the countdown.
7. After an interface reload (anything that rebuilds the UI Lua state), the next pump reports `lua_epoch_changed` and combat capture resumes.

## UI readers and item tools

These tools serve automated UAT: read what the client shows, and act on items the way a player does. Readers use the stock UI's own Lua bindings (`getItemIDForSlot`, `getUnitStat`, `getEffectInfo`, `getLootInfo`, the CEGUI window tree); actions put the UI cursor on a widget's screen rectangle and send real button and key messages through the input path above. Container, stat and channel ids are read from the client's `Container`, `Stat` and `UIChannel` tables at run time, never hard-coded.

**Native level.** Every result carries `native_level`, `native_tier` (the UAT matrix labels the world and combat tools use) and a `native_steps` list. The levels, most native first: `real_input` (N1, key and mouse messages), `slash_command` (N2, a line typed into chat, which the client parses and sends itself), `client_ui_lua` (N3, a call into the stock UI's Lua). The overall level is the least native step; `native_pass` is true only when every step was real input, so a UAT runner can refuse to count a pass that fell back. Readers report `native_tier: "read"` and `mode: "read"`. A failure is an MCP error naming the tool, the widget or step, and the elapsed time.

| Tool | How it drives or reads | Fallback (reported) |
|---|---|---|
| `client_window_read` | Walks the window's subtree: name, type, visibility, enabled state, text, screen rectangle, list rows (Listbox and MultiColumnList, with selection). `kind` adds the module's state: loot items (`getLootInfo`), trainer abilities with id, cost and trainable (`TrainerMod`), mail headers, vendor stock (`getVendorItemInfo`), greet topics, crafting permission per craft type, vault items, pet, DHD active. | — |
| `client_window_click` | Real click at a named widget, at the widget showing a text, or at a list row (the row's point comes from the list's `getItemAtPoint`, else from item heights), then reads the row's selection back. | Row not selected after the click: `setItemSelectState` (`client_ui_lua`). Scrolling a row into view is `ensureItemIsVisible` (`client_ui_lua`). |
| `client_chat_log` | Pumps the lab's one chat capture, the `chat.line` ring (see [Abilities, combat and event waits](#abilities-combat-and-event-waits)), into the event store and reads it through its own cursor (`chat_log:<name>`), so it never takes lines from `client_wait_event`. Adds the channel name (`UIChannel`), speaker-flag names, and the colour and tabs the chat window uses for the channel. | — |
| `client_inventory` | Every container the client has loaded, the bandolier's ammo per slot, cash. `snapshot` / `diff_against` give slot changes, per-item quantity deltas and the cash delta. | — |
| `client_player_state` | Position, facing (`unitOrientation` is a 0..1 turn fraction; `heading_deg` too), world, level, experience, every stat, effects, the active weapon's ammo, the target. | — |
| `client_item_action` | `use` / `equip` / `unequip` / `rightclick`: a right-click on the item's slot, which the stock UI turns into `contextSensitiveUseItem`. The inventory or character window is opened with its bound key (`getBindingKey`) and the right tab and the All filter are clicked first. `loot_all` clicks Loot All; `loot_slot` pages the loot window and double-clicks the row. | Window toggle unbound: the window's toggle handler (`client_ui_lua`). Slot beyond the 40 visible: the scrollbar (`client_ui_lua`). Slot cannot be put on screen: `/useitem`, `/equip`, `/lootitem` (`slash_command`). |
| `client_drag_drop` | Button down on the source slot, the cursor walked to the target in steps (CEGUI cursor placement plus a posted `WM_MOUSEMOVE` with the button held), button up. `split` holds Ctrl: the stock inventory pulls one item off the stack on a Ctrl-drag; its Shift-drag split is an unimplemented `TODO`. Verified by an inventory diff: `drag_started`, `moved`, `snap_back`. | Posted motion does not start a drag: the motion is replayed through CEGUI's `injectMousePosition` (`client_ui_lua`). |

Quirks to keep in mind:

- The chat log only has lines shown after the chat ring was first installed (by any pump); read `client_ui_state`'s chat tail for older ones. Centre-screen splash text does not go through the chat handler and is not in the log.
- Items in the Mission and Crafting tabs share `InventoryWin` with Main: a drag between two tabs of the same window is refused, because both ends cannot be on screen at once.
- Vault slots can be read and dragged only while the vault window is open at a banker.

**Not yet proven on the live client.** These tools were built and unit-tested against fixtures and a Lua 5.1 mock of the bindings (every reader chunk loads and runs under Lua 5.1), without a live client. The first live session should check:

1. `client_chat_log`: a typed `/say` line appears once, with `channel: "Say"`, the chat window's colour and tab; a `/tell` error arrives as a `Feedback` line; the chat window still shows every line; a second `client_chat_log` with the same cursor returns nothing new, while `client_wait_event {kind: "chat.line"}` still sees the line.
2. `client_window_read {kind: "loot"}` and `{kind: "trainer"}` at the debug hub's crate and trainer: items and abilities match the windows.
3. `client_window_click` on a `Trainer_Choices` row: `method: "item_at_point"` and `selected: true` with no `fallback`.
4. `client_item_action {action: "use"}` on a consumable: the inventory window opens with its bound key (`getBindingKey('ToggleInventory', 1)` returns a key), the right-click consumes one (diff `delta: -1`), `native_level: "real_input"`.
5. `client_item_action {action: "equip"}` and `unequip`: the item moves between `Main` and its equipment container.
6. `client_drag_drop` between two `Main` slots: `drag_started` true without `motion_injected` (if the drag only starts after injection, posted motion does not reach CEGUI); with `split: true`, one item moves (Ctrl reaches CEGUI's button state as 9).
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
resumes it (#985). Build it with
`cargo build -p cimmeria-start32 --target i686-pc-windows-msvc`.
