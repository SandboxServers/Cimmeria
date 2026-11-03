PACKET SP-DL2 VERIFIED 2026-11-03 by Sonnet; driver attempts: 1 (a1, instance p2, character Fxdltwo, player 109); server evidence window 14:26:19Z-14:28:30Z (bookmarks `SPU SP-DL2 leg2 start` 14:26:24.418Z and `leg2 end` 14:28:08.290Z)

Source: the server's own log file of the `dr-lab` run plus read-only `server_db_query`. Looker not run (the only screenshot clause is NEEDS_HUMAN). The brief said `p3`; the drive ran on p2, no effect on the server grade.

| Row | Grade | Tier (N1/N2/N3/G) | Server evidence (query or log event, time) | Screenshot | Issue |
|---|---|---|---|---|---|
| LEG-2 fixture applied | PARTIAL | G (stage_fixture) | Start bookmark: world Castle (466.365, 70.397, 991.466). Item type 3730 held. Mission 1360 NOT staged: `sgw_mission` for player 109 holds only 622@2113 and 689@68900, no 1360 (same refusal as SP-DL1, `mission 1360 was not written`, 14:22:57Z). | none | SP-DL1-1 (cited, not re-filed) |
| LEG-2 the dial | PASS | G (`/gmdhd 3` is the step) | `GM-gated cell method authorized` gmDHD 14:26:25.977Z; `gmDHD: dialing stargate` gate_addr 3 14:26:25.982Z; expected WARN `reason=gm_address_grant` 14:26:25.982Z (not a finding); `Gate travel: dial accepted, gate opens in 4s` to Harset 14:26:25.983Z; `Gate dial timer expired - opening gate` 14:26:26.167Z. | none | none |
| LEG-2 the crossing | PASS | G (`.gotolocation` to the gate region) | `triggerClientHintedGenericRegion` Castle.Stargate entering 14:26:34.083Z; `Stargate passed - crossing` Harset 14:26:34.083Z (one row); `fire_stargate_crossed: matched` destination Harset, actions 1, 14:26:34.084Z (one row); `Crossing hold elapsed` 14:26:35.676Z; RESET_ENTITIES for world Harset 14:26:35.684Z. Crossing fired on the first entry; 2.4b was not needed. | L2-harset-arrival: lab_screenshot-20261103-082732.004.png | none |
| LEG-2 chain 1366 firing / step advance 4037->4038 | BLOCKED | n/a | Fixture limitation: 1360 was never staged, so there was nothing to advance. No chain 1366 or `advance_step` row in the run for this window; `sgw_mission` has no 1360 row, before or after. Not FAIL. | none | SP-DL1-1 |
| LEG-2 destination | PASS | n/a | `leg2 end` bookmark: world Harset, space 65544, x -0.076, y -67.264, z 38.011, region `Harset.Stargate`. Target dais (-0.076, -67.274, 38.011): 0.010 m off. `player entered world` Harset start_cause travel 14:26:45.252Z. Client world_id 57 is Harset. `PersistPosition` Harset at logout 14:27:36.193Z carries the same coordinates. | none | none |
| LEG-2 on the navmesh | PASS | n/a | No `off_mesh` or `arrival_unrecoverable` row in the window; no WARN/ERROR for player 109 other than the expected `gm_address_grant`. `leg2 end` bookmark: navmesh_loaded true, on_navmesh true, is_on_ground true, ground_y -67.233 vs y -67.264 (0.03 m). Player stayed in Harset (not sent back to Castle). | none | none |
| LEG-2 item 3730 kept | PASS | n/a | `sgw_inventory` character_id 109 after the relog: item_id 10647, type_id 3730, container_id 2, stack_size 1 (10647 is the instance id the client shows as Frost's Letter; same item). Main item 10644 (type 4343) unchanged. | none | none |
| LEG-2 relog keeps it | PARTIAL | N1 (lab_logout, ensure_in_world) | `player session ended` logoff_character_select 14:27:36.191Z; relog: `player entered world` Harset start_cause login, entity 98, 14:27:47.764Z; `missions_restored` 622:2113,689:68900h (no 1360, fixture). Item 3730 survives (above). The 1360 half is untestable. | none | SP-DL1-1 |
| LEG-2 client agrees | PASS (client-read only) | N1 | Driver 2.6 and 2.7: Main 10644 x1 and Mission 10647 Frost's Letter x1, world_id 57 before and after relog; matches the server tables. | none | none |
| LEG-2 gate opening / crossing animation | NEEDS_HUMAN | n/a | Look-and-feel (unified-uat B19). Server side: `Stargate_MakeGate` (event 6100) sent 14:26:26.167Z and `Stargate_CrossGate` (6113) 14:26:34.084Z, each with 1 witness. | L2-harset-arrival: lab_screenshot-20261103-082732.004.png | none |
| LEG-2 overall | PARTIAL | G route (dial and entry are G by design) | Dial, one crossing, Harset arrival on the dais, navmesh, item 3730 and relog all held. The 1360 4037->4038 advance and its survival are BLOCKED by the fixture limitation, so PASS is not reachable. Not FAIL. | L2-harset-arrival: lab_screenshot-20261103-082732.004.png | SP-DL1-1 |

## Findings

- No new bug. The 1360 staging limitation is already filed as `results/issues/SP-DL1-1.md` (`persist_active_letter_mission` in `crates/base-methods/src/base/world_entry/methods/missions/letter_turn_in.rs` refuses a snapshot without active objective [4650]); reused, not duplicated.
- Observation, not a defect: the start bookmark was taken at the Castle landing point (region Castle.ArmoryRingDropZone), and the `.gotolocation` to the gate region came after the dial, as the brief intends. Gate opened 4 s after dial, entry 8 s later.

## Not run

- Chain 1366 (`advance_step 1360 '4038'`) and the 1360 step advance, plus "1360 survives the relog": BLOCKED, 1360 could not be staged (SP-DL1-1).
- Looker on `L2-harset-arrival`: not run; the clause is NEEDS_HUMAN and no server grade depends on it.
- Driver step 2.4b (second gate entry): not needed, the first entry crossed.

## Resume hint

Once the fixture can stage 1360 (active objective 4650 with step 4037, or a path that writes via the letter-mission persist), re-run LEG-2 with 1360@4037 and item 3730 in container 2 on world Castle. Before the dial, check that `sgw_mission` holds 1360 status 1 step 4037. Then `/gmdhd 3`, wait 8 s, `.gotolocation` to the Castle gate region, and verify one chain 1366 firing and 1360 at 4038 after the crossing and after the relog. Everything else in the leg is already proven.

## Recipe

1. Bookmarks: `Select-String` on `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab\logs\server.log` for `SPU SP-DL2` (note in field `note`; start 1793715984418, end 1793716088290).
2. Events: `Select-String` the `2026-11-03T14:2[678]` lines for `gmDHD|gm_address_grant|stargate|gate_travel|1366|off_mesh|player_loaded|"WARN"|"ERROR"`, excluding `h2::` and `rmcp`.
3. Missions: `SELECT * FROM public.sgw_mission WHERE player_id=109`. Inventory: `SELECT item_id, container_id, type_id, stack_size FROM public.sgw_inventory WHERE character_id=109` (keyed by `character_id`).

NEEDS_HUMAN EVIDENCE FOR SIGN-OFF:
- lab_screenshot-20261103-082732.004.png (L2-harset-arrival): driver took it after arrival at world_id 57 and noted no floor or fall problem; server says the player stands on ground in Harset at the gate dais (-0.076, -67.264, 38.011), region Harset.Stargate. Gate opening and crossing animation: judge by eye. Looker: not run.