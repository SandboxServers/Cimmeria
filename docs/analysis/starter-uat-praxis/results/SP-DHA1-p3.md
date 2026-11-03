PACKET SP-DHA1-p3 VERIFIED 2026-11-03 by Sonnet; driver attempts: 2; server evidence windows 14:52:27Z-14:54:40Z (attempt 1, legs 1-3 up to the dialog) and 14:59:23Z-14:59:55Z (attempt 2, leg 3 choice)
Log source: local server log of the dr-lab worktree (logs\server.log, UTC); SigNoz and ssh not used. Character Fxdcatwo (player 105, account 11). Attempt 1 bookmarks "h31 start" 14:52:32.6Z, "h31 a end" 14:54:19.8Z; attempt 2 bookmarks "h31 a2 start" 14:59:28.3Z, "h31 a2 end" 14:59:50.8Z. Lookers skipped by instruction.

| Row | Grade | Tier | Server evidence | Screenshot | Issue |
|---|---|---|---|---|---|
| H31a leg 1 offer and accept | PASS | N1 (world click, lab_finish_dialog) | Attempt 1. 14:52:09.730 Content: adding dialog set 5254 on template 10, chain 6511, at load. 14:52:42.073 fire_interact_tag CmdCenter_Marsh, chain 6512 displaying dialog 4457. 14:52:54.742 dialogButtonChoice 4457 button 8, fire_dialog_choice matched (3 actions), chain 6513 accepting mission 1361, set 5254 removed, set 6397 added on Moh'Katan. | client-read only | none |
| H31a leg 2 Moh'katan | PASS | N1 | Attempt 1. 14:53:13.054 fire_interact_tag CmdCenter_Mohkatan, chain 6515 displays 4458, removes set 6397, advances 1361 to step 4041 (mission_step_advanced). | client-read only | none |
| H31a leg 3 Hansen (bind, dialog, choice) | PASS | N1 (travel and .gotolocation were G setup) | Bind: attempt 1 14:53:27.007 chain 6516 adds set 6399 on template 212 at the 68-to-57 crossing (fire_player_loaded); attempt 2 session re-fired 6516 at 14:59:06.492 (entity 104, before the window). Attempt 2: 14:59:37.101 interact on Hansen opens per-player dialog 4459. 14:59:40.388 dialogButtonChoice 4459 button 195. 14:59:40.393 fire_dialog_choice matched (3 actions). 14:59:40.394 chain 6518 displays dialog 4460, removes set 6399, advances 1361 4041 to 4042 (mission_step_advanced 14:59:40.396). 14:59:47.925 dialogButtonChoice 4460 button -1 is the blurb closing. DB sgw_mission player 105 mission 1361: current_step_id 4042, completed steps 4040 and 4041, status 1. | lab_screenshot-20261103-085952.754.png (client-read only) | none |
| Check (a) enablement | PASS | n/a | 6511 offered at load to an archetype 1 (Soldier) character in world 68, 1360 and 1664 not active (fixture applied 14:52:04.134). | none | none |
| Check (b) disjoint from 1360 | PASS | n/a | No chain 6501 or 6502 line in either window; 1360 not in the player's mission rows. The 4038 half is graded by SP-DHA2-p1 to p4. | none | none |

Overall H31a (legs 1-3): PASS. Chains 6511, 6512, 6513, 6515, 6516 and 6518 each fired once and the DB holds 1361 at step 4042. Step 6517 is intentionally absent. NEEDS_HUMAN items listed.

## Not run
- Nothing. The earlier leg 3 gap (choice click) was closed by attempt 2.

## Resume hint
None for H31a. The next part starts from 1361 at step 4042 (Deliver the weapons samples to Moh'katan) in world 57.

## Recipe
- Leg 1: ensure_in_world, .bug start, client_world_click Marsh at 4 m, lab_finish_dialog accept=true (6 pages, Accept = button 8). The offer is accepted via the DialogWin accept.
- Leg 2: .gotolocation Harset_CmdCenter -5 0.37 -12, client_world_click Moh'Katan at 4 m, lab_finish_dialog (5 pages, Done).
- Leg 3: .gotolocation Harset -24 -68.92 10 (or a 68-to-57 crossing so 6516 binds; check "adding dialog set 6399"), client_world_click Hansen (template 212, right-click, 4 m) opens dialog 4459. The choice button is Dialog_GenericButton1 (label "Convince Hansen.", button id 195) on page 1 of the dialog: click it with client_ui_click BEFORE lab_finish_dialog. Result was native_cegui N1 posted_only/inconclusive; the server row is the proof. The next page then shows Dialog_NextButton and Dialog_PrevButton1; lab_finish_dialog (3 pages, Done) closes it.

## SIGN-OFF SHEET
- H31-marsh-offer: is there a "?" icon over Marsh? Not judged by the driver. Screenshot lab_screenshot-20261103-085237.717.png. Looker not run.
- H31-offer-dialog: is the offer dialog open with an Accept button? Server side holds (dialog 4457, Accept button 8). Screenshot lab_screenshot-20261103-085245.174.png. Looker not run.
- Hansen dialog: does page 1 show the "Convince Hansen." button? Server confirms button 195 was received; attempt 2 screenshot lab_screenshot-20261103-085952.754.png (taken at the end bookmark). Looker not run.

## Findings
1. Earlier leg 3 PARTIAL is resolved. Attempt 1's driver misreported "no choice button" (the button is Dialog_GenericButton1 on page 1, not on a later screen) and closed the dialog with Done. Cause was a driver/recipe error, not a product defect. No issue filed.
2. Offer shape. The Marsh offer is a DialogWin with no blurb window, matching chain 6512 display_dialog 4457 on interact. No defect.
3. Warnings. No WARN or ERROR inside attempt 2's window. Attempt 1: fixture noise at 14:52:04.132 WARN "Mission 1360 generic update refused" (invalid_or_stale_snapshot) during fixture apply, 28 s before the first bookmark; nothing inside the leg windows.
4. Attempt 2 reused the fixture character without a fresh crossing; 6516 had re-bound at 14:59:06.492 on the session load, 22 s before the start bookmark, so the bind is evidenced by that row and by chain 6518 removing set 6399.
