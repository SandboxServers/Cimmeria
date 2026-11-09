Previous grade: PARTIAL (BLOCKED on SP-DL1-1, 2026-11-03)

PACKET SP-DL2 VERIFIED 2026-11-09 by Sonnet; driver attempts: 1 (a4 re-drive after #2068, instance p2, character Fxdltwo, player 109); server evidence window 06:12:50Z-06:15:35Z (bookmarks `SPU SP-DL2 leg2 start` 06:13:20.221Z and `leg2 end` 06:15:01.656Z)

Source: the `dr-lab` run's `logs/server.log` plus read-only `server_db_query`. No looker (the only screenshot clause is NEEDS_HUMAN). Driver-reported clock times (06:13:16-06:15:03) match the bookmarks; the driver gave positions in client units only, so position was graded from the server.

| Row | Grade | Tier (N1/N2/N3/G) | Server evidence (query or log event, time) | Screenshot | Issue |
|---|---|---|---|---|---|
| LEG-2 fixture applied | PASS | G (stage_fixture) | `lab fixture applied` player 109, world Castle, `missions: 1, items_granted: 1, moved: true` 06:12:52.376Z. `Restored missions from the saved journal` active `622:2113,689:68900h,1360:4037` 06:12:56.573Z, world Castle. Pre-crossing inventory (driver 2.1 and final table) holds 3730 in container 2. | none | none |
| LEG-2 the dial | PASS | G (`/gmdhd 3` is the step) | `GM-gated cell method authorized` gmDHD 06:13:21.742Z; `gmDHD: dialing stargate` gate_addr 3 06:13:21.742Z; `Gate travel: dial accepted, gate opens in 4s` Castle to Harset 06:13:21.742Z; `Gate dial timer expired - opening gate` 06:13:21.903Z; `Stargate_MakeGate` (6100) sent 06:13:21.904Z. No refusal. | none | none |
| LEG-2 the crossing | PASS | G (`.gotolocation` to the gate region) | `triggerClientHintedGenericRegion` Castle.Stargate entering 06:13:29.837Z; `Stargate passed - crossing` Harset 06:13:29.837Z (one row); `Stargate_CrossGate` (6113) 06:13:29.837Z; `fire_stargate_crossed: matched` destination Harset, actions 2, 06:13:29.838Z (one row). First entry crossed; no second entry needed. `Crossing hold elapsed` 06:13:31.395Z; RESET_ENTITIES for Harset 06:13:31.400Z. | L2-harset-arrival: lab_screenshot-20261109-001420.945.png | none |
| LEG-2 chain 1366 firing | PASS | n/a | `Content: advancing step` mission 1360, step 4038, chain_id 1366 ("1360 - Castle exit route: advance Frost...") 06:13:29.838Z, one row, after the dial and the crossing row, inside the window. | none | none |
| LEG-2 step advanced 4037 to 4038 | PASS | n/a | `mission_step_advanced` 1360 old_step 4037 to new_step 4038 ("Give Cpl. Frost's Letter to Col. Marsh.") 06:13:29.839Z, once (only one such row in the window). `sgw_mission` player 109 after the window: 1360 status 1, current_step 4038, completed_steps [4037], completed_objectives [4650], active [4651]. | none | none |
| LEG-2 destination | PASS | n/a | `player entered world` Harset 06:13:35.006Z; `PersistPosition` Harset (-0.076, -67.2636, 38.011) 06:14:27.464Z and again 06:15:33.758Z. `leg2 end` bookmark: world Harset, space 65544, region `Harset.Stargate`. Target dais (-0.076, -67.274, 38.011) per `stargates.sql:65`: 0.010 m off. Client world_id 57 (driver 2.4, 2.7) is Harset. | none | none |
| LEG-2 on the navmesh | PASS | n/a | No `off_mesh` or `arrival_unrecoverable` row anywhere in 06:12-06:16. `leg2 end` bookmark: navmesh_loaded true, on_navmesh true, is_on_ground true, ground_y -67.233 vs y -67.264 (0.03 m). Player stayed in Harset, not sent back to Castle. | none | none |
| LEG-2 item 3730 kept | PASS | n/a | `sgw_inventory` character_id 109 after the relog: item_id 10664, container_id 2, type_id 3730, stack_size 1. Main item 10644 (type 4343) unchanged. | none | none |
| LEG-2 relog keeps both | PASS | N1 (lab_logout, ensure_in_world) | `player session ended` logoff_character_select 06:14:27.454Z; relog `player entered world` Harset, entity 5, 06:14:41.099Z; `missions_restored` active `622:2113,689:68900h,1360:4038` 06:14:41.097Z. Item 3730 x1 container 2 survives (table above). Leg-end bookmark (entity 5) shows 1360 at step_id 4038. | none | none |
| LEG-2 client agrees | PASS (client-read only) | N1 | Driver 2.6 and 2.7: Frost's Letter x1 in the Mission container before and after the relog, chat "Mission Advance: Give Cpl. Frost's Letter to Col. Marsh.", tracker "Give Cpl. Frost's Letter to Col. Marsh.", world_id 57 before and after the relog. Matches the server tables. | none | none |
| LEG-2 gate opening / crossing animation | NEEDS_HUMAN | n/a | Look-and-feel (unified-uat B19). Server side: `Stargate_MakeGate` (6100) sent 06:13:21.904Z and `Stargate_CrossGate` (6113) 06:13:29.837Z, each with 1 witness. | L2-harset-arrival: lab_screenshot-20261109-001420.945.png | none |
| LEG-2 overall | PASS | G route (dial and entry are G by design) | Fixture, dial, one crossing, one chain 1366 firing, 1360 advanced once 4037 to 4038, Harset arrival 0.010 m from the dais on the navmesh, 3730 x1 kept, relog kept both. | L2-harset-arrival | none |

## Findings

- No WARN or ERROR rows in the window (excluding h2/rmcp transport noise). The brief expected a `gm_address_grant` WARN on the dial; none appeared this time. It is not a defect (the dial succeeded), but the character may already hold the Harset address, or the grant path no longer warns. Not investigated further.
- Fixes from #2068 hold: the old BLOCKED cause (SP-DL1-1, 1360 not staged) is gone; 1360@4037 was written and restored from the journal.
- Observation: the player id in the client changed 4 to 5 across the relog (entity id, as expected for a new session); no effect.
- No new product bug; no issue file written.

## Not run

- Looker on `L2-harset-arrival`: not run; the clause is NEEDS_HUMAN and no server grade depends on it.
- Driver step 2.4b (second gate entry): not needed.

## Resume hint

None for the server grade. Only the human look at the gate opening and crossing animation remains (sign-off sheet).

## Recipe

1. Bookmarks and events: `Select-String` on `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab\logs\server.log` for `SPU SP-DL2` (note in field `note`; start 1794204800221, end 1794204901655), then the `2026-11-09T06:1[2-5]` lines matching `lab fixture applied|gmDHD|gm_address_grant|stargate|Stargate|gate_travel|1366|mission_step_advanced|missions_restored|off_mesh|player entered world|PersistPosition|session ended|"level":"(WARN|ERROR)"`, excluding `h2::` and `rmcp`.
2. Missions: `SELECT * FROM public.sgw_mission WHERE player_id=109`. Inventory: `SELECT item_id, container_id, type_id, stack_size FROM public.sgw_inventory WHERE character_id=109` (keyed by `character_id`).
3. Driver recipe (for a repeat): stage the fixture (world Castle, 1360@4037, 3730 in container 2), ensure_in_world, `.bug SPU SP-DL2 leg2 start`, `/gmdhd 3`, `.gotolocation Castle 761.677 63.466 551.716`, wait ~8 s, screenshot, logout, ensure_in_world, `.bug SPU SP-DL2 leg2 end`.

NEEDS_HUMAN EVIDENCE FOR SIGN-OFF:
- lab_screenshot-20261109-001420.945.png (L2-harset-arrival): driver took it after arrival at world_id 57 and did not view it; server says the player stands on the ground in Harset at the dais (-0.076, -67.264, 38.011), region Harset.Stargate. Gate opening and crossing animation: judge by eye. Looker: not run.
