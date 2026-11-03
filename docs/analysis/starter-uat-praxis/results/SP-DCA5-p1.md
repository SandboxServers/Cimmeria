PACKET SP-DCA5-p1 VERIFIED 2026-11-03 by Sonnet; driver attempts: 1; server evidence window 13:15:34Z-13:18:03Z (local lab server, run dr-lab, server.log UTC)

Substitutions: Soldier S6 = Fxdcathree (player 106, entity 79, archetype 1); Jaffa S7 = Fxdcafour (player 107, entity 80, archetype 8). Both on the p2 client (brief said p3). Bookmarks present: S6 start 13:15:39.6Z, S6 end 13:16:00.0Z, S7 start 13:17:06.6Z, S7 end 13:17:42.4Z. Looker skipped by order; screenshots listed by file name only.

| Row | Result | Tier | Server evidence (time, source, key fields) | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| S6 | PASS | N1 walk, G bookmarks | sgw_player 106 archetype=1 (not 8). Bookmark S6 end 13:16:00Z: pos (358.04,70.16,894.48), regions_inside ["Castle.Infirmary"]; recent_events region_hint "Castle.Infirmary entering=true" ~12.9 s before. Missions at S6 end: 704 (status 2) and 1520 (status 2) only, no 1521 row. Log grep for accept_mission/1521/1369 in 13:15-13:18: hits only for player 107 (13:17:14), none for 106. No WARN/ERROR for player 106. | move_to arrived, no fallback; DIALOG none (client-read only) | skipped | Entered the box (x 358.04, z 894.48 inside 344.9-371.0 / 880.2-904.6); offer did not fire. |
| S7 | PARTIAL | N1 walk + right-click, G bookmarks | Route: server_nav_path space_id 65537 (8 is "not loaded", see defects) from (358,70.2,912) to (356,70.18,900.5): status ok, 2 waypoints (a straight line). Offer: 13:17:14.18Z chain 1369 accept_mission 1521 then chain 1370 step 4455->4456 for player 107; S7 end snapshot shows 1521 status 1, step_id 4456. Walk: S7 start (358.0,70.22,912.0) to S7 end (355.98,70.16,900.57), about 11.4 m in one leg, 4.57 m from the tank (356,70.18,896) = within 5.5 m. Click: 13:17:37.88Z interact target 100388 "Symbiote Tank", fire_interact_tag Castle_SymbioteTank matched, chain 1374 StartMinigame Livewire, "Minigame started" room 1020 13:17:38.18Z; state BSF_PlayingMinigame|BSF_MovementLock. | move_to arrived, legs 1, no fallback; world_click verdict confirmed, MinigameWin opened (client-read) | skipped | Every server clause held; graded PARTIAL only because waypoints were never pasted (brief rule: straight-hop). The single hop equals the real nav route (2 waypoints), so the walk is in effect proven. |

FINDINGS:
1. Row S7 window, 13:16:10Z and 13:16:18Z (during the Jaffa play/relog, before the S7 bookmark): WARN "Archetype has no ability-tree rows; sending an empty onAbilityTreeInfo" (event tree_missing, reason archetype_has_no_tree, player 107, archetype 8 Jaffa), targets abilities / world_entry.play_character and bandolier.refresh_appearance. Not expected by the row and outside the S7 bookmark window; Jaffa tree seed gap on a lab fixture character. No existing issue found by `gh issue list --search`, and no K-number covers it. Not filed: it is not on the 1521 path, and I cannot tell without reading the ability seed whether it is a deliberate fixture state. Flag for the coordinator.
2. No product bugs found on S6/S7; no issues filed.

NOT RUN / BLOCKED: none. Brief defects:
- S7.2 (and the fixture): waypoints were not pasted (nav space not loaded for the coordinator); the driver used a straight hop, and its report "distance 0.27 m" is the end-to-target gap, not the leg length (the real hop was about 11.4 m). Reword the step to name the leg length.
- Verify file S7 route check: it says `space_id: 8`; that is the world_id. The nav space id is 65537 (space 8 returns "not loaded"). Fix the verify file.
- Brief said instance p3; the runner used p2. Fixture said 1521 not_active for the Soldier, true; the Soldier's S6 snapshot also shows 1520 completed (consistent with the fixture).
- S6 driver did not read the mission list or chat; server snapshot covered it.

RESUME HINT: none required for a PASS/PARTIAL. To upgrade S7 to PASS, rerun only the walk with the 2-point route pasted (the route is (358.0,70.38,912.0) -> (356.0,70.18,900.5)).

SIGN-OFF SHEET
S6 | lab_screenshot-20261103-071601.977.png (S6-infirmary) | avatar in the Infirmary, no dialog, per the driver's client reads (DIALOG none); picture not inspected | not run (looker skipped) | cannot tell
S7 | lab_screenshot-20261103-071741.524.png (S7-tank-click) | MinigameWin opened after the tank click (client-read); the driver did not inspect the picture | not run (looker skipped) | cannot tell (server confirms Livewire started, BSF_PlayingMinigame)
