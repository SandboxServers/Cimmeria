PACKET SP-DCB2-p2 MERGED FINAL GRADE 2026-11-03 by Sonnet; driver attempts: 2 (attempt 1 history kept below; attempt 2 section at the end supersedes rows .11-.16); server evidence windows 07:46:21Z-07:49:26Z (attempt 1) and 07:51:45Z-07:53:20Z (attempt 2); attempt-1 header: (bookmarks CB-U08b start 1793691981773, end 1793692166451; SigNoz, lab_run dr-lab, character Dcbch player_id 99)

| Row | Result | Tier | Server evidence (time, scope, event, key fields) | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| CB-U08b Cast (spawns 520/521) | PASS | G (setup) | 07:46:46.55Z DEBUG `Spawned instance NPC from DB` npc 102452 spawn 520 and 102453 spawn 521, template 176 "Op-CORE Soldier", space 65599 (post-relog space; start bookmark was space 65598). `server_entity_query` now: 102452 at (-271.0, 65.45, -135.0), 102453 at (-254.0, 65.45, -135.0), ai_state Idle, velocity 0, faction 1. INFO row is `Spawned instance NPCs` npc_count 49, not `instance_populated`. | not needed | not run (no screenshots) | Placement, stationary and spawn rows hold. "Friendly" is only faction 1 / alignment 0, not checked against a hostility table. |
| CB-U08b Bark (chain 11010) | PARTIAL | N3 (read of chat cursor) | No `Content: npc bark` row from any chain between 07:46:00Z and 07:49:30Z. So: none on re-entry and none after the relog hold. The "once for the crossing player" bark happened in p1 or earlier, outside this window, and was not re-checked. | driver cursor d2 showed no bark lines | n/a | Clauses "none on re-entry", "at most one after relog", "none after step 2145" hold; "exactly once at crossing" is not graded here. |
| CB-U08b WARN absent | PASS | n/a | SigNoz WARN/ERROR plus bodies `screen_not_cached`, `empty_text`, `actor_not_player` in 07:46:00Z-07:49:30Z (lab_run dr-lab): 0 rows. | n/a | n/a | Query had `fields:["body"]`; zero rows is the result. |
| CB-U08b Second client / AoI | PARTIAL | n/a | Only one session (Dcbch, entity 48). Witness count of both NPCs 0 (player far away). No second client, no packet tap. | BLOCKED: no packet tap (brief says so) | n/a | The AoI clause was never exercised. |
| CB-U08b.1-.3 GOTO x2, read cursor | PASS (setup) | G / N3 | Start bookmark at (-265, 65.45, -135), space 65598. | driver | n/a | Not graded. |
| CB-U08b.4 Relog | PASS (setup) | N3 | `world_enter` Castle_CellBlock at 07:46:53.6Z (end bookmark `recent_events`), new space 65599, 49 NPCs respawned 07:46:46Z; restore chains 1006 (Frost dialog, step 2113) and 1026 (Prisoner 329) ran 07:46:53.64Z. | driver | n/a | Driver quoted 07:47:38 for the logout; server shows 07:46:46-53. The driver clock is off by roughly a minute. Not graded. |
| CB-U08b.9 Search Frost | PASS (setup) | N1 target + N2 `/interact` | 07:48:01.659Z `interact: per-player dialog set` dialog 3995 target 102414 (Frost); 07:48:01.660Z chain 1003 `advancing step` mission 622 step 80623 "Search the NID Guard's body for a weapon."; `adding dialog set` map 5230 dialog 3996 template 21 flags INT_MissionWorldObject for entity 48; 07:48:01.661Z `accepting mission` 1360 (chain 1121). Inventory has type 3730 (Frost's Letter) in container 2. | driver tap | n/a | Server state is exactly what the row expects. Not graded, setup. |
| CB-U08b.10 LOOT_ALL on Frost | FAIL-case (c), not graded | n/a | Chain 1003 grants the letter directly (add_item 3730); no loot window is opened by this chain. | `loot_all` error "loot window is not open" x2 | n/a | Brief defect: step expects a Frost loot window that the seed does not open. Remove the step. |
| CB-U08b.11 RCLICK Guard body | FAIL-case (c), not graded | N1 attempted | Guard body entity 102411 (spawn 15, tag ArmYourself_GuardBody, template 21 "Cellblock guard corpse") is at (-322.51, 73.47, -209.83); global `interaction_type_flags` 0. Server did bind the per-player dialog 3996 via map 5230 at 07:48:01.660Z with INT_MissionWorldObject, and mission 622 was at step 80623 (end bookmark). No `interact` row for 102411 and no dialog 3996 display: the client never sent an interact (setTargetID TargetId 0). No WARN/ERROR. | client_target mouse_over 102411 but target_after 0; world_click went to own avatar | n/a | The server had the body interactable (see below). The failure is a client-side targeting problem, not a product bug. |
| CB-U08b.12-.15 dialog close, INV, EQUIP, Reload | BLOCKED (setup, not graded) | n/a | Chain 1005 never fired, so no `granting item` 55 and no grant of 592/594/597/1218. `sgw_inventory` for character 99: 6 rows (types 3440, 3437, 3438, 3497, 4343, 3730); no 55. | Main only item 10572 "Accessory" | n/a | Consequence of .11. |
| CB-U08b.16 End bookmark | PASS (setup) | n/a | Bookmark `SPU SP-DCB2-p2 CB-U08b end` at 07:49:26.45Z, mission 622 step 80623 (completed_steps [2113]), 638 step 2114, 1360 step 4037. | n/a | n/a | |

SIGN-OFF SHEET (NEEDS_HUMAN):
- H-CB-U08-1 (name plate "Op-CORE Soldier" above the two NPCs): no screenshot taken by the driver. Server name for template 176 is "Op-CORE Soldier". Open for a human look.

FINDINGS:
none. No product bug (case a or b). No issue bodies written.
- Case (c) only: CB-U08b.10 and CB-U08b.11, see NOT RUN / BLOCKED.

NOT RUN / BLOCKED:
- CB-U08b second-client AoI clause: BLOCKED, no packet tap and no second client. Needs `server_packet_tap_start` (verifier cannot start a tap while the driver is not live) plus a second character near the NPCs, or the AoI clause moves to a packet.
- CB-U08b.5 (chat read after relog): NOT-RUN. Chat hook was reinstalled after the world load.
- CB-U08b.10: brief defect (c). Frost gives no loot window; chain 1003 adds item 3730 and the step advance only. The step must not be in the next brief.
- CB-U08b.11-.15: driver/harness failure (c), not a product bug.
  - Was the Guard body interactable? Yes at the server. Mission 622 was at step 80623 (completed step 2113 and objective 2452) and chain 1003 added dialog set 5230 (dialog 3996, template 21) with INT_MissionWorldObject. The entity itself shows `interaction_type_flags` 0 because the flags are per player.
  - Pistol item 55 (chain 1005) conditions: `step_status` mission 622 step 80623 = active; archetype neq 5, 6, 7. Dcbch is Commando, so this passes. Nothing depends on mission 639, 638 or 689, so the p1 outcome (638 accepted at step 2114, 639 never done) is not a brief defect and does not block the pistol. Trigger: dialog_open 3996, scope player.
  - Likely cause: a corpse model with no mouse-over/target through the client's pick (mouse_over was 102411 once, but a left-click target never held; the world_click hit the own avatar). p4 of SP-DCB1 reached the same state and passed through `client_world_click`, so the camera/standing position here differs.
- CB-U08b.13-.15: BLOCKED by .11, not graded.

RESUME HINT:
- Re-run only the tail: state is carried (622 step 80623, dialog set 5230 bound, no item 55). Do not replay .1-.9; do not repeat the Frost search (chain 1003 has no `once`, but it would not re-run: step is no longer 2113).
- Drop CB-U08b.10 (Frost loot window does not exist).
- For .11, GOTO the SP-DCB1-p4 position `-325.5 73.47 -213.0` (already there), then follow verify-click-recipe.md: `client_entity_find` for the body, `client_camera` aimed at (-322.51, 73.47, -209.83) at about 4.4 m, then `client_world_click` right-click with the camera not rotated; if mouse_over is not 102411 move to 2-3 m and retry once. Fallback if the click still misses: `client_target` 102411 (check `target_after` is 102411) then `/interact`. A fallback is N2 (NATIVE_SHORTFALL at best).
- Confirm on the server: `interact: per-player dialog set` dialog 3996 target 102411, `Content: granting item` item 55, chain 1005; then .12-.16 (inventory, equip, reload).

---

## ATTEMPT 2 (merged final grade, server window 07:51:45Z-07:53:20Z, SigNoz lab_run dr-lab, Dcbch player_id 99)

| Row | Result | Tier | Server evidence | Notes |
|---|---|---|---|---|
| CB-U08b Cast / WARN absent / Bark / AoI | unchanged from attempt 1 (PASS / PASS / PARTIAL / PARTIAL) | | | Not re-driven. AoI stays PARTIAL (no packet tap, no second client). |
| .10 LOOT_ALL on Frost | NOT-RUN, brief defect | n/a | Chain 1003 grants item 3730 directly; no loot window. | Remove the step from the brief. Not graded. |
| .11 RCLICK Guard body | PASS | N1 (client_world_click right-click, camera pitch -10, 3.33 m) | 07:52:36.518Z INFO `interact` entity 48 target 102411 heading_age_ms 1594; `fire_interact_tag/template: no chains matched` (expected, dialog set is per player); 07:52:36.522Z `interact: per-player dialog set -> onDialogDisplay` dialog 3996 "Search the Guard's Corpse"; 07:52:36.523Z chain 1005 `set interaction type` (clears INT_AStoryMissionPending) and `Content: granting item` item 55 "SI 3 9mm Pistol" count 1; `interaction.pose` radius 3.33 m, route fallback, space 65599. | Attempt A at 2.58 m produced no interact (client side); attempt B reached the server. |
| .12 Dialog close | PASS (client op "closed"; server needs no row) | N1 | No WARN/ERROR in window. | Weak: no server close row; nothing contradicts it. |
| .13 INV read | PASS | N3 read | Final `sgw_inventory` char 99: item 10574 type 55 in container 3; item 10572 type 4343 container 1. | Driver's Main slot 2 before the move is consistent with the grant. |
| .14 EQUIP pistol to Bandolier | PASS | N1 (drag) | `sgw_inventory` item 10574 (pistol, ammo_types Bullet_Default..) now container 3 (not the main bag container 1), durability 100. | Container 3 is the bandolier per the driver's slot readout; DB slot 0 is 0-based vs the driver's slot 1. |
| .15 Reload ability | FAIL as written; classified brief defect (c), no issue | N2/N3 attempt | `sgw_player_ability_grants` for 99: only 592, 594, 597, 1218 (source chain 1005, tutorial 622, 07:52:36.53Z). `resources.abilities` 596 "Reload" exists (flags 1, training_cost 1, warmup 2) but chain 1005 grants only [592,594,597,1218]; no content action, ability set or archetype tree row references 596. The client's `onKnownAbilitiesUpdate` [594,597,1218,592] matches. | Reload is not given by the tutorial; it is a trainer-learned ability (training_cost 1), so a character straight out of this tutorial does not know it. The brief expectation is wrong. Fix the brief (drop .15 or teach 596 first). Whether 596 should be in the tutorial grant list is a design question, not evidenced as a regression; no issue body written. |
| .16 End bookmark | PASS (setup) | | Driver last UTC 07:52:54. | |

Findings: none; no issue bodies written (no product bug). WARN/ERROR in cimmeria-server 07:51:45Z-07:53:20Z: 0 rows.

FINAL PER-ROW (merged): Cast PASS; WARN absent PASS; Bark PARTIAL (once-at-crossing never re-checked); AoI PARTIAL; .1-.9 PASS (setup); .10 NOT-RUN (brief defect); .11 PASS (N1); .12 PASS; .13 PASS; .14 PASS; .15 brief defect (Reload not granted, ability 596 is trainer-learned); .16 PASS.

Sign-off sheet: H-CB-U08-1 (name plate "Op-CORE Soldier") still open, no screenshot taken.
