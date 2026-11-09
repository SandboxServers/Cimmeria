PACKET SP-DHA2-p2 VERIFIED 2026-11-09 by Sonnet; driver attempts: 1 (Sonnet takeover, no Haiku attempt); server evidence window 06:21:32Z-06:23:25Z (local dr-lab server.log, JSON lines; not SigNoz)

Note: the previous grade was BLOCKED by SP-DL1-1 fixture limits. This re-drive uses the SP-DHA2-p1 re-drive state. Character: Fxdcafour, player 107, archetype 8 (Jaffa), access_level 2 (GM account). Carry-over: SP-DHA2-p1 is PARTIAL (a1) and BLOCKED (b1, Jaffa), not FAIL, so the part ran.

| Row | Grade | Tier | Server evidence | Screenshot | Issue |
|---|---|---|---|---|---|
| HA-1360b b2 | BLOCKED | n/a (not driven) | Archetype 8 (Jaffa): chains 6511/6512 need archetype neq 8 and neq 6, so the 1361 offer (DSM 5254, dialog 4457) cannot appear. DB: no mission 1361 row for player 107 (only 1360 exists of 1360/1361). No `6511`/`5254` log row in the window. Fixture limit, not a product finding. | none taken | none |
| HA-1360a a2 | PASS (graded on a GM account; client-side window absent) | N1 (client_world_click on Marsh) | Bookmarks `a2 start` 06:21:37.307. Click reached the server: 06:21:49.155 `interact` player 107 -> Marsh 100199; `interact: no per-player interactions for template` (template 10); `target has no static interaction type`. No `fire_interact_tag`, no `harset.letter_turn_in`, no `mission_completed`, no `Frost letter turn-in request queued`, no dialog set add. DB after: exactly ONE `sgw_mission_1360_receipt` row (06:18:52.879, from the p1 hand-in, unchanged); mission 1360 status 2, completed_step_ids [4037,4038], unchanged; `sgw_inventory` type 3730 has no row for player 107 (4 rows exist for other characters 98, 101, 108, 109). | none | none |
| HA-1360a a3 | PASS (graded on a GM account) | N1 (lab_logout + lab_ensure_in_world, then client_world_click) | Relog: 06:22:00.626 logOff, 06:22:06.314 playCharacter, 06:22:16.364 `missions_restored` completed_count 4, active "" (1360 stays completed, nothing active). 06:22:16.369 `fire_player_loaded: matched` actions 1 = chain 6301 (Moh'Katan, dialog set 5149) only; chain 6502 did NOT re-bind DSM 120101 (correct, 1360 no longer active). Bookmarks `a3 start` 06:22:40.243, `a3 end` 06:23:03.127. Click 06:22:57.705 `interact` -> Marsh 100199, same no-interaction lines as a2; no letter_turn_in, completion or consumption. Receipt table and 3730 inventory unchanged (as above). | lab_screenshot-20261109-002305.190.png (driver: not looked at; no looker run) | none |

Graded on a GM account. No `.give`-type command ran (only `.bug` bookmarks and lab_logout/ensure_in_world). The only WARN in the window is 06:22:06.322 "Archetype has no ability-tree rows" for the Jaffa archetype (known, unrelated to the row; not a finding for this packet).

Shape of the evidence for a2/a3: the repeat click is a no-op by design here. With 1360 completed, chain 6501's conditions (1360 active, step 4038 active) are false and the Marsh dialog set 120101 is not bound, so the server finds no interaction, opens no dialog and consumes nothing. The driver saw no window in either leg, consistent with this. The brief's "no second commit, 3730 unchanged, no second completion" clauses hold.

## Not run
- b2 (1361 offer, dialog 4457, "?" icon): BLOCKED, Jaffa character; needs a non-Jaffa/Goauld character.
- Haiku looker on `HA1360-after-relog-click`: not run (NEEDS_HUMAN item below).
- non_gm repeat: not run.

## Resume hint
To reach b2: stage a non-Jaffa, non-Goauld (archetype not 8, not 6), preferably non-GM character, then run SP-DHA2-p1 (START LEG b1, fixture: 1360 active at step 4038, letter 3730 in the Mission container, world 68 / Harset_CmdCenter) through the real hand-in, then this part. Before the a2 window, read chains 6511/6505 rows (`6511`, `5254`) and confirm 1361 is not in the mission table; the Marsh click should then show dialog 4457. Note for a2/a3 on a non-Jaffa character: after the hand-in, Marsh will have the 1361 offer dialog set bound, so the repeat click WILL open a dialog (4457) rather than being a no-op; grade it as the b2 offer, not as a second turn-in.

## Recipe
1. State: mission 1360 completed on player 107 (status 2), letter 3730 consumed (p1 hand-in).
2. a2: `.bug SPU SP-DHA2-p2 a2 start`; `client_camera` face Marsh 100199 (pitch -10); ONE `client_world_click` template 10, right button. Server shows `interact` with no interaction and no consumption.
3. a3: `lab_logout`, `lab_ensure_in_world` (Fxdcafour, world 68); `.bug ... a3 start`; camera at Marsh; ONE `client_world_click`; `.bug ... a3 end`.
4. Verify with `server_db_query` on `sgw_mission` (1360), `sgw_mission_1360_receipt`, `sgw_inventory` type_id 3730, plus the server.log window.

## SIGN-OFF SHEET
- a3 screenshot lab_screenshot-20261109-002305.190.png (HA1360-after-relog-click): expected no dialog window and no error text. Driver: not looked at. Looker: not run. NEEDS_HUMAN.
- b2 icon "?" over Marsh (`HA1360-marsh-after-complete`): no screenshot (BLOCKED). NEEDS_HUMAN on a non-Jaffa character.
- non_gm: NOT-RUN, needs a non-GM repeat.

## Findings
1. No product findings. b2 is a fixture limit (Jaffa archetype), not a product bug.
2. Observation (not an issue): repeat click on Marsh after completion logs only `interact: no per-player interactions` and no explicit "chain conditions not met" line, so a no-op repeat is only provable by absence plus unchanged DB state. Same telemetry gap as the p1 finding (no `mission_completed`/`harset.letter_turn_in` line exists in the log at all), so the "no second `letter_turn_in`" clause is shown by the DB (single receipt row, mission state, inventory) and not by a positive log row.
