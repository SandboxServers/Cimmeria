PACKET SP-DHA2-p1 VERIFIED 2026-11-09 by Sonnet; driver attempts: 2 (1 Haiku + 1 Sonnet takeover, graded from the takeover); server evidence window 06:17:50Z-06:19:25Z (local dr-lab server.log, JSON lines; not SigNoz)

Note: the previous grade was BLOCKED by SP-DL1-1 fixture limits. This re-drive staged the fixture (1360 active at step 4038, letter 3730 present). Character: Fxdcafour, player 107, archetype 8 (Jaffa), access_level 2 (GM account).

| Row | Grade | Tier | Server evidence | Screenshot | Issue |
|---|---|---|---|---|---|
| HA-1360b b1 | BLOCKED | N1 (relog via lab_logout + lab_ensure_in_world) | Archetype is Jaffa, so the brief says BLOCKED (the 1361 offer cannot appear). The rebind itself held: 06:18:04.64 `fire_player_loaded: matched` (actions 2), then `Content: adding dialog set` chain 6502, DSM 120101 "Letters Home", template 10, slot rebind; `missions_restored` active `1360:4038`, 3 completed. Mission and letter unchanged by the relog. | lab_screenshot-20261109-001856.866.png (icon not looked at; NEEDS_HUMAN) | none |
| HA-1360a a1 | PARTIAL | N1 (client_world_click on Marsh) | 06:18:52.877 `fire_interact_tag: matched` tag `CmdCenter_Marsh` actions 1; 06:18:52.878 `Frost letter turn-in request queued` (Marsh entity 100199); exactly ONE receipt row in `sgw_mission_1360_receipt` (06:18:52.879, item 10666, stack 1 -> 0); 06:18:52.892 `Frost letter persisted mission installed` stack_after 0; 06:19:01.706 `dialogButtonChoice` dialog 4576 "Letters Home" (dialog closed). DB: mission 1360 status 2 (completed), completed_step_ids [4037,4038], current_step null; `sgw_inventory` type 3730 now 0 rows. No WARN or ERROR in the window. Gaps: no `mission_completed` / `harset.letter_turn_in` log line for 1360 exists in the log (the only `mission_completed` row is for player 108, mission 688), so the "chain event" clause rests on the DB state; the "processing" feedback line ("Checking Frost's Letter...") is client-read only. | lab_screenshot-20261109-001856.866.png | none |

Graded on a GM account (the lab accounts are GM). No `.give`-type command ran in the takeover; the letter came from the fixture. Cellblock accept/grant and the Castle crossing belong to SP-DL2.

## Not run
- a1.10 full bag: no way to fill a bag.
- A1.13: not claimed.
- Haiku looker screenshots: not run (all visual items are NEEDS_HUMAN and listed below).
- Non-GM repeat: not run.

## Resume hint
Server clauses for the hand-in are done. A next attempt only needs: (1) a non-GM character (`non_gm`), (2) a non-Jaffa/Goauld character if HA-1360b should reach the 1361 offer (chains 6511/6512 need archetype neq 8 and neq 6). START LEG b1 with fixture: 1360 active at step 4038, letter 3730 in the Mission container, character in Harset_CmdCenter (world 68). Note leftover state: mission 1360 is now completed on player 107, so restage the fixture first.

## Recipe
1. Fixture: 1360 at step 4038, letter 3730 (item instance 10666) present on player 107 (was staged before this re-drive).
2. `lab_logout`, then `lab_ensure_in_world` (second world load fires chain 6502). Bookmark `.bug SPU SP-DHA2-p1 b1 end`, then `.bug SPU SP-DHA2-p1 a1 start`.
3. `client_camera` pitch -10 with face_entity_id 100199 (Colonel Marsh), then ONE `client_world_click` with template 10, button right, expect window. (The Haiku attempt's `client_world_click` timed out at hover_timeout: facing Marsh first fixed it.)
4. `lab_finish_dialog`: three pages (Next, Next, Done), title "Colonel Marsh".
5. Verify with `server_db_query` on `sgw_mission`, `sgw_mission_1360_receipt`, `sgw_inventory` (type_id 3730).

## SIGN-OFF SHEET
- b1 icon: Marsh shows a "!" after the relog? Screenshot lab_screenshot-20261109-001856.866.png (the same path the driver named; no looker run). Driver sentence: none recorded for the icon.
- a1 dialog 4576: three screens, speaker Marsh. Driver: DialogWin opened with title "Colonel Marsh", 3 pages clicked. Screenshot lab_screenshot-20261109-001856.866.png.
- non_gm: NOT-RUN, needs a non-GM repeat.

## Findings
1. Telemetry gap (row HA-1360a): the letter hand-in completes mission 1360 (DB status 2) but writes no `mission_completed` event and no `harset.letter_turn_in` line in the server log; only the "request queued" and "persisted mission installed" lines exist. The brief expects both. No issue file written: whether this is a missing event or a doc/brief mismatch is for the coordinator to decide (the executor path is `crates/content-engine/src/loader/action_letter_turn_in.rs`; not investigated further).
2. Minor: mission 1360 shows `active_objective_ids` [4651] still set after completion (status 2). Possibly expected; not investigated, no issue.
3. Harness (not a product issue): the Haiku driver's `client_world_click` on Marsh timed out at hover_timeout and lab_disconnect was missing from its toolset; camera-facing first solved it.
