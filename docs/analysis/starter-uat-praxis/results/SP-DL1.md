PACKET SP-DL1 VERIFIED 2026-11-03 by Sonnet; driver attempts: 3 (a1, a2 fixture/brief failures; a3 ran the row on instance p2, not default); server evidence window 14:20:24Z-14:22:21Z (bookmarks `SPU SP-DL1 leg1 start` 14:20:29.919Z and `leg1 end` 14:22:01.157Z, player 108)

Source: the server's own log file of the `dr-lab` run plus read-only `server_db_query`. Lookers not run (the only screenshot clause is NEEDS_HUMAN and no server grade depends on it).

| Row | Grade | Tier (N1/N2/N3/G) | Server evidence (query or log event, time) | Screenshot | Issue |
|---|---|---|---|---|---|
| LEG-1 fixture applied | PARTIAL | G (stage_fixture) | Fixture held 688 at 80688 and item 3730 only. `lab fixture applied` 14:18:43Z (missions 1, items_granted 1, moved true). Mission 1360 NOT staged: `lab fixture not applied` ERROR, `mission 1360 was not written: stored None` at 14:16:55Z and 14:18:05Z. `missions_restored` 14:18:48Z active `622:2113,689:68900h,688:80688`, no 1360. Driver ran on p2, not default. | none | SP-DL1-1 |
| LEG-1 transfer on first press | PASS (server) | N1 per driver; driver verdict was `inconclusive / no_post_read` | One `fire_interact_tag` for `Cellblock_ArmoryRingSwitch` 14:20:38.425Z, entity 103703, then chain 1109 once: `Content: completing mission` 688 14:20:38.426Z, `cross-world teleporting entity` to Castle 14:20:38.428Z (count of those rows in the run: 1). Inside the driver window, 8.5 s after the start bookmark. | none | none |
| LEG-1 destination | PASS | n/a | Teleport row position [466.365, 70.397, 991.466], world Castle. `leg1 end` bookmark: world Castle, space 65537, x 466.36499, y 70.39709, z 991.46600 (0.000 m from the target). `triggerClientHintedGenericRegion` 62 `Castle.ArmoryRingDropZone` entering 14:20:44.39Z. | none | none |
| LEG-1 on the navmesh | PASS | n/a | No off_mesh or arrival_unrecoverable row; no WARN/ERROR for player 108 from 14:20:24Z to 14:22:21Z. `leg1 end` bookmark: navmesh_loaded true, is_on_ground true, ground_y 70.559 vs y 70.397 (0.16 m), speed 0. Client position (99146.6, 46636.5, 7039.71) is the server position in client units, as in the driver note. | none | none |
| LEG-1 688 completed once | PASS | n/a | `Mission completed directly` 688 `Secure the Armory` 14:20:38.427Z, one row. `sgw_mission` 688: status 2, repeats 1, current_step null, completed steps [2356, 80688]. 80688 was active before the press (`missions_restored` 14:18:48Z). No second completion row. | none | none |
| LEG-1 state survives: 1360 at 4037 | BLOCKED | n/a | Fixture limitation: 1360 was never staged (see row 1), so there was nothing to survive. `sgw_mission` for player 108 has no 1360 row. Not FAIL. | none | SP-DL1-1 |
| LEG-1 state survives: item 3730 x1 in container 2 | PASS | n/a | `sgw_inventory` character_id 108: item_id 10639, type_id 3730, container_id 2, stack_size 1. Item 3730 is the type id; 10639 is the instance id the client shows (Frost's Letter): the same item. Main item 10635 (type 4343) unchanged. | none | none |
| LEG-1 state survives: appearance intact | NEEDS_HUMAN | n/a | Look-and-feel; client-read only. | L1-arrival-castle: lab_screenshot-20261103-082126.294.png | none |
| LEG-1 relog keeps it | PASS | N1 (lab_logout, ensure_in_world) | One load of Castle after the relog: `fire_player_loaded` entity 95 world Castle 14:21:40.998Z (the first Castle load, entity 94, was the transfer itself at 14:20:42.88Z). No second chain 1109 firing, no second cross-world row, `PersistPosition` Castle at 14:21:30Z and 14:22:38Z both at the landing point. 688 still status 2 / repeats 1; item 10639 still container 2. 1360 not testable. | none | none |
| LEG-1 client agrees | PASS (client-read only) | N1 | Driver 1.7 and 1.9 show Main 10635 and Mission 10639 unchanged, and the chat line `Mission Complete: Secure the Armory`, matching the server tables. | none | none |
| LEG-1 overall | PARTIAL | N1 click; the other steps N1 | Transfer, destination, navmesh, single completion, item survival and relog all held. The 1360 clause could not run (fixture). Per the brief, PASS needs 1360 at 4037, so the row is PARTIAL, not FAIL. | L1-before-press: lab_screenshot-20261103-082032.789.png | SP-DL1-1 |

## Findings

- Mission 1360 cannot be staged by `server_stage_fixture` with only `{id:1360, step:4037}`. Cause (code read): the generic mission update for 1360 goes to `persist_active_letter_mission` in `crates/base-methods/src/base/world_entry/methods/missions/letter_turn_in.rs`, which accepts the active snapshot only if `active_objective_ids == [4650]` (and no completed steps or objectives, repeats 0). The fixture writes the step with empty objective lists, so the guard returns false, the log says `invalid_or_stale_snapshot`, and the fixture check reports `stored None`. The guard is deliberate (it stops stale snapshots overwriting the letter mission), so this is a harness/fixture defect, not a product bug in the game path. It is real enough to file: it blocks every UAT that needs 1360 at 4037 (this row, clause 3). Also seen for player 109 at 14:22:57Z (same error). Issue body: `results/issues/SP-DL1-1.md`.
- The brief said instance `default`; the drive ran on p2. No effect on the server grade.
- The driver's click verdict was `inconclusive` (`no_post_read`) but the server shows exactly one switch interaction at 14:20:38Z; graded on the server.
- `fire_player_loaded` for the transfer arrival matched `actions:1`; nothing unexpected. No WARN/ERROR in the window for player 108.

## Not run

- Clause 3 of the row, mission 1360 advance 4037 (and "1360 at 4037 survives" in the state and relog clauses): BLOCKED, the fixture could not stage 1360 (SP-DL1-1).
- Lookers on `L1-before-press` and `L1-arrival-castle`: not run; the appearance clause is NEEDS_HUMAN and no server grade depends on them.

## Resume hint

Once the fixture can stage 1360 (active objective 4650 with step 4037, or a stage path that writes via the letter-mission persist), re-run LEG-1 with `missions: [688@80688, 1360@4037 (objective 4650)]` and item 3730 in container `mission`. Verify before the press that `sgw_mission` holds 1360 status 1 step 4037 and a log row `lab fixture applied`. The ring press, transfer, 688 completion and item survival are already proven; only the 1360 survival (before press, after transfer, after relog) is left. Run it on the instance the brief names, or update the brief to p2. Look up the switch by tag (server_entity_query tag `Cellblock_ArmoryRingSwitch` gives entity 103703), not by client name.

## Recipe

1. Bookmarks: `Select-String` on the `dr-lab` `logs\server.log` for `SPU SP-DL1` (note in field `note`; start 1793715629919, end 1793715721157).
2. Events: `Select-String` the 14:20-14:22Z lines for `cross_world`, `Cellblock_ArmoryRingSwitch`, `completing mission`, `ArmoryRingDropZone`, `fire_player_loaded`, `off_mesh`, and WARN/ERROR excluding `h2::` and `rmcp`.
3. Missions: `server_db_query` `SELECT * FROM public.sgw_mission WHERE player_id=108`. Inventory: `SELECT item_id,type_id,container_id,slot_id,stack_size FROM public.sgw_inventory WHERE character_id=108` (the table is `sgw_inventory`, keyed by `character_id`; there is no `sgw_inventory_item`).
4. 1360 refusal: `crates/base-methods/src/base/world_entry/methods/missions/mod.rs` (line ~181) and `letter_turn_in.rs` (`persist_active_letter_mission`).

NEEDS_HUMAN EVIDENCE FOR SIGN-OFF:
- lab_screenshot-20261103-082126.294.png (L1-arrival-castle): driver took it after the transfer, before the relog; server says the player stands on ground at Castle (466.365, 70.397, 991.466). Appearance after transfer: judge by eye. Looker: not run.
- lab_screenshot-20261103-082032.789.png (L1-before-press): context shot only. Looker: not run.