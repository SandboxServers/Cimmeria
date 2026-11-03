# PACKET SP-DCB2-p2 VERIFIED 2026-11-03 by Sonnet; driver attempts: 1; server evidence window 09:35:29Z-09:39:25Z

Character Dcbch (NEW, created 2026-11-03 ~09:29Z), player_id 101 (account 10), entity 55 before the relog and 56 after. Bookmarks: start 09:35:34.32Z (bookmark_id 1793698534318), end 09:39:04.97Z (1793698744967). Source: SigNoz service cimmeria-server, lab_run dr-lab, plus a live `server_entity_query` of space 65607.

| Row | Result | Tier | Server evidence | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| CB-U08b Cast | PASS | G | entity_query space 65607: spawn 520 id 102844 at (-271, 65.45, -135), tag Cellblock_Ambient_Prisoner1; spawn 521 id 102845 at (-254, 65.45, -135), tag Cellblock_Ambient_Prisoner2. Both faction 1, ai Idle, velocity 0. 09:36:12.128Z DEBUG `Spawned instance NPC from DB` spawn_id 520 and 521; 09:36:12.129Z INFO `Spawned instance NPCs` (once, at the relog world load). | none | n/a | `instance_populated` not seen. Template 176 `Castle_OpCoreSoldier_Unarmed`. |
| CB-U08b Bark | PASS | - | No `Content: npc bark` row (chain 11010 / screen 200160) in 09:35:29-09:39:25Z, so none on re-entry (.1/.2, .6/.7), none after the relog, none after step 2145 (mission 639 `advancing step` to 2145, chain 1031, 09:35:44.319Z). Start and end bookmark recent_events hold no bark in the window. | none | n/a | Zero barks satisfies "at most one more after the relog". The after-relog bark was not seen, so that sub-clause is a non-event, not a positive read. |
| CB-U08b WARN absent | PASS | - | `severity_text IN (WARN, ERROR)` over 09:35:20-09:39:50Z, service cimmeria-server, lab_run dr-lab: 0 rows. | none | n/a | `screen_not_cached`, `empty_text`, `actor_not_player` not seen. |
| CB-U08b Second client | PARTIAL (BLOCKED no packet tap) | - | witness_count 0 on both NPCs at query time; no second client. | none | n/a | Needs `server_packet_tap_start` and a second client. |
| CB-U08b Setup .12 Frost (not graded) | server rows present | driver N1/N2 | 09:37:50.763Z `interact` entity 56; `interact: per-player dialog set` dialog 3995; chain 1003 `advancing step` mission 622 step 80623; `Content: granting item` item 3730 (Frost's Letter, MISSION container, slot 0); chain 1003 `adding dialog set` 3996; chain 1121 accepting mission 1360 (already held). | - | n/a | Item and step match the row. No loot window, as designed. |
| CB-U08b Setup .15 Guard body native (not graded) | BLOCKED (harness) | N1 | No `interact`, no dialog 3996 display, no chain 1005 row after 09:37:50Z. Bookmark end: last_interaction_target 102806 (Frost), target_id 102806, offered_dialog_ids []. | `client_world_click` observe failed, target_after 102806 | n/a | Lab harness gap: synthetic hover/click cannot pick this entity; per the owner the product path works for a human player. Issue file reused: `results/issues/SP-DCB2-p2-1.md`. Not duplicated. |
| CB-U08b Setup .15G fallback (not graded) | PASS | G | 09:38:39.880Z INFO `gmGiveItem: granting item to GM` item 55; `Item granted to player` item 55 MAIN slot 1 (SI 3 9mm Pistol). `GmGrantAbility: persisted` (persisted true) plus `GmAbilityGranted: cell mirrored`: 592 Pistol Shot 09:38:41.383Z, 594 Strike 09:38:42.892Z, 597 Heal Focus 09:38:44.424Z, 1218 Medical Attention: Recuperation 09:38:45.932Z. | - | n/a | Tier G only; never turns .15 into PASS. Ability 596 (Reload) not granted, as expected. |
| CB-U08b Setup .16 dialog 3996 (not graded) | BLOCKED (follows .15) | - | No dialog 3996 display or chain 1005 row. | DIALOG `none` | n/a | The driver's PASS (G) was a skipped-path claim; no server row backs it. |
| CB-U08b Setup .17/.18 equip (not graded) | server rows present | N1 (.18) | 09:38:52.137Z `fire_item_equipped: no chains matched` item 55; 09:38:52.138Z `weapon_ability_swap` added [579, 708], slot 0; `SyncBandolierItems: equip-display decision` drew_weapon true, active_bandolier_slot 0; active weapon visual `WP-Human.WP_Pistol_1A`. | - | n/a | Pistol in bandolier slot 1 (active slot 0). |
| CB-U08b Setup .19 reload (not graded) | server rows present | N1 (key posted) | 09:38:56.275Z DEBUG `requestReload` entity 56; 09:38:56.282Z INFO `Weapon reload started` (cooldown 1, warmup 2, target 15). Then holster tick `Item_Unequip` for item 55 at 09:39:06Z (after the end bookmark). | - | n/a | The server did receive the R key. The visual clause stays NEEDS_HUMAN. |

SIGN-OFF SHEET (NEEDS_HUMAN):
- H-19-1, CB-U08b.19: R-key reload effect (animation and ammo) not visually checked. No screenshot saved (driver reports none; path: none). Server shows `requestReload` and `Weapon reload started`. Looker: none (no screenshot).
- H-CB-U08-1 (name plate `Op-CORE Soldier` above the two corridor NPCs): this part takes no screenshot; carried to SP-DCB2-p1.

FINDINGS:
1. CB-U08b.15 native Guard-body click (BLOCKED (harness)): reproduces the known lab harness gap (synthetic hover/click cannot pick template 21); not a product defect and not an owner decision. No new issue. Existing body: `docs/analysis/starter-uat-praxis/results/issues/SP-DCB2-p2-1.md` (reused, entity ids there are from an earlier run). This run adds: guard body id 102803, `target_after` stayed 102806 (Frost), no interact row.
2. Note, not a finding: the driver's `.16` PASS and the batch `.1`/`.2` parallel send were driver-side slips. They did not change the server result.

NOT RUN / BLOCKED:
- Second client / AoI witness clause: no packet tap, no second client.
- .9 packet tap: BLOCKED by design.
- .13 loot window: NOT-RUN by design.
- .15 / .16 native Guard body: BLOCKED (harness) (issue SP-DCB2-p2-1).

RESUME HINT: Nothing to redo for graded rows. The native .15 waits for a harness route that can pick the Guard body (no owner decision involved). Optional: a second client plus `server_packet_tap_start` for the AoI clause.

GRADE COUNTS: PASS 3 (Cast, Bark, WARN absent), PARTIAL 1 (second client), BLOCKED (harness) 1 (.15 native), setup .15G PASS (tier G).

CHARACTER STATE AFTER: Dcbch entity 56, player 101, Castle_Cellblock at (-324.8, 73.47, -212.25). Mission 622 active at step 80623 (objective 90623); mission 639 at step 2145; mission 1360 at step 4037; mission 638 complete; mission 689 complete (hidden). Items: Frost's Letter (3730, MISSION), SI 3 9mm Pistol (55) in bandolier slot 1. Granted abilities: 592, 594, 597, 1218 (plus 579, 708 from the weapon). Ability 596 not granted.
