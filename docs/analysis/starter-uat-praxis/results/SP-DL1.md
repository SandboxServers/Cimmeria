Previous grade: PARTIAL (BLOCKED on SP-DL1-1, 2026-11-03)

PACKET SP-DL1 VERIFIED 2026-11-09T06:13Z by Sonnet; driver attempts: 1 (re-drive a4, instance p2, character Fxdlone, player_id 108); server evidence window 06:10:08Z-06:12:13Z (bookmarks `SPU SP-DL1 leg1 start` 06:10:13.147Z, `leg1 end` 06:11:52.836Z; fixture 06:07:41Z)

| Row | Grade | Tier (N1/N2/N3/G) | Server evidence (query or log event, time) | Screenshot | Issue |
|---|---|---|---|---|---|
| LEG-1 Fixture applied | PASS | G (fixture) | `lab fixture applied` 06:07:41.129Z (missions 2, items_granted 1, moved true); `missions_restored` 06:07:47.405Z world Castle_CellBlock active `622:2113,689:68900h,1360:4037,688:80688`, completed_count 0; item 3730 in container 2 (inventory row item_id 10662, type_id 3730); bookmark start world Castle_CellBlock (world_id 12 per interaction_pose row) | - | none |
| LEG-1 Transfer on first press | PASS | N1 (world_click, native_camera; the driver's own read was `inconclusive`, the server row is the evidence) | `fire_interact_tag: matched` tag `Cellblock_ArmoryRingSwitch` actions 3, 06:10:22.855Z (9 s after the start bookmark, inside the press window); `Content: completing mission` 688 chain 1109 06:10:22.855Z then `cross-world teleporting` chain 1109 06:10:22.857Z; exactly one firing (2 chain-1109 rows: one completion, one teleport), none before or after; `interaction_pose` route `chain_tag` target 100595 | L1-before-press (not graded) | none |
| LEG-1 Destination | PASS | server | `cross-world teleporting` world Castle position [466.365, 70.397, 991.466] (exact); old space 65555 `Destroyed instanced space`; new space 65537; `triggerClientHintedGenericRegion` region 62 `Castle.ArmoryRingDropZone` entering 06:10:26.221Z; end bookmark x 466.365 y 70.397 z 991.466 world Castle | - | none |
| LEG-1 On the navmesh | PASS | server | no `off_mesh`, `unrecoverable` or fall row for player 108 in 06:10-06:12; end bookmark navmesh_loaded true, y 70.397 (target 70.4), `PersistPosition` 06:11:20Z same x/y/z. Client-read: driver 1.5 world_id 8 matches the server | - | none |
| LEG-1 688 completed | PASS | server | before: 688 active at 80688 (06:07:47Z). `Mission completed directly` event `mission_completed` 688 06:10:22.856Z, exactly one row. DB after: `sgw_mission` 688 status 2, repeats 1, completed_step_ids [2356, 80688]. No second completion row, no reward row | - | none |
| LEG-1 State survives | PASS (appearance clause NEEDS_HUMAN) | server | DB after window: 1360 status 1 current_step 4037 (the ring does not advance 1360, as specified); 622 at 2113, 689 at 68900 intact; `sgw_inventory` character_id 108: type 3730 x1 in container 2 (item_id 10662), Main container 1 holds 10635 (type 4343) x1; `sgw_player` world_id 8. No appearance before/after snapshot exists, see NEEDS_HUMAN | L1-arrival-castle | none |
| LEG-1 Relog keeps it | PASS | server (driver `lab_logout` + `lab_ensure_in_world`) | `session.start` entity 3 start_cause `login` 06:11:31.908Z, world Castle, one load; `missions_restored` 06:11:31.906Z active `622:2113,689:68900h,1360:4037`, completed_count 1; no second chain 1109 firing and no second `mission_completed` after 06:10:22Z; DB tables above read after the relog | - | none |
| LEG-1 Client agrees | PASS (client-read only) | client-read | Driver 1.7/1.9: Main 10635 x1, Mission 10662 x1 matches DB item_ids 10635 and 10662 (type 3730 shows as client instance id 10662). Chat tail `Mission Complete: Secure the Armory` matches the 688 completion row. Tracker still lists "Arm Yourself!" and "Frost's Letter" lines, which belong to 622/1360 and are consistent with those missions staying active | - | none |

**Row LEG-1 overall: PASS.** Fixture held (688 at 80688, 1360 at 4037, 3730 x1 in container 2), one chain 1109 firing, one cross-world travel to Castle landing exactly on the platform with the drop-zone region firing and no off-mesh row, 688 completed once, 1360 at 4037 and 3730 x1 survived, and the relog read matched. The press was an N1 world click (driver `tier: N1 native_camera`); the server confirms it caused the interact (route `chain_tag`). Appearance is NEEDS_HUMAN and does not change this grade.

Findings (WARN/ERROR in scope in the window): none for player 108 between 06:10 and 06:12.

Notes on the old BLOCKED cause: the old run was blocked because the fixture did not stage 1360. This run staged 1360@4037, so the 1360 clauses are graded here and held. The ring did not advance 1360; the Castle crossing that does is SP-DL2.

## Not run

- Appearance intact after the transfer: no before snapshot and driver 1.6 did not assess the floor or model (NEEDS_HUMAN below).
- Lookers skipped by instruction; screenshots are NEEDS_HUMAN.
- The ring animation is not a clause (needs the client patch).

## Resume hint

None required for the server clauses. For the appearance clause, a person reviews `L1-arrival-castle`. A later driver should read 1.6 properly (floor, model) and record `sgw_player` appearance columns before the press.

## Recipe

1. Ids: lab-server `server_db_query` `SELECT player_id, world_id FROM public.sgw_player WHERE player_name ILIKE '%Fxdlone%'` gives 108.
2. Bookmarks and chain rows from the server log (SigNoz not needed):
   `Select-String -Path <dr-lab>\logs\server.log -Pattern 'SPU SP-DL1'` for the two bookmarks; then
   `Select-String ... -Pattern '"timestamp":"2026-11-09T06:1[0-2]'` filtered with `-match 'chain_id":1109|cross-world|ArmoryRingDropZone|off_mesh|unrecoverable|missions_restored|session.start|fire_interact_tag'`.
3. Fixture before the press: log lines `lab fixture applied` and `missions_restored` at 06:07:4x.
4. After state: `SELECT mission_id, current_step_id, status, completed_step_ids, repeats FROM public.sgw_mission WHERE player_id=108`; `SELECT * FROM public.sgw_inventory WHERE character_id=108`; `SELECT player_id, world_id FROM public.sgw_player WHERE player_id=108`. Mission status 2 = completed, 1 = active.
5. Item 3730 appears as item_id 10662 in the DB and the client; compare by type_id.

NEEDS_HUMAN EVIDENCE FOR SIGN-OFF:
- `.claude\reports\starter-uat-praxis\` shot `lab_screenshot-20261109-001114.352.png` (L1-arrival-castle, taken 06:11:14Z after landing in Castle): clause "appearance after the transfer is intact" and the look of the landing (standing on a floor, normal model, no error window). Driver gave no description (1.6: "floor/falling and model appearance not assessed"). Looker: skipped by instruction.
- `lab_screenshot-20261109-001015.189.png` (L1-before-press, 06:10:15Z): context only, not a clause. Looker: skipped.
