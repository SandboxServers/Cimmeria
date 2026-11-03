# PACKET SP-DCB2-p2 VERIFIED 2026-11-03 by Sonnet; driver attempts: 1; server evidence window 08:37:33Z-08:41:57Z

Character Dcbch, player_id 100 (account 10), entity 50 before the relog and 51 after. Bookmarks: start 08:37:38.67Z (bookmark_id 1793695058673), end 08:41:37.22Z (1793695297223). Source: SigNoz service cimmeria-server, lab_run dr-lab.

| Row | Result | Tier | Server evidence | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| CB-U08b Cast | PASS | G | entity_query space 65602: spawn 520 id 102599 at (-271, 65.45, -135), spawn 521 id 102600 at (-254, 65.45, -135); both faction 1, ai Idle, velocity 0; spawnlist is_stationary true. 08:38:03.368-.373Z DEBUG `Spawned instance NPC from DB` spawn_id 520 and 521; 08:38:03.374Z INFO `Spawned instance NPCs` | none | n/a | `instance_populated` not searched. |
| CB-U08b Bark | PASS | - | no `Content: npc bark` row (chain 11010 / screen 200160) anywhere in 08:37:33-08:41:57Z, so none on re-entry, none after relog, none after step 2145 (mission 639 advance to 2145, chain 1031, 08:37:44.378Z) | none | n/a | Zero barks satisfies "at most one after the relog". The after-relog bark was not seen, so that sub-clause is a non-event, not a positive read. |
| CB-U08b WARN absent | PASS | - | no WARN or ERROR rows in the window; `screen_not_cached`, `empty_text`, `actor_not_player` not seen | none | n/a | Filtered by severity; also no lines matching those names. |
| CB-U08b Second client | PARTIAL (BLOCKED no packet tap) | - | witness_count 0 on both NPCs at query time; no second client | none | n/a | Needs server_packet_tap_start and a second client. |
| CB-U08b Setup: Frost (not graded) | server rows present | driver says N1/N2 | 08:39:49.391Z `interact: per-player dialog set` dialog_id 3995 entity 51; 08:39:49.392Z `granting item` chain 1003 item 3730; `advancing step` mission 622 step 80623 | - | n/a | End bookmark shows mission 622 at step 80623. |
| CB-U08b Setup: Guard body (not graded) | NO server rows | - | no `interact`, `granting item` 55, ability grants or chain 1005 rows after 08:39:49Z; no rows naming entity 102558 except avatar-update broadcasts to witness 51 | - | n/a | See findings and hint. |

SIGN-OFF SHEET (NEEDS_HUMAN): none. This part took no screenshot. H-CB-U08-1 (name plate `Op-CORE Soldier`) is carried to SP-DCB2-p1.

FINDINGS:
none filed. Setup failure at .15/.16/.18/.19 classified (c) lab/driver (unresolved), see below.

Guard body state at the .15 attempts (server): entity 102558 "Cellblock guard corpse", spawn 15, tag ArmYourself_GuardBody, template 21, alive in space 65602 at (-322.51, 73.47, -209.83), static, ai Idle. The end bookmark (08:41:37Z) records it 3.33 m from the player, caller_witnesses_it true, has_los_to_caller true, ability_ids [592]. avatar updates went to witness 51 (the player) every few seconds from 08:40:14Z to 08:41:56Z, so it was in AoI and sent to the client. Its interaction_flags is 0 on the entity (Frost's is also 0 after its dialog, so per-player dialog sets are not visible there; the player's offered_dialog_ids was [] at the end). No server row at all references a click, target or interact on 102558, so the attempts never reached the server. The seed binds dialog 3996 to this body with flag 0x40000000 (dialog_set_maps 5230, set 803); whether this player held set 803 after the relog cannot be read from the entity. The same sequence worked for the earlier Dcbch (player 99) run, so this is most likely a client targeting or hover problem (mouse_over 0, target not settable), but a product cause (the body not flagged interactable for this player) is not excluded.

NOT RUN / BLOCKED:
- Second client / AoI witness clause: no packet tap, no second client.
- Guard body setup (.15-.19): not graded; no server rows. Equip and reload (.18, .19) could not run since item 55 was never granted.
- .5 chat hook read: not run (not graded).

RESUME HINT: Setup only. Before retrying the Guard body, check with client_entity_find / client_hover_probe that 102558 is in the client entity table, stand 2-3 m from (-322.51, 73.47, -209.83), and confirm the server logs `interact` for 102558 (dialog 3996, chain 1005). If client_target still gives target 0, read the player's dialog sets for set 803 with server_db_query and file a product issue if absent.

GRADE COUNTS: PASS 3 (Cast, Bark, WARN absent), PARTIAL 1 (second client, BLOCKED no tap).
Ungraded setup: Frost done, Guard body not done (no server rows).
