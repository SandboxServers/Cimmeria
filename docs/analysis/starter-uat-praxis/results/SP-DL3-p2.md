PACKET SP-DL3-p2 VERIFIED 2026-11-09T07:2xZ by Sonnet; driver attempts: 1; server evidence window 07:17:55Z-07:20:30Z (dr-lab logs/server.log + DB)

Deviations from the brief: instance p2, reused character Fxdltwo (player_id 109, no "Uat " prefix). The client showed item id 10668 "Frost's Letter"; graded from the DB (sgw_inventory.type_id 3730). Extra missions 622 and 689 are fixture side effects, ignored. Evidence source is the server.log file (the lab-server memory buffer and SigNoz were not used).

| Row | Grade | Tier | Server evidence | Screenshot | Issue |
|---|---|---|---|---|---|
| LEG-3b fixture | PASS | G | Bookmark `leg3b start` 07:18:00: world Harset, pos (-0.08, -67.26, 38.01) on_navmesh true, mission 1360 step 4038 status 1 (completed step 4037). DB: sgw_inventory character 109 container 2 item 10668 type_id 3730 stack 1 | - | - |
| Part B dial | PASS | G (`/gmdhd 4` is the step) | 07:18:06.676 "GM-gated cell method authorized" gmDHD idx 159; 07:18:06.676 "gmDHD: dialing stargate" gate_addr 4; WARN reason=gm_address_grant (expected); 07:18:06.677 "dial accepted, gate opens in 4s" Harset -> Tollana | - | - |
| Crossing | PASS | G | 07:18:14.809 "Stargate passed - crossing"; 07:18:16.403 "Gate travel: sending RESET_ENTITIES for world transition" world Tollana, player 109; 07:18:21.230 fire_player_loaded world Tollana. One travel, after the dial | - | - |
| Destination and navmesh | PASS | G | `leg3b end` bookmark 07:20:09 (after relog, entity 22): world Tollana, pos (211.045, 0.010, -547.644), on_navmesh true, is_on_ground true, navmesh_loaded true. XZ offset from gate (211.045, -547.644) is 0.00 m; Y 0.01 vs 0.606 (0.6 m, the 2 m clause holds). No `arrival_unrecoverable_off_mesh` row in the log; no player-sent-back row. Server position directly after the crossing (before the relog) has no bookmark; the relog bookmark and the on-gate position are the only position samples | - | - |
| State survives | PASS | G | After the crossing: bookmark 07:20:09 shows 1360 step 4038 status 1, nothing completed beyond 4037. DB (final): sgw_mission player 109 has 1360@4038 status 1 (also 622, 689); sgw_inventory has type 3730 x1 in container 2. No `letter_turn_in` row for player 109 (the only two are player 107 at 06:18Z) | - | - |
| Relog keeps it | PASS | G | 07:19:24.724 fire_player_loaded entity 22 Fxdltwo world Tollana saved_missions 3; bookmark 07:20:09 on the new entity: 1360@4038, world Tollana; DB as above | - | - |
| Client agrees | PASS (client-read only) | N1/N3 reads | Driver 3.7 and 3.8: Main 10644 x1, Mission 10668 x1 before and after relog; world 19 both reads. Client-read only, plus the DB agreement above | `L3-tollana-arrival` NEEDS_HUMAN | - |

LEG-3b overall: PASS. Combined LEG-3 on the sign-off sheet: `PARTIAL (A: <p1 grade>; B: PASS)`.

WARN/ERROR in the window: only the expected gmDHD gm_address_grant WARN. No findings.

## Not run
Haiku looker skipped by instruction. In-client Tollana checks (Copplemann within about 7 m, DHD 39 clickable, ring console reachable): not run, client-read only, NEEDS_HUMAN.

## Resume hint
None needed. Optional: a bookmark right after the crossing and before the relog would give a pre-relog position sample.

## Recipe
- Log: `<dr-lab worktree>\logs\server.log`, grep `SP-DL3-p2`, `gmDHD`, `Gate travel`, `fire_player_loaded`, `off_mesh`; the bookmark rows carry x/y/z, on_navmesh and missions.
- DB: `SELECT mission_id,current_step_id,status FROM public.sgw_mission WHERE player_id=109`; `SELECT item_id,type_id,stack_size,container_id FROM public.sgw_inventory WHERE character_id=109`.
- Design item id is `type_id`, not `item_id` (client sees item_id 10668).

NEEDS_HUMAN EVIDENCE FOR SIGN-OFF:
- lab_screenshot-20261109-011901.593.png (L3-tollana-arrival): driver took it at world 19 after the crossing, no description beyond the label. Looker skipped. Server shows player on navmesh at the Tollana gate.
