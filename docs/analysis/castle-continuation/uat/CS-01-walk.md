# CS-01 Castle walk: step file for a Haiku lab-driver

> **Type:** Test script (step file). **Audience:** the Castle coordinator and a Haiku `lab-driver`.
> **Status:** Written, never run. Every row is `UATPending`.
> **Companions:** [CS-01 bounded probe](CS-01.md), [Castle ledger](../README.md), [lab-driver brief](../../../../.claude/agents/lab-driver.md), [live research lab](../../../guides/live-research-lab.md), [CS-01a worknote](../worknotes/CS-01a.md).

This file walks a new character from Cellblock arrival through missions 701, 702, 703, 704, 706 and 708 to the Harset gate crossing. It closes the G-ROUTE rows only if every PASS rule holds with a server-log line beside it.

**Ask the user before any lab use and wait until the lab is free.** Do not start this walk without that OK.

## Provenance tags

Each step's expectation carries one tag.

- **[SEED]**: read straight from a seed row (`db/resources/Content/Seed/castle_*_chains.sql`, `Worlds/Seed/spawnlist.sql`, `Events/Seed/point_sets.sql`, `Missions/Seed/mission_steps.sql`). Chain numbers are the seed's.
- **[CODE]**: read from the Rust log strings and GM handlers on `FEATURE_SHA`.
- **[INFER]**: inference, not a seed row. A FAIL on an [INFER] row means "re-check the expectation" before it means "defect".

## Who does what

The Haiku driver has only `cimmeria-lab` tools. It has **no** `lab-server` tools, no `client_minigame_play`, no `lab_livewire_plan`, no `client_input_key`, no `lab_route_record` and no `lab_characters`/`lab_create_character` (see its `disallowedTools`). So:

- **DRV** rows: the Haiku driver runs them from the tool call given.
- **COORD** rows: the coordinator runs them with `lab-server` tools (`server_log_tail`, `server_entity_query`, `server_entity_get`, `server_sessions`, `server_db_query`, `server_nav_path`) or with the driver-forbidden tools. A DRV row's PASS is never decided from the driver's text alone: it needs the COORD log line named in its `Server` field.
- Minigames (the three Livewire boards) are COORD rows: `client_minigame_play` after `lab_livewire_plan`.

## Front-loaded gotchas

1. **One lease per instance.** `p4` is the player, `p5` is the observer. `lab_lease_acquire {owner, purpose, instance: "p4"}` then the same for `p5`. Pass each `lease_id` on every driving call to that instance. Read-only tools without `instance` hit the first client, not yours: always pass `instance` or a lease.
2. **Stand 4-6 m off targets.** The server's interact range is 5.0 m; the driver brief says 3.5-4.5 m works and closer than 3.5 m puts the camera inside the avatar. Never approach with your back to a wall. Three seeded targets sit inside 4 m of another target (`Castle_CommsTerminal` is 3.0 m from `Castle_Zuritska_Comms`; Marsh and Moh'katan are 3.4 m apart; the DHD is 3-4 m from both). Stand on the side that keeps the wanted target at 4-5 m and say which one you used in the record.
3. **Clear dialogs before moving or typing.** An open dialog orphans a chat command. Close with `lab_finish_dialog`. If one is orphaned: `client_batch` Lua `DialogMod.onDialogDoneClicked(DialogWin, DialogWin)`. Close `TutorialWin` first (`client_window_click {target: "TutorialWin__auto_closebutton__"}`) when it sits over a dialog.
4. **Never trust a driver claim without a server-log line.** Every DRV row names the COORD check. No log line, no PASS.
5. **Entity ids change after every relog and zone change.** Look the entity up with `client_entity_find` right before each use. Never reuse an id from an earlier row.
6. **New-character hold.** After the first login the server holds entity introductions for 16 s. Wait 17 s (`client_batch [{op: "wait", ms: 17000}]`) before the first dialog click. A visible NPC in `client_entity_find` confirms it.
7. **Chat commands:** type with `client_ui_sequence {do: "type", text, into: "Inst1Chat_Input"}`, read back with `client_batch` Lua `return Inst1Chat_Input:getText()`, press Enter only on an exact match.
8. **Teleport with `.gotolocation <World> x y z`.** Never `.gotoxyz`.
9. **No row may use the straight-line `client_move_to` result as route evidence.** It does not pathfind (CS-01). A geometry stop is "route not proven", never a defect on its own.
10. **Log level.** The strings in the Server fields are `INFO` unless marked DEBUG. DEBUG lines (`resolve_event: chain matched`) need the Castle server started with a debug filter; if absent, use the INFO line.
11. **One retry, then record and continue.** A DRV action that fails or shows nothing gets one retry of the same call (re-find the entity first, rule 5). If it still fails, record the result in the run log, take the row's stated fallback if it has one, and go on to the next row. Never loop, never repeat a chat command to see if it "took".

## Server log strings used below

All via `server_log_tail {contains: "<string>", since_ms: <row start>}` or `Select-String` on `<castle-cs-01b worktree>\logs\server.log` (the server writes `logs\server.log` as JSON lines, `crates/server/src/logging/mod.rs`; match the `contains` text inside the line).

| String | Meaning | Source |
|---|---|---|
| `Mission accepted` | a mission was accepted (chain or `/gmmissionassign`) | [CODE] `cell/missions/lifecycle.rs:173` |
| `Mission step advanced` | a step advanced | [CODE] `cell/missions/progression.rs:245` |
| `Mission completed!` | a mission completed | [CODE] `cell/missions/progression.rs:559` |
| `fire_dialog_choice: matched` | a `dialog_choice` event matched a chain | [CODE] `named_telemetry_tests.rs:195` |
| `resolve_event: chain matched` (DEBUG) | a chain matched; field `chain_name` | [CODE] `named_telemetry_tests.rs:200` |
| `gmMissionAssign: assigned mission` | GM grant succeeded | [CODE] `cell-console/gm/missions.rs:170` |

These log lines carry mission and step ids in fields. If the field names differ, record the line verbatim and grade on mission id + step id.

## Preconditions

| Item | Value | Tag |
|---|---|---|
| Server | Castle server from worktree `castle-cs-01b`, DB `sgw_castle_cs_01b`. Never `reload-db.ps1` or `live-db-test.ps1` there while it runs. | [SEED] resume-packets |
| Not touched | Cellblock's server, worktrees, DBs. This walk uses only the Castle server's own Cellblock world copy. | rule |
| Build identity | Before row S-00, the coordinator records the client build and the server build hash and checks that the lab-server endpoint is the Castle server. CS-01 failed this (`server_sessions` returned zero while the client was in world 8). | [CODE] CS-01 |
| Instances | `p4` (player), `p5` (observer). `lab instances status` must show both. | lab guide |
| Character A (ordinary) | Archetype `Soldier`, `praxis`. Any non-Jaffa archetype satisfies chains `archetype neq 8`. | [SEED] chain 1202 |
| Character B (Jaffa) | Archetype `Jaffa`, `sgu`. Chains `archetype eq 8` (1203, 1205, 1351, 1354, 1355, 1359, 1363) fire only for it. The seed names the archetype value 8 in the chains; the archetype table lists `ARCHETYPE_Jaffa` and `ARCHETYPE_Sholva` as separate rows, so the coordinator confirms the created Jaffa gets value 8 (`server_entity_get`). | [INFER] |
| Character creation | The driver cannot create characters. The coordinator creates A and B, or runs `lab_ensure_in_world {create: {alignment, archetype, gender}}` on `p4` for each in turn. | [CODE] automated-uat.md |
| Observer | `p5`: any character in Castle, standing at the S-OBS positions. | rule |
| GM account | The lab account must be able to run `.gotolocation` (CS-01 did). `/gmmissionassign`, `/gmmissionadvance`, `/gmmissionclear` are the documented GM commands (`docs/commands.md:369-371`; they dispatch to `gmMissionAssign` 109, `gmMissionClear` 110, `gmMissionAdvance` 116). `/gmmissionreset` is documented (`docs/commands.md:378`) but not implemented, and must not be used. | [CODE] dispatch table |
| Reset between runs | There is no mission reset. To rerun a mission use `/gmmissionclear <missionId>` (abandon) then re-enter by the natural path, or use a fresh character. Completed missions are not re-enterable. | [CODE] |

GM commands are typed in chat as `/gmmissionassign 701 1`, `/gmmissionadvance 701 2400`, `/gmmissionclear 701`. The argument order is `<missionId> <popup>` for assign (popup is a UI hint; `1` as in the docs example), `<missionId> <step>` for advance, and `<missionId>` for clear [docs/commands.md:369-371]. The spellings are the documented ones; if the chat refuses one, record the refusal text and follow the row's fallback. A GM jump is allowed only in the rows marked **GM-SKIP**; a natural row never uses one.

## Per-step record

Every row, driver and coordinator, fills these fields. Put them in one line per row in the run log:

`row | position (server x,y,z from player.position.server) | timestamp (UTC, ms) | step state (mission id: step id: active/completed from server_db_query or client_ui_state) | evidence (log line, screenshot path, tool result key)`

Take the position from any `client_entity_find` result (`player.position.server`). Take the step state from the coordinator's `server_db_query` read-only query of the character's saved mission rows, or, between saves, from the mission log text in the client.

## Coordinates

All positions are server metres, from `spawnlist.sql` world 8 unless stated.

| Tag | x, y, z | Spawn | Template |
|---|---|---|---|
| Ring landing (chain 1109 target) | 466.365, 70.397, 991.466 | n/a | n/a |
| `Castle_SgtGerschon` | 429.64, 70.11, 996.56 | 112 | 149 |
| `Castle_PreRomneyChest` | 350.0, 70.38, 997.0 | 283 | 410 |
| `Castle_Coppleman` | 352.69, 70.27, 952.32 | 87 | 48 |
| Ogilvie (Infirmary) | 362, 70.18, 886 | cooked map | n/a |
| `Castle_Zuritska_Cell` | 268.0, 66.79, 1042.59 | 238 | 168 |
| `Castle_Romney` | 244.0, 66.79, 1036.0 | 240 | 169 |
| `Castle_Zuritska_Comms` | 271.7, 55.2, 858.0 | 239 | 168 |
| `Castle_CommsTerminal` | 271.7, 55.2, 855.0 | 246 | 173 |
| `Castle_AccessPanel` | 330.49, 41.18, 653.11 | 92 | 147 |
| `Castle_SurrenderGuard` | 958.0, 25.0, 480.0 | 245 | 172 |
| `Castle_BravoOfficer1/2/3` | (955,25,475) / (965,25,485) / (968,25.2,477) | 242-244 | 171 |
| `Castle_Muelbach` | 1008.0, 48.0, 414.0 | 241 | 170 |
| `Castle_ColMarsh` | 810.73, 55.20, 515.01 | 118 | 10 |
| `Castle_Mohkatan` | 807.48, 55.21, 515.40 | 120 | 54 |
| `Castle_DHD` | 806.27, 55.10, 517.24 | 2 | 162 |

Regions (`Events/Seed/point_sets.sql`, bounding boxes from `point_set_points.sql`):

- `Castle.InterrogationBlock` (2082): x 226 to 336, z 1018 to 1103, y 66.79 to 74.79.
- `Castle.CommsRoom` (2083): x 260 to 284, z 848 to 863, y 55.2 to 63.1.
- `Castle.ThroneRoom` (2049): x 331 to 396, z 617 to 683, y 41 to 48.

`CA-SecurityLock00_Pf0` is at (399.79, 58.91, 919.41) and the chamber prefab cluster at (398.83, 55.04, 929 to 941): both [CLIENT_MAP] from the CS-01 route boundary, not spawn rows.

## Phase 0: setup (COORD then DRV)

**S-00 (COORD).** Acquire both leases. `lab_client_status` on both instances. Record the build hashes. `server_sessions` must list the `p4` and `p5` sessions after S-01. PASS: both clients report a bridge and `server_sessions` is non-empty. FAIL or INCONCLUSIVE otherwise: stop the walk (the CS-01 mismatch).

**S-01 (DRV, p4).** `lab_ensure_in_world {lease_id: L4}` as character A. Then `client_batch [{op:"wait", ms:17000}]`. Then `client_entity_find {lease_id: L4, fields: ["player"]}` (or any find) to read `player.position.server`.
- Expect: in world, a Cellblock world id, not 8. [INFER]
- Server: COORD `server_sessions` shows the session; `server_log_tail {contains: "player_loaded"}` shows a load line for the character. [INFER: exact string]
- PASS: both. FAIL: either missing.

**S-02 (DRV, p5).** `lab_ensure_in_world {lease_id: L5}`. Leave p5 idle until S-OBS-1.

## Phase 1: Cellblock arrival (chain 1109)

**S-10 (COORD) prepare mission 688.** The character A needs mission 688 on step 80688. Natural route is the Cellblock path (out of this file's scope). Use **GM-SKIP**: `/gmmissionassign 688 1`, then `/gmmissionadvance 688 80688` typed by the driver (S-11). The coordinator reads the Armory ring switch position with `server_entity_query {space_id: <A's space>, tag: "Cellblock_ArmoryRingSwitch"}`.

**S-11 (DRV).** Type `/gmmissionassign 688 1`, then `/gmmissionadvance 688 80688`, each with the chat procedure (gotcha 7). Expect feedback `gmMissionAssign: assigned mission 688` [CODE].
- Server: `Mission accepted` with mission 688; the step read shows 688: 80688 active.
- PASS: both. FAIL: GM line says "no mission def" or the step read differs.
- If `/gmmissionadvance` is refused, do not retry: a refusal of this kind is deterministic. Record the refusal text and continue; S-12 then uses its fallback. Gap G-01 in the worknote (no natural 688 route in this file).

**S-12 (DRV).** Walk to within 4 m of the Armory ring switch (position from S-10; `client_move_to {x,y,z}` in hops of at most 20 m, read position after each hop). Aim with `client_camera {face_entity_id}`, `lab_screenshot`, then `client_world_click {entity_id, expect: "window", settle_ms: 5000}` on the switch (`client_entity_find {name: "Ring"}` first, or by the position from S-10).
- Expect: the client changes to world 8 and the character appears at about (466.365, 70.397, 991.466). [SEED chain 1109, `cross_world_teleport` target]
- Server: `Mission completed!` for mission 688, then a cross-world teleport line (record verbatim; string [INFER]).
- PASS: world 8 and position within 5 m of the landing and the 688 completion line. FAIL: any missing.
- **Retry**: if the click opens nothing, retry the same click once (gotcha 11). **Fallback** if still nothing: **GM-SKIP** `.gotolocation Castle` (CS-01 saw it return the pad coordinates and world 8). Record `fallback: true`. The fallback proves the landing only, not chain 1109.

**S-13 (DRV).** Wait 17 s if this is the first Castle load on this character. `client_entity_find {name: "Gerschon"}`. Expect Gerschon in view. [SEED spawn 112]
- PASS: found at about (429.64, 70.11, 996.56), 37 m from the landing.

## Phase 2: mission 701

**S-20 (DRV): Gerschon offer, ordinary.** Move to 4-5 m from Gerschon. Camera, screenshot, `client_world_click {entity_id, expect: "window"}`.
- Expect: dialog 2573 opens. [SEED chain 1202]
- Server: `resolve_event: chain matched` chain 1202 (or the dialog-open INFO line, string [INFER]).
- PASS: DialogWin opens and chain 1202 matched. For character B the dialog is 5861 (chain 1203); the Accept button is on the last screen 96789 (DU-02b), so read to the end. [SEED]

**S-21 (DRV): accept.** Advance to the last screen with the dialog's next/Done control. On the last screen, read the window list with `client_ui_state`, then click the window whose text is `Accept`. The repo names it: dialog 2573's last screen 113558 ("Get it done.") has one button, `button_id` 8, type 2, text `Accept` [SEED db/resources/Dialogs/Seed/dialog_screen_buttons.sql:4466]. Record the window name read from `client_ui_state`. If nothing changes, retry once (gotcha 11); if the dialog still does not close, close it with `lab_finish_dialog`, record the window list, and continue.
- Expect: dialog closes, the mission log shows 701 step 2399 "Capt. Copplemann is pinned down in the hallway. Go support her!" [SEED]
- Server: `fire_dialog_choice: matched` with dialog 2573 (B: 5861); `Mission accepted` mission 701; the step read 701: 2399 active.
- PASS: all. FAIL: no `fire_dialog_choice: matched` (the button raised no choice).
- B only: dialog 5862 opens after accept [SEED chain 1205]. Close it with `lab_finish_dialog`.

**S-22 (DRV): Pre-Romney chest, optional.** Stand 4 m from `Castle_PreRomneyChest` (350.0, 70.38, 997.0). Right-click. Expect a loot window with table 8 (A) or table 9 (B). [SEED chains 1274/1275, `once_per_character`] Close with the loot window close. Skip if mission 703 is not active yet: with 703 inactive chain 1276 says the chest is empty. Purpose: checks the once-per-character chest gate while 703 is active (the loot table is the only observable), so the observer can tell chain 1274/1275 apart from 1276.
- PASS: loot window; COORD sees `open_loot` or the loot log line (string [INFER]).

**S-23 (DRV): Copplemann step 2399.** Walk to 4-5 m from Copplemann (352.69, 70.27, 952.32). Click.
- Expect: dialog 2574. [SEED chain 1231]
- Server: chain 1231 matched.

**S-24 (DRV).** Click the 2574 choice.
- Expect: step 2400 "Free Capt. Copplemann from her security boot." [SEED chain 1232]
- Server: `fire_dialog_choice: matched` 2574; `Mission step advanced` 701 to 2400.
- PASS: both.

**S-25 (DRV + COORD): Livewire at Copplemann.** DRV clicks Copplemann again. Expect a Livewire minigame window opens. [SEED chain 1233]
- COORD: `lab_livewire_plan`, then `client_minigame_play` on `p4`. Server: the Livewire win line (string [INFER]).
- Expect after the win: dialog 2575 opens and step 2401 starts. [SEED chain 1234: `display_dialog 2575`, `advance_step 2401`]
- PASS: window opens, win logged, `Mission step advanced` to 2401.

**S-26 (DRV): escort delay.** Click the 2575 choice. Wait 11 s (the seed delay is 10500 ms). [SEED chain 1235]
- Expect: step 2421 "Speak to Capt. Copplemann to receive further orders." [SEED]
- Server: `fire_dialog_choice: matched` 2575, then `Mission step advanced` to 2421 about 10.5 s later.
- PASS: both, with the timestamps 10 to 11.5 s apart. A relog during the 10.5 s wait re-arms the timer (chain 1242, 2401 active); do not relog here.

**S-27 (DRV): turn in.** Click Copplemann. Expect dialog 2576. [SEED chain 1236] Click the choice.
- Expect: 701 complete, 702 "Make your way to the Interrogation Block." and 703 "Locate NID Interrogator Romney" in the log. [SEED chains 1237 to 1239]
- Server: `Mission completed!` 701; `Mission accepted` 702 and 703; step reads 702: 2402, 703: 2403 active.
- PASS: all. This is one choice raising three chains (1237, 1238, 1239); a missing accept is a chain-order defect (trap 3 in the mission-system notes).

## Phase 3: missions 702 and 703

**S-30 (DRV): enter the Interrogation Block.** The route Copplemann to the block is not specified in any seed. Walk with `client_move_to` hops towards (300, 66.79, 1050); read the position after each hop. Record every stop.
- Expect: entering the block (x 226 to 336, z 1018 to 1103) advances 702 to 2419 and 703 to 2404. [SEED chains 1261 and 1271]
- Server: `Mission step advanced` for 702 to 2419 and for 703 to 2404.
- PASS: both advances. If the hop is still stuck after the retry, record the stop position and use **GM-SKIP** `.gotolocation Castle 300 66.79 1050` with `fallback: true`, then continue; the fallback proves the region event, not the walk (gap G-02). Mark the row INCONCLUSIVE, not FAIL, when the fallback was used.

**S-31 (DRV): free Zuritska.** Stand 4-5 m from `Castle_Zuritska_Cell` (268.0, 66.79, 1042.59). Expect the quest cue (`INT_AStoryMissionActive`). [SEED chain 1261] Click. Expect dialog 2577. [SEED chain 1262] Click the choice.
- Expect: 702 complete, 704 accepted, Zuritska follows. [SEED chain 1263]
- Server: `Mission completed!` 702; `Mission accepted` 704; step read 704: 2405 active.
- PASS: all three. Record the follow: take `client_entity_find {name: "Zuritska"}` twice 5 s apart while walking; the followed entity's position must move.

**S-32 (COORD): Romney kill, optional.** Romney (244.0, 66.79, 1036.0) is level-bound; the kill is combat. This file does not drive combat: **GM-SKIP** is not available for a kill. List it as gap G-03. If the coordinator kills Romney through another means, record `Mission completed!` 703 and item 2135 granted (`add_item 2135`, chains 1272/1273).

**Relog point R-1 (after the Zuritska rescue).** With 704 on step 2405:
1. DRV: `lab_logout {lease_id: L4}`; then `lab_ensure_in_world {lease_id: L4}`; wait 17 s.
2. Expect: character is in world 8; step read 704: 2405 active (not NULL). [SEED chain 1296]
3. DRV: `client_entity_find {name: "Zuritska"}`. Expect a Zuritska, flagged for the mission (cue present) near the cell, and it follows again after one click. [SEED chain 1296: re-arms follow and the cell cue; chain 1302: clicking Zuritska on 2405 re-arms the follow]
4. Server: `resolve_event: chain matched` 1296 (DEBUG) or `Mission accepted` is NOT logged again. A second `Mission accepted` for 702, 703 or 704 is a FAIL (a re-accept on relog).
- PASS: step still 2405, a Zuritska follows after the click, no re-accept.

## Phase 4: mission 704

**S-40 (DRV): escort to the Comms Room.** Walk with Zuritska following to the Comms Room (x 260 to 284, z 848 to 863). The route from the block is not in a seed. Hop with `client_move_to` towards (272, 55.2, 870). Record every geometry stop. The playtest saw the escort pass through walls and floors (54 follow legs, each a straight step); the PASS rule judges the mission state, not the escort path.
- Expect on entering the room: 704 to 2406 "Use the Communications Terminal…". [SEED chain 1291]
- Server: `Mission step advanced` 704 to 2406.
- PASS: advance logged. If the escort hops stall, retry the stalled hop once (gotcha 11). If still stalled, use **GM-SKIP** `.gotolocation Castle 272 55.2 870` (`fallback: true`, gap G-02) and continue.

**S-41 (DRV): see the second Zuritska.** After entry, `client_entity_find {name: "Zuritska"}` must return a second entity at about (271.7, 55.2, 858.0). [SEED spawn 239] Record both entities' positions. The cell Zuritska should have gone back to (268.0, 66.79, 1042.59) [SEED chain 1291 `move_waypoint`].
- PASS: two Zuritska entities in the list, one near each position. This is the "duplicate Zuritska" and is expected seed content; the check is that both exist and that p5 sees the same two (S-OBS-1).

**S-42 (DRV + COORD): terminal.** Stand 4-5 m from `Castle_CommsTerminal` (271.7, 55.2, 855.0). Click. Expect a Livewire window. [SEED chain 1292] COORD wins the board. Expect dialog 2580 and an item "Data Crystal" (item 5029) in the bag, step 2407 "Deliver the Data crystal to Zuritska." [SEED chain 1293]
- Server: win line; `Mission step advanced` to 2407.
- DRV: `client_inventory {lease_id: L4}` must list item 5029.
- PASS: all three.

**S-43 (DRV): deliver.** Click `Castle_Zuritska_Comms` (the one near 271.7, 55.2, 858.0). Expect dialog 2581. [SEED chain 1294] Click the choice.
- Expect: crystal removed, 704 complete, 706 "Get to the Throne Room." [SEED chain 1295]
- Server: `Mission completed!` 704; `Mission accepted` 706; step read 706: 2411; inventory no longer lists 5029.
- PASS: all.

## Phase 5: mission 706

**S-50 (DRV): walk to the Throne Room.** Region x 331 to 396, z 617 to 683, y 41 to 48. No seed gives the route from the Comms Room (gap G-02). Hop with `client_move_to` towards (360, 41, 650). Record every stop.
- Expect: 706 to 2412 "Hack into the Goa'uld communications grid." and a glow on the Access Panel. [SEED chain 1321]
- PASS: `Mission step advanced` 706 to 2412. If the hops stall, retry the stalled hop once (gotcha 11). If still stalled, use **GM-SKIP** `.gotolocation Castle 360 41 650` (record `fallback: true`) and continue.

**S-51 (DRV): access panel.** Stand 4-5 m from `Castle_AccessPanel` (330.49, 41.18, 653.11). Click. Expect dialog 2584 and 706 complete, 708 "Discover why the Stargate will not dial out." accepted. [SEED chain 1322, note 1344 shares the tag]
- Server: `Mission completed!` 706; `Mission accepted` 708; step read 708: 2415 active.
- PASS: all. Also check that dialog 5004 did NOT open (chain 1344 needs 708 step 2415 active at event time; trap 3).

## Phase 6: mission 708 and the Harset gate

**S-60 (DRV): step 2415.** Walk towards the Armory (surrender guard (958,25,480)). The route from the Throne Room is not in a seed (gap G-02). Hop in 20 m legs towards (950, 25, 480). Record every stop. A stalled hop is retried once (gotcha 11). If still stalled, use **GM-SKIP** `.gotolocation Castle 950 25 480` (`fallback: true`) and continue.
- Click `Castle_SurrenderGuard`. Expect dialog 5003. [SEED chain 1342] Click the choice. Expect objective 2794 complete and the step advance to 2416 "Retrieve the DHD Control Crystal." [SEED chain 1343]
- Alternatively click the Access Panel diagnostic path (chains 1344/1345, dialog 5004). Only one of the two is needed; the panel tag also fires chain 1322 if 706 were still on 2412.
- Server: `fire_dialog_choice: matched` 5003; `Mission step advanced` 708 to 2416.
- PASS: both.

**S-61 (COORD): relog point R-2a (2415).** If run before S-60, `lab_logout` and `lab_ensure_in_world`, then the guard cue returns (chain 1361). Optional.

**S-62 (COORD + DRV): crystal.** Kill one of the Bravo officers or Warden Muelbach. Combat is outside this file (gap G-03). **GM-SKIP** for the state only: `/gmmissionadvance 708 2417`, and record that the crystal item (2790) was NOT granted, so chain 1346 to 1349 are unexercised. If the coordinator can kill an officer by other means: expect item 2790 and step 2417 "Take the Control Crystal to Checkpoint Alpha." [SEED chains 1346 to 1348] and the report-to cue on Marsh (ordinary) or Moh'katan (Jaffa). [SEED chains 1350/1351]
- PASS (kill path): item 2790 in the bag, step 2417 active, one `Mission step advanced` (a second one from a second officer is a FAIL: the chains share the step gate).

**S-63 (DRV): report to Marsh or Moh'katan.** Walk to the DHD area, about (808, 55.2, 515). Ordinary character A: stand 4-5 m from `Castle_ColMarsh` (810.73, 55.20, 515.01). Click. Expect dialog 5008. [SEED chain 1352] Click the choice. Expect objective 5185 complete, step 2418 "Repair the DHD." [SEED chain 1353] Jaffa B: `Castle_Mohkatan` (807.48, 55.21, 515.40), dialog 5009, objective 5186. [SEED chains 1354/1355]
- Server: `fire_dialog_choice: matched` 5008 (B: 5009); `Mission step advanced` 708 to 2418.
- PASS: both. The wrong archetype's NPC must not open the report dialog.

**Natural walk W-2: Armory to the DHD (G-ROUTE).** The target is (806.27, 55.10, 517.24). The Armory officers are at x 955 to 968, z 475 to 485.
1. COORD: `server_nav_path {space_id: <Castle space>, from: [958, 25, 480], to: [806.27, 55.10, 517.24]}` before the row. The CS-01 notes say the DHD has a tested mesh path to Bravo bunker, not from the Armory. A `partial` or `no_path` reply is the expected evidence, not a pass.
2. DRV: walk from the guard to the DHD with `client_move_to` hops. If `status` is `ok`, use the returned `waypoints` as the hops. Otherwise hop towards (806, 55, 517) in 20 m legs. Read the position after every hop.
3. PASS: reaches within 5 m of the DHD on foot with no GM jump. FAIL: stops with the position recorded. A geometry stop with `no_path` is `G-ROUTE open` (it is not a defect claim).
- S-60 and S-63 use a GM jump if W-2 fails; the walk result is still recorded.

**S-64 (DRV + COORD): repair the DHD.** Stand 4-5 m from `Castle_DHD`. Click. Expect a Livewire window. [SEED chain 1356] COORD wins. Expect step 4462 "Use the DHD to dial the Stargate to Harset." and `grant_stargate_address 3`. [SEED chain 1357]
- Server: win line; `Mission step advanced` to 4462.
- PASS: both.

**Relog point R-2 (after the DHD).** Step 4462 active.
1. DRV: `lab_logout`, `lab_ensure_in_world`, wait 17 s.
2. Expect: world 8, step read 708: 4462 active. [SEED chain 1365 re-binds the dial topic]
3. Server: no second `Mission accepted` for 708.
- PASS: step still 4462. FAIL: NULL step or a restart at 2415.

**S-65 (DRV): dial Harset.** The playtest (H10) saw 76 dead clicks on the DHD after the repair and dialog-set 3073 resolving to `dialog_id = None`. Whether the feature branch fixed that is **not proven** in this file [INFER]. Click the DHD; retry the click once (gotcha 11). Record the window that opens, or take `lab_screenshot` and record its path. Not matchable: no server log line is grep-able for an unhandled click (`unhandled_interaction_flag` appears only in playtest docs, not in any crate), so the server-side check for this row is the step read in the Server line below.
- Expect if the gate dials: `stargate_dialed` for `Harset`, step 4469 "Enter the active Stargate to leave the Castle." and dialog 5010 (A) or 5011 (B). [SEED chains 1358/1359]
- Server: `Mission step advanced` to 4469.
- PASS: step 4469 active. FAIL (with the screenshot path recorded): defect H10 still open.

**S-66 (DRV): cross.** Walk into the active Stargate. The prefab position is not a spawn row (the DHD spawn is the nearest anchor; gap G-04). Hop towards the gate, record the stop. Expect the world change to Harset and 708 complete. [SEED chain 1360, `stargate_crossed` key `Harset`]
- Server: `Mission completed!` 708. If the 1360 carry-in is present at this commit: step 4037 to 4038 requires mission 1360 active, so the row only applies to a character that holds 1360 (CS-08a, not this file).
- PASS: `Mission completed!` 708 and `client_player_state` shows a Harset world id. A crossing without 4469 is a FAIL.

**Natural walk W-3: Harset to Castle landing (G-ROUTE).** On a character already in Harset (a separate run, or A after S-66): cross back through the Harset gate. Record the landing position, `client_player_state` world id 8, and whether the character can move after landing (`client_move_to` 3 m). The seed gives no return arrival point: the CS-01 notes accept the landing as unvalidated on this advisory navmesh, so only this client row can prove it. PASS: world 8, a position inside walkable space (a 3 m `client_move_to` moves the position), no fall (y within 5 m of the DHD y 55.1 for the Castle gate side). Record the landing x, y, z. [INFER]

## Natural walk W-1: Infirmary to the chamber cluster (G-ROUTE)

Run on a character already in Castle. Ogilvie is at (362, 70.18, 886); the target is the chamber cluster (398.83, 55.04, 929 to 941); `CA-SecurityLock00_Pf0` is at (399.79, 58.91, 919.41).

1. COORD: `server_nav_path {space_id: <Castle space>, from: [362, 70.18, 886], to: [398.83, 55.04, 935]}` and record `status`, `reason`, `end_snap`. The CS-01 notes say no sourced stair or door connects the two. A non-`ok` reply is the expected result; the world is `advisory`, so it does not reject a player walk.
2. DRV: `.gotolocation Castle 362 70 886` (GM-SKIP, setup only). `client_entity_find {name: "Ogilvie"}` must find an entity at (362, 70.18, 886) [CS-01 saw entity 100102; the id changes].
3. DRV: walk to the cluster. If step 1 was `ok`, hop through its `waypoints`. Otherwise hop 10 m legs towards (386, 55.38, 940) and record every geometry stop with its position. CS-01 already recorded a stop at about (371.07, 70.16, 889.75) on a straight line: do not repeat that as evidence.
4. PASS: reaches within 5 m of (398.83, 55.04, 935) with no GM jump. FAIL: record the last position and the stop reason. A stop on a straight-line `client_move_to` leaves G-ROUTE open (it is not a defect).
5. Walk back to Ogilvie and record the result the same way.

## Observer checks (p5)

**S-OBS-1: shared actor presentation, including the duplicate Zuritska.** After S-41 (p4 in the Comms Room):
1. DRV p5: `.gotolocation Castle 262 55.2 870` (setup), then `client_entity_find {instance: "p5", name: "Zuritska"}` with p5's lease.
2. COORD: `server_entity_query {space_id, tag: "Castle_Zuritska_Cell"}` and the same for `Castle_Zuritska_Comms`. Both must return one entity.
3. Expect on p5: Zuritska at (271.7, 55.2, 858.0) is visible. The cell Zuritska should be where `move_waypoint` put it, (268.0, 66.79, 1042.59), not following p4.
4. PASS: p5 sees the same two entities (names and positions within 1 m of the COORD reads). FAIL: p5 sees a follower at p4's position, or one Zuritska fewer. Record both lists. [INFER: "same actor" means position agreement]

**S-OBS-2: per-player cues.** While p4 holds 704 step 2406, p5 (no 704) stands 4-5 m from `Castle_CommsTerminal`. `client_entity_find` on p5 and `client_hover_probe`. Expect no Livewire cue visible to p5 [INFER: `set_interaction_type` is per-player in chains 1291/1297; the DB flag is a shared actor attribute on shared NPCs, so this is the check that decides it]. Record what p5 sees. Gap G-05 if the cue shows on p5.

**S-OBS-3: dialog-set binds.** p5 clicks `Castle_Coppleman` while p4 holds 701 step 2399 (before S-24): p5 must not open dialog 2574 [SEED chain 1231 gates on p5's own step state]. PASS: no dialog on p5 and no chain 1231 line for p5.

## Row summary for the run report

| Phase | Rows | Needs COORD | Needs GM-SKIP |
|---|---|---|---|
| 0 setup | S-00 to S-02 | S-00 | none |
| 1 arrival | S-10 to S-13 | S-10 | S-11 (688 grant), S-12 fallback |
| 2 mission 701 | S-20 to S-27 | S-25 (Livewire) | none |
| 3 mission 702/703 | S-30 to S-32, R-1 | S-32 | S-30 fallback |
| 4 mission 704 | S-40 to S-43 | S-42 (Livewire) | S-40 fallback |
| 5 mission 706 | S-50, S-51 | none | S-50 fallback |
| 6 mission 708, gate | S-60 to S-66, R-2, W-2, W-3 | S-62, S-64 | S-60, S-62 |
| G-ROUTE walks | W-1, W-2, W-3 | nav_path reads | setup only |
| Observer | S-OBS-1 to S-OBS-3 | S-OBS-1 | setup only |

Release both leases at the end, even after a failure.
