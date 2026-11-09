# SP-DTO3-p4 result (verifier)

Character Fxdcatwo (player_id 105), instance p2, Tollana. Evidence: dr-lab `logs\server.log` (UTC), window 07:56-08:01Z. Fixture `lab fixture staged` 07:56:47.280, `lab fixture applied` 07:56:50.724 (missions 1). Bookmarks: U3 start 07:57:54.107, U3 turned-in 07:58:23.180, U4c start 07:58:24.738. No `dialog_set_maps cache miss` WARN in the window (0 matches); no WARN/ERROR for the character in 07:56:57-08:00:50 other than the pre-fixture `Inbound client message dropped` (unhandled_account_method) at 07:56:43.568, which is before the part.

| Row | Grade | Tier (N1/N2/N3/G) | Server evidence (query or log event, time) | Screenshot | Issue |
|---|---|---|---|---|---|
| U3 | NEEDS_HUMAN | N1 (client_world_click on Copplemann; Next clicks posted-only; G for .gotolocation/.bug) | Start: fixture applied 07:56:50.724; `adding dialog set` 6393 (tmpl 420, chain 9009, dialog 2154) 07:56:57.027 on load. Click: `Content: displaying dialog` 2154 (chain 9003) 07:58:07.106. Close: `fire_dialog_choice: matched` dialog 2154 07:58:23.176, `removing dialog set` 6393 (chain 9005) 07:58:23.176, `Content: completing mission` 568 (chain 9005, once) 07:58:23.177, `Mission completed directly` 07:58:23.177. Seed: `resources.dialog_screens` 2154 idx0 speaker 1100, idx1 speaker 0, idx2 speaker 1100; 2153 idx 0,2,4 speaker 1100, idx 1,3 speaker 0. Mission DB: coordinator snapshot after the part status 2, step null, repeats 1 (my own read at verify time returns no row because the 08:01:21 fixture of the next attempt re-staged the mission; snapshot is the coordinator's). Label shown is client-read only. | DTO3-U3-dialog-2154 p1 lab_screenshot-20261109-015811.048.png, p3 lab_screenshot-20261109-015820.347.png | none |
| U4c | PASS +NEEDS_HUMAN | N1 (relog by coordinator tooling; camera via client_camera) | Session end 07:59:19.713, `Loaded saved missions from DB` + `Restored missions from the saved journal` 07:59:31.875/.879. No `adding dialog set` (2656, 4959 or 6393) after the relog, no accept/advance/complete lines after 07:58:23. Exactly one `lab fixture applied` (07:56:50.724) in the part; the next one at 08:01:21.762 is after the part (next attempt). Status completed (2) before (log 07:58:23) and after (coordinator snapshot); client world_id 19, same pos. | DTO3-U4c-completed lab_screenshot-20261109-020029.773.png | none |

## Not run

- Phasing rows U-PH1 to U-PH5: not runnable today.
- U3 2153 half: not opened in this part; cited from `results/SP-DTO3-p1.md` (looker there: "Capt. Copplemann" shown on 2153 page 4; seed speaker 1100 on 2153 idx 0,2,4).
- Pages 1 and 3 of 2154 were looked at; page 2 (shot lab_screenshot-20261109-015815.511.png) was not sent to a looker. Next clicks were posted-only (inconclusive), but text changed per page.
- Known gaps (SP-DTO4-p1..p3): abandon, silent completed click, double-click, objective-despite-step_enabled.

## Resume hint

Fxdcatwo had mission 568 completed (status 2, repeats 1) at the end of the part. The character was then re-staged by a later fixture (08:01:21Z), so its mission 568 row is currently absent; re-run needs a fresh fixture at step 1966.

## Recipe

- Ids: player 105 (Fxdcatwo) from the p1 result.
- Log: parse dr-lab `logs\server.log` JSON lines (lines around `SPU SP-DTO3-p4` bookmarks), filter on player_id 105 and messages containing `dialog set`, `mission`, `fixture`, `cache miss`.
- DB: `server_db_query` on public.sgw_mission (now empty: re-staged) and `resources.dialog_screens` for 2153/2154.
- Three Haiku lookers on the p1, p3 and U4c shots.

## Findings

1. U3 (client-read): the speaker is displayed only as the dialog title "Capt. Copplemann" (looker p1 and p3 both read it as the label next to the portrait), matching screen speaker 1100, not the template speaker 968. Not graded as a failure (project row W6).
2. U4c (client-read): looker saw no mark above the head, but a "?" icon at waist level (the HUD-level "?" also seen in p1 U1/U4a shots). It is not an exclamation mark. The server state is completed with no bind, so a "?" at waist level on a completed NPC may be a different HUD element; needs a person to say whether it is a mission glyph. Not a product finding on this evidence.
3. No unexpected WARN/ERROR in the part window; `cache miss` count 0.

## Sign-off sheet

| Row | What to look at | Screenshot path | Driver's words | Looker pre-screen | Your read of the numbers |
|---|---|---|---|---|---|
| U3 | Speaker label on 2154 (name "Copplemann" or template speaker) | lab_screenshot-20261109-015811.048.png (p1), lab_screenshot-20261109-015820.347.png (p3) | Title "Capt. Copplemann" on every page, no separate speaker label read; shots not viewed | p1: Q1 YES, Q2 YES ("Capt. Copplemann" beside portrait), Q3 "Capt. Copplemann". p3: Q1 YES, Q2 YES (label above text), Q3 "Capt. Copplemann" | Seed screens: Copplemann lines 1100, player lines 0. The shown name is Copplemann-style; 2153 cited from SP-DTO3-p1 (same name). |
| U4c | No mark above Copplemann after relog in completed state | lab_screenshot-20261109-020029.773.png | Copplemann targetable, shot after camera faced him; not viewed | Q1 NO, Q2 NO, Q3 NO (the "?" is at waist level, not above head), Q4 NO (a "?" HUD icon visible at waist level) | Server: completed, no bind restored after relog. The waist-level "?" needs a person to identify. |
