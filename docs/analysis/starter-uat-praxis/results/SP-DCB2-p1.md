PACKET SP-DCB2-p1 VERIFIED 2026-11-03 by Sonnet; driver attempts: 1; server evidence window 09:29:29Z-09:32:38Z (player_id 101, entity 55, space 65606, log file dr-lab server.log)

| Row | Result | Tier | Server evidence (time, scope, event, key fields) | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| S1/S2 | PASS | N1 | Start bookmark 09:29:34Z, player 101 Dcbch, entity 55, at (-334.23, 73.47, -228.03) | in_world | - | fresh character |
| CB-U08a.1 BOOT | PASS (minigame result UNVERIFIED) | N1 | Prison boot unlock line per driver (seq 511854); not independently re-read | MinigameWin opened | - | no server-side contradiction |
| CB-U08a.2 Region2 crossing | PASS | N2 | 09:30:09.589 fire_enter_region matched Castle_Cellblock.Region2; 09:30:09.590 accepting mission 638 chain 1011; Mission accepted step 2114; 09:30:09.592 adding dialog set "Free Prisoner 329"; all before first interact (09:30:48) | GOTO command | - | tier N2 (GOTO), grade PASS on server clauses; first crossing fired only chain 1013 (stub system message), no bark |
| CB-U08a.3 | PASS | N2 | positions consistent with bookmark data | FIND | - | |
| CB-U08a.5 RC Prisoner | PASS | N2 | 09:30:48.625 interact -> target resolved -> per-player dialog set onDialogDisplay "Free Prisoner 329" | target fallback (N3 targetUnit) | - | camera face failed (lab issue), target set by fallback |
| CB-U08a.6 ACCEPT | PASS | N1 | 09:30:53.557 dialogButtonChoice; advancing step 638 -> 2115 chain 1015 | lab_finish_dialog | - | |
| CB-U08a.7 RC door button | PASS | N2 | 09:31:22.663 interact; fire_interact_tag matched; Content: starting minigame chain 1016; Starting minigame session | fallback target | - | |
| CB-U08a.8 MG | PASS (+ NEEDS_HUMAN 0 server clause) | N1 | 09:31:23.004 Minigame started; 09:31:26.716 Minigame victory + session_end; chain 1017 advancing 638 -> step 2116 | driver text inconclusive/server_unreachable (expected on win) | - | driver NEEDS_HUMAN not needed: server shows the win. "player_closed_window" cancel at 09:31:26.789 and 09:31:28.834 after the victory, harmless |
| CB-U08a.9 RC Prisoner again | PASS | N1 | 09:31:46.561 interact, dialog "Free Prisoner 329" | hover_verified | - | |
| CB-U08a.10 ACCEPT | PASS | N1 | 09:31:51.429 choice -> dialog "Ambernol" (1021); 09:31:52.732 choice -> accepting mission 639 step 2117; completing mission 638 (completed directly); adding dialog sets 11000, 11030; removing dialog set 1019 | lab_finish_dialog | - | DB sgw_mission: 638 status 2 completed_steps 2114,2115,2116; 639 status 1 current_step 2117 |
| CB-U08a.11-12 cast + bark | PASS | N2 | 09:32:00.392 fire_enter_region Region2 (second crossing); Content: npc bark chain 11010 screen 200160, exactly once; entity query: spawn 520 id 102795 tag Cellblock_Ambient_Prisoner1 at (-271, 65.45, -135), spawn 521 id 102796 Cellblock_Ambient_Prisoner2 at (-254, 65.45, -135), both faction 1, Idle, velocity 0; instance_populated 09:28:39Z 49/49 spawned, 0 failed | FIND | - | witnesses of 102795: only entity 55. No second client present, so the second-client clause is untested (PARTIAL) |
| CB-U08a.13 SHOT-H | NEEDS_HUMAN | - | - | screenshot | Q1 UNSURE (one figure visible, second only as HUD portrait); Q2 NO (no overhead plates) | see sheet |
| CB-U08a.14 READ | PASS | read | consistent with the server log | | | |
| CB-U08a.15 | PASS | N1 | end bookmark 09:32:18Z at (-265, 65.45, -135) | | | |

Overall CB-U08a: PASS on server clauses (638 accepted on first crossing, Prisoner and door chain 638->639, cast placement, bark once on second crossing, no WARN/ERROR in window, no screen_not_cached/empty_text/actor_not_player). Second-client AoI clause PARTIAL (single client). Visual name plate NEEDS_HUMAN.

Open question for coordinator (not graded): spawns 520/521 exist at world load (09:28:39Z, 49/49), long before mission 638 completes (09:31:52Z); they are not gated on 638. Product or doc question.

SIGN-OFF SHEET (NEEDS_HUMAN):
- H-CB-U08-1 (CB-U08a.13): clause "name plate Op-CORE Soldier visible above two NPCs"; path C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261103-033213.673.png; driver did not open the image; looker: Q1 UNSURE (one orange-suited figure, second only as HUD portrait), Q2 NO (names only in top HUD frames). Server shows both soldiers 6 m and 11 m from the player at the time.

FINDINGS: none (no product bug found). Lab note: client_camera face failed on step .5 and client_minigame_play reads inconclusive on a win; both lab/driver issues, not product.

NOT RUN / BLOCKED: second-client witness check (no second client). Inventory not checked (no usable column in sgw_inventory for player_id).

RESUME HINT: none for the main flow. To close the AoI clause, a second client must stand near the cast; to close H-CB-U08-1, retake a screenshot with both soldiers in frame.

Part complete: YES. Character state: Dcbch (player 101) at (-265, 65.45, -135); mission 638 completed; mission 639 active at step 2117 (objective 2458); mission 622 active at step 2113; mission 689 completed. Items not verified.
