# Lab automation: client instrumentation gap report and backlog

> **Date**: 2026-09-28
> **Audience**: whoever drives or extends the Live Research Lab, and the sessions working on `cimmeria-client-telemetry`, `cimmeria-lab` and `cimmeria-lab-mcp`
> **Type**: analysis (gap report plus prioritized backlog)
> **Status**: items C1 and C2 landed with this document. C3 (Lua errors), C6 (outbound methods) and the entity part of C7 (create, enter, leave, destroy, appearance, per-entity CME ids, inbound message paths) plus a CME event catalog landed 2026-09-28 ([client-telemetry.md](../../architecture/client-telemetry.md#entity-lifecycle-inbound-and-outbound-messages-lua-errors-and-the-cme-catalog)); the rest are proposals
> **Companion docs**: [live-research-lab.md](../../architecture/live-research-lab.md) (ADR), [client-telemetry.md](../../architecture/client-telemetry.md), [client-instrumentation-hookpoints.md](../../reverse-engineering/findings/client-instrumentation-hookpoints.md), [cme-event-signal.md](../../reverse-engineering/findings/cme-event-signal.md), [instrumentation-discipline.md](../../architecture/instrumentation-discipline.md)

## Goal

An agent should be able to drive the real client through login, character creation, world entry and the Castle Cellblock tutorial, and tell from telemetry alone whether each step happened and, when one did not, which side dropped it. The server side is well instrumented. The client, `SGW.exe`, logs almost nothing on its own, so the missing half is the client's view: what it received and accepted, what its UI showed, what it sent back, and what failed quietly.

This document lists what is missing, in priority order. Each item names the hook technique, the anchor and how sure we are of it, the event it would emit, and whether it is safe in the telemetry DLL every tester runs or belongs behind the `lab-bridge` feature.

## What changed with this document

| ID | Item | Where |
|---|---|---|
| C1 | `client.cme.event`: an inline hook on the CME event-registry create (`0x00a5c0f0`) names every event the client creates by name. The load-bearing caller is `Client_NetIn_EntityMethodDispatch`, so every inbound entity method the client routed appears as `kind = net_in`. Input actions (`Event_Action_*`) and connection events come through the same hook. Throttled per name. | `crates/client-telemetry/src/hooks/inline_hooks/cme_event_factory.rs` |
| C2 | `client.ui.cegui_log` now carries the line text and a level name, at `error` / `warn` / `debug`. CEGUI exceptions, including the tolua glue's `ScriptException`, log through this logger. | `crates/client-telemetry/src/hooks/cegui_log.rs`, `hooks/vtable_hooks.rs` |
| — | The CME subscriber install was **removed**. `0x00a5c150` is `count(name)` on the event registry, not a subscribe, and `0x00a5c0f0` takes a `std::string`, not the C string the install passed it. The two events it was meant to produce (`onClientMapLoad`, `onClientReady`) now appear through C1. | [cme-event-signal.md § Correction](../../reverse-engineering/findings/cme-event-signal.md#correction-2026-09-28-the-registry-is-an-event-factory-not-a-subscriber-api) |
| — | Both new events are also pushed to the lab bridge's local ring (`cme.event`, `cegui.log`) when the DLL is built with `lab-bridge`. | `crates/client-telemetry/src/bridge/events.rs` |
| — | The two inline-hook primitive tests that patch the same function are serialized. Under the parallel test runner they failed or faulted about one run in six. | `crates/client-telemetry/src/hooks/primitives/tests.rs` |

None of this has run inside the live client yet: no client was launched for this work. The anchors, string layouts and calling conventions were checked against the QA `SGW.exe` by disassembly and headless Ghidra, and every hooked address is covered by the fingerprint gate, so a build that differs installs nothing.

## What changed on 2026-09-29 (telemetry blind spots)

A colo playtest could only be diagnosed by asking the player for local files. These close the client-side part of that gap. None of it has run in a live client yet; every new anchor was checked against the QA `SGW.exe` and is in the fingerprint gate.

| ID | Item | Where |
|---|---|---|
| C3 fix | `client.lua.error` rows had `message: null, status: -1, status_name: unknown`. `lua51.dll` is C++-compiled and its protected-call runner's `catch (...)` returns -1 for an exception Lua did not raise (a C++ throw from a CEGUI or tolua binding, or a structured exception); that path writes no error value. -1 is now `foreign_exception`, every row carries `value_type`, and a -1 row names the called function (`function_source`, `function_line`) when the loaded `lua51.dll` is the QA build. | `crates/client-telemetry/src/hooks/iat_hooks/lua_error.rs`, `hooks/lua_stack.rs` |
| C10 | Built for all players (see the table below). | `crates/client-telemetry/src/hooks/inline_hooks/lua_debug_log.rs` |
| C15 | `client.sequence.dropped`: the SequenceManager's silent drop paths (no Source entity, no pawn, no cooked data, expired, culled by distance, instance refused) with `sequence_id`, `entity_id`, `path`. The cache-ready handler (`0x00d06f30`) and the play step (`0x00d06dd0`) are hooked; the `onSequence` handler (`0x00d05790`) is a proposal only, because its drop branch holds the ids only inside an unverified CME property tree. | `crates/client-telemetry/src/hooks/inline_hooks/sequence_manager.rs`, [client-telemetry.md](../../architecture/client-telemetry.md) |
| — | Launcher: `client.launcher.install_result` (one row per install run, every patch's outcome and a patchset hash mismatch's path and hashes) and `client.patches.counts` (the client-patches DLL's claimed, delivered and dropped counts with the last reason). | [dev-session-telemetry.md](../../architecture/dev-session-telemetry.md) |
| — | Server: player vitals samples, vendor interactions and refusals, numeric `player_id` and item ids on loot and grant rows. | [observability-target-catalog.md](../../architecture/observability-target-catalog.md) |

## What exists today

| Surface | What it can see | The gap for automation |
|---|---|---|
| Telemetry DLL (every tester) | Engine tick, level streaming, package loads, state-flag dispatcher, anim notifies, console command, Bink ticks, the entity-method drop oracle, Lua call counts, thread and module timeline, CEGUI log lines, and since this change every CME event created by name. | No payloads: a dialog opened, but which dialog? Nothing on outbound client methods, UI windows, buttons, Lua errors, or mission-tracker updates. |
| Lab bridge (`lab-bridge` feature) | Lua eval with captured returns and `print`, memory read and write, native calls, dynamic logging hooks, a local event ring, slash commands. | The event ring is drain-and-clear with no sequence numbers; there is no "wait until" primitive and no canned state readers. |
| Lab supervisor (`cimmeria-lab`) | Launch, inject, stop, restart, status, autologin through the screens' Lua handlers, screenshots, crash report, a timeline merge of the heartbeat ring with packet-tap rows. | Autologin does not create characters; "entered world" means the login windows closed, with no server confirmation; `lab_timeline` merges only heartbeats. |
| Server endpoint (`cimmeria-lab-mcp`) | Console exec, sessions, log tail, content reload, read-only SQL, live entity snapshots, witness lists, packet taps. | No mission, inventory or player-state tools; no cursor-based event feed; no wait primitive. |

## The Cellblock tutorial: what proves each step

The tutorial's step and chain ids are in [mission-chains.md](../../content/mission-chains.md) and `db/resources/Content/Seed/castle_cellblock_chains.sql`; the tester checklist is [castle-cellblock-rebuild/uat-guide.md](../castle-cellblock-rebuild/uat-guide.md). For each kind of step, the table names the server evidence that exists today and the client evidence that would close the loop.

| Step kind (example) | Server evidence today | Client evidence: today, then proposed |
|---|---|---|
| Mission accepted on load (622 Arm Yourself) | `mission.accept` span, `sgw_mission` row | Today: `client.cme.event` for the mission-update NetIn event. Proposed: C5 (event payload) for the mission id. |
| Search a body (dialog open 3995 or 3996) | `dialog.event_open`, `dialog.display` | Today: `Event_NetIn_onDialogDisplay` (or the matching event) via C1. Proposed: C4 (window shown) and C6 (the choice sent). |
| Equip an item (step 80622, item 55) | `inventory.move_item`, `item_equipped` chain | Proposed: C6 (the client's move request), C5 (the inventory update it applied). |
| Dialog choice (2300 or 5021 for Prisoner 329) | `dialog.event_choice` | Proposed: C4 (button clicked) and C6 (the choice method sent). |
| Interact with a tagged object, then Livewire | `interact_tag` chain, minigame session | Proposed: C4 (minigame window shown), C6 (the use request). |
| Enter a region (Region2, Region8, Region11) | `enter_region` chain, `player.journal` `region_hint` | Today: nothing on the client. Proposed: C6 would show the region-entered method if the client sends one; position via C9. |
| Kill a tagged NPC (`MessHall_Guard1`) | `entity_dead_tag`, `player.journal` `kill` | Today: state-flag dispatcher (`BSF_Dead` path) is counted but not attributed. Proposed: C7 (entity property applied, with entity id). |
| Ring transport to Castle (688) | `cross_world_teleport`, `world_entry.*` spans | Today: level streaming, package loads, `Event_NetIn_onClientMapLoad` via C1. Proposed: C8 (load-screen phases). |

## Backlog: client-side seams

Ordered by value for automated play-testing against cost. "All players" means the event is safe and cheap enough for the base telemetry DLL; "lab" means it belongs behind `lab-bridge`, because it is expensive, invasive or only useful with an agent attached.

| ID | Seam | Why it helps an agent | Where to hook | Event and fields | Scope | Effort | Risk |
|---|---|---|---|---|---|---|---|
| C3 | **Lua errors from `lua_pcall`** | UI scripts fail through `pcall` and the game shows nothing. The error string is the single most useful client-side debug fact. | The existing `lua_pcall` IAT detour (`hooks/iat_hooks/mod.rs`, owned by the input-injection session): when the original returns non-zero, read the error at stack index -1 with `lua_type` and `lua_tolstring`, resolved by name from `lua51.dll` (the bridge's `lua_capture.rs` already does this). | `client.lua.error` `{status, message, nargs}`, `warn`, throttled per message | All players | S | Low. It reads the stack after the call returns and does not change it. Use `lua_tolstring` only when `lua_type` is a string; it converts numbers in place. |
| C6 | **Outbound client methods** | Every button that does anything sends a method. Seeing it proves the client acted, and pairs it with the server's inbound log. | `RouteOutgoingEntityRpc` at `0x00c6fc40`, the single exit for outgoing entity methods ([entity-property-sync draft §1.5](../../drafts/spec/entity-property-sync.md)). Prologue is `push -1; push 0x016f50ea; mov eax, fs:[0]` (checked 2026-09-28). The argument list is not decoded yet: a headless decompile was blocked by a Ghidra project lock. | `client.net.out` `{method_id, route: cell or base}`, `info` | All players | M | Medium until the signature is decoded. It runs on the main thread for every outgoing call, including movement-adjacent methods, so it needs the per-name throttle. |
| C4 | **CEGUI window shown, hidden, clicked** | "Did the dialog window open, and did the agent's click land" is the question every UI step asks. | `CEGUI::EventSet::fireEvent` (or `Window::onShown` / `onHidden` / `PushButton::onClicked`), found through the wide event-name strings (`Clicked`, `Shown`, `Hidden`). An allowlist of event names keeps it cheap; the window name is a `std::wstring` (same reader as C2). | `client.ui.window_event` `{window, event}`, `info` for `Clicked`, `debug` otherwise | All players, with the allowlist | M | Medium: `fireEvent` is hot during layout loads. Needs the anchor resolved and fingerprinted. |
| C5 | **Payloads of accepted NetIn events** | C1 names the event; for assertions the agent needs the dialog id, the mission and step id, the item. | Read the fields the dispatcher writes into the new event object after `0x00a5c0f0` returns (the per-argument descriptors at `desc+0x24`, `vtable+0x10`), or hook `CmeEventData_GetField` (`0x005783b0`) on the handler side. Needs RE of the `GenericEvent` field store. | `client.cme.event` gains `args` for an allowlist of events (dialog, mission, inventory, error code) | All players for the allowlist | L | Medium: typed field decoding per event. |
| C7 | **Entity create, destroy and property applied** | Answers the invisible-guard class of bug (#838) from the client side: did the client create the entity, and did it apply the appearance and position? | `Event_Entity_Destroyed` / `Event_Entity_PawnGiven` through C1 are free once confirmed. Creation and property application need the `EntityManager` enter-AoI path and the property parse path ([entity-property-sync.md](../../reverse-engineering/findings/entity-property-sync.md)). | `client.entity.created` / `.destroyed` / `.property` `{entity_id, type_id, prop_id}` | Lab for `.property` (volume), all players for create and destroy | M to L | Medium. |
| C8 | **World and map load phases** | World entry can hang in eight phases; the client side of each is partly visible now (streaming, package loads, `onClientMapLoad` via C1). The loading screen's show and hide are missing. | Loading-screen show and hide through C4, or the UE3 `FFullScreenMovie` start and stop around the Bink tick hook. | `client.load.phase` `{phase}` | All players | S once C4 exists | Low. |
| C9 | **Player position and target as the client sees them** | Movement assertions ("reached the cell door") need the client's position, not only the server's. | Lab: a canned Lua reader through the bridge (`client_lua_eval`). All players: a 1 Hz sample from the `FEngineLoop::Tick` hook reading the local pawn's location, once its offset is resolved. | `client.player.position_sample` `{x, y, z, yaw}`, 1 Hz | Lab first | S (lab), M (native) | Low. |
| C10 | **`Debug:log` output** | The stock UI's only Lua logger, and the Black Market overlay's (`[Cimmeria BM]`). **Built 2026-09-29, all players.** It reached no file: the `ScriptedDebug` bindings call `0x0081c2e0`, a bare `ret`. | Inline hooks on the three tolua bindings (`log` `0x00aa1620`, `warn` `0x00aa1710`, `error` `0x00aa1800`), fingerprinted; the detour reads the string argument and runs the binding unchanged. No Lua shim needed. See [client-telemetry.md](../../architecture/client-telemetry.md). | `client.lua.debug_log` `{channel, source, text}`, throttled per (channel, shape) | All players | Done | Low: read-only on a string argument. |
| C11 | **The client's own log files** | `SGWDebugLog.log` and the log4cxx output hold engine-side warnings nobody reads. | Tail the file from the uploader thread, or install a log4cxx appender (hookpoints doc, Tier 1). | `client.log.line` `{source, level, text}` | All players | M | Low (tail), medium (appender). |
| C12 | **The real CME subscribe** | Subscribing to a named signal gives typed payloads with no inline patch, and would make C5 cheaper. | Candidates `FUN_00a37790` and `FUN_00a374a0`, which native subscribers call with a freshly built `MemberCallback` (for example from `FUN_00d351d0`). The invoker at `0x00e04570` pushes two stack arguments and expects the handler to pop both (`ret 8`); the removed thunks popped one, which would also have unbalanced the stack. | Per-event targets | All players | M | Medium: lifetime rules in client-telemetry.md apply. |
| C13 | **Inbound Mercury dispatch** (#989) | Below the method dispatcher: which messages arrived at all. | `0x0157bd30` (`ret 0x10`), unconfirmed. | `client.mercury.dispatch` | Lab | M | Medium. |
| C14 | **Disconnect and error prompts** | A kicked or disconnected client should say why. | Connection events already flow through C1 (`Event_Net_*`, and `Event_NetIn_onErrorCode` when the server sends one). The message box itself needs C4. | covered by C1 and C4 | All players | S | Low. |

Input handling is the other session's area (DirectInput injection). `Event_Action_*` events through C1 already show which input actions the client raised, throttled.

## Backlog: lab bridge and supervisor (for the session that owns them)

These touch `crates/client-telemetry/src/bridge/` and `crates/lab/`, which another session is changing, so they are proposals only.

| ID | Proposal | Why | Effort |
|---|---|---|---|
| B1 | **Sequence numbers and a cursor on the event ring.** Give `LabEvent` a monotonic `seq`, keep a bounded history, and let `events_read` take `since_seq` instead of draining. | Two readers (the agent and `lab_timeline`) currently steal each other's events, and a crash loses what was drained but not yet processed. | S |
| B2 | **`client_wait_for`.** Evaluate a Lua predicate once per tick on the main thread until it is true or a timeout passes, returning the value and the elapsed time. | Agents hand-roll sleep-and-poll loops through `lua_eval`, one round trip per poll. **Built** as a supervisor-side poll (one round trip per poll, not per tick). | S |
| B3 | **Canned state readers** (`client_ui_state`): visible top-level windows, the open dialog's id and text, mission-tracker entries, target, player position, and the error box text, as one Lua probe. | Every step assertion needs these; writing the Lua each time is slow and error-prone. **Built** without target and position. | M |
| B4 | **`lab_timeline` over the full ring**, including `cme.event` and `cegui.log`, not only heartbeats. | Puts "server sent X" and "client accepted X" on one line. | S after B1 |
| B5 | **Character creation in autologin** (`lab_create_character`): drive the creation screen through its Lua module, with name, archetype and appearance as parameters. | The tutorial needs a fresh character each run. **Built** with the native input instead of the Lua module (no appearance parameters). | M |
| B6 | **Server-confirmed world entry**: `lab_login` waits for `client.cme.event` `Event_NetIn_onClientMapLoad` (or the server's `player entered world`) instead of the login windows closing. | "Entered world" is currently a guess. | S |
| B7 | **Screenshot on assertion failure**: a wait that times out attaches a `lab_screenshot`. | A failed step is then diagnosable after the fact. | S |

## Backlog: server endpoint (secondary)

The server is the better-instrumented side, so these rank below the client work. None were built with this document.

| ID | Tool or seam | What | Effort |
|---|---|---|---|
| S1 | `server_player_state` | World, position, level, archetype, health, access level for a session or player id: the live cell snapshot (`LabQuery::EntityGet`) joined with the `sgw_player` row. | S |
| S2 | `server_mission_state` | Active missions with current step and objectives. The live copy is `CellEntity.missions` (a new `LabQuery` variant); `sgw_mission` is the persisted copy, written by `MissionUpdate` a round trip later. The in-memory `counters` (`messhall_kills`) and `fired_once_chains` are never persisted, so only the cell query sees them. | M |
| S3 | `server_inventory` | Read-only `sgw_inventory` rows for a character, with item names from `resources.items`. | S |
| S4 | `server_journal_tail` | Expose the existing per-player `player.journal` ring (64 entries, per-player `seq`, kinds `step_advance`, `dialog`, `kill`, `teleport`, …), which is already the closest thing to a progression feed. Add the two missing kinds: mission accept and item grant. | S |
| S5 | `server_events_since` | A monotonic sequence number on the admin API's `LogBuffer` entries and a cursor read, filtered by target. The buffer holds 500 entries and turns over quickly at debug. | S |
| S6 | `server_wait_for` | Poll a predicate (mission step reached, entity within range, journal kind seen) with a timeout, on the server side. | M |
| S7 | Chain ids on `content.execute_actions` | The span carries no chain id today, so "which chain fired" needs log text. | S |

> The [Cellblock autoplay campaign](../cellblock-autoplay/work-packets.md) (2026-09-29) schedules S1-S4 (AP-13) and the other tools the full tutorial walkthrough needs.

## Suggested order

1. C3 (Lua errors) and B1 (ring cursor): small, and they unblock everything that reads the client.
2. C6 (outbound methods) and C4 (UI window events): together with C1 they make every tutorial click visible end to end.
3. B2, B3 and B6: the agent can then assert steps without hand-written Lua.
4. S1 to S4: cheap server tools that turn the existing journal and tables into assertions.
5. C5, C7 and C12: the payload and entity work, which needs more reverse engineering.
