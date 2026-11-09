# SP-DTO3-p1 result (verifier)

Character: reused fixture character Fxdcatwo (player id 105), client instance p2, world Tollana. Fixture `lab fixture applied` at 07:35:17Z (missions 1, moved true); only one applied line for this character, none on the relog. Evidence from the dr-lab server.log (UTC) and a DB read; no `dialog_set_maps cache miss` WARN and no WARN/ERROR for the character in the window.

| Row | Grade | Tier (N1/N2/N3/G) | Server evidence (query or log event, time) | Screenshot | Issue |
|---|---|---|---|---|---|
| U1 | PASS +NEEDS_HUMAN | N1 (client_world_click, native_camera; dialog paging and close via N3 Lua) | Start: chain 9007 `adding dialog set` 2656 at world entry 07:35:36.480, no `Mission accepted` before the click. `fire_interact_tag` + `Content: displaying dialog` 2153 (chain 9001) 07:36:58.432. Close (`dialogButtonChoice` dialog 2153, button -1) 07:37:21.338; then exactly one `Content: accepting mission` / `Mission accepted` (568, step 1963) 07:37:21.342-.343, `removing dialog set` 2656 (tmpl 420, chain 9004) 07:37:21.339, `adding dialog set` 4959 (chain 9006) 07:37:21.344, `advancing step` 1964 07:37:21.345. DB after: current_step_id 1964, status 1, completed_step_ids [1963], active_objective_ids [2310], repeats 0. | DTO3-U1-offer-glyph (lab_screenshot-20261109-013649.408.png), DTO3-U1-dialog-2153 (lab_screenshot-20261109-013718.260.png) | none |
| U4a | PASS +NEEDS_HUMAN | N1 (relog by coordinator tooling; click via client_world_click) | `player session ended` 07:39:00.978, world entry 07:39:19.573, `Loaded saved missions from DB`, `adding dialog set` 4959 (chain 9008) on `fire_player_loaded` 07:39:19.587; no 2656 restored; no second fixture applied. DB step 1964, status 1, repeats 0 (unchanged). Click: `displaying dialog` 2153 (chain 9002) 07:40:28.449, no accept/advance after the close at 07:40:33.596. | DTO3-U4a-1964 (lab_screenshot-20261109-014020.638.png) | none |

## Not run

- Rows U2a, U2b, U3, U4b, U4c: parts SP-DTO3-p2 to p4, out of scope here.
- Phasing rows U-PH1 to U-PH5: not runnable today.
- Page 5 of dialog 2153 was never viewed (driver closed at page 4 of 5). The five-screen and speaker-1100 clauses of "Offer shown" are therefore client-read partial: pages 1-4 read, the page-5 text unverified. The accept-on-close clause does not depend on the page reached: the server matched the close (button -1, `closed`) and accepted once.
- U4a "dialog 2153 again, not a new offer": server shows 2153 displayed via chain 9002 (in-progress), no new offer. Client first text was the page-4 line "We looking for anything in particular?", not the opening page (driver note); this is a client paging behaviour on reopen, noted, not graded against the row.

## Resume hint

Fixture character Fxdcatwo is now at mission 568 step 1964, active (status 1), dialog set 4959 bound. A re-run of U1 needs a fresh fixture apply (mission 568 not_active) to reset it; parts p2-p4 can start from this state only if they expect step 1964.

## Recipe

- Ids: `SELECT player_id FROM public.sgw_player WHERE player_name ILIKE '%Fxdcatwo%'` -> 105.
- Log: `Select-String` on the dr-lab `logs\server.log` for the character name and `SPU SP-DTO3`, window 07:35-07:41Z (the in-memory server_log_tail ring is too short for past events).
- DB: `server_db_query` on public.sgw_mission for player 105, mission 568.
- Screenshots resolved in the lab screenshots folder; three Haiku lookers run.

## Findings

1. U1 / client-read: the offer-glyph screenshot (taken 07:36:49, before accept, while the offer bind 2656 was active) was read by the looker as a "?" icon at waist level, not an exclamation mark above the head. Server cannot confirm the glyph; client-read only, goes on the sign-off sheet. Not graded as a failure.
2. U1: `.gotolocation` echoed as a /tell to the own character (driver note); player still moved. Lab/driver chat behaviour, not a product finding for this packet.
3. No unexpected WARN/ERROR; `dialog_set_maps cache miss` count 0 in the window.

## Sign-off sheet

| Row | What to look at | Screenshot path | Driver's words | Looker pre-screen | Your read of the numbers |
|---|---|---|---|---|---|
| U1 | Offer glyph above Copplemann before accept (expect an offer mark) | lab_screenshot-20261109-013649.408.png | "shot not viewed" | Q1 NO: only a "?" icon near the waist, not above the head. Q2 NO: question mark, not exclamation. Q3 UNSURE: no distance indicator. | Server: offer bind 2656 active, distance 4.03 m (driver read). Glyph shape needs a person. |
| U1 | Dialog 2153 window with speaker label | lab_screenshot-20261109-013718.260.png | page 4 of 5 | Q1 YES: dialog open with "We looking for anything in particular?". Q2 NO: not "Welcome to" (shot is page 4). Q3 YES: "Capt. Copplemann" shown. | Server logged dialog 2153 displayed; first-page text and page 5 unverified by shot. |
| U4a | Glyph after relog at step 1964 (expect in-progress mark) | lab_screenshot-20261109-014020.638.png | "shot not viewed" | Q1 NO: "?" at waist level, not above head. Q2 NO. Q3 YES: white round "?" icon. Q4 NO: a mark is visible. | Server: 4959 in-progress bind restored, step 1964. Mark is a question mark, consistent with in-progress; position needs a person. |