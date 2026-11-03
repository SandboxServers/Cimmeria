PACKET SP-DHA1-p2 VERIFIED 2026-11-03 by Sonnet; driver attempts: 1; server evidence window 2026-11-03 14:44:10-14:45:30 UTC
| Row | Grade | Tier | Server evidence | Screenshot | Issue |
|---|---|---|---|---|---|
| H20b | PASS (NEEDS_HUMAN items listed) | N1 (real_input right-clicks on 54, 42, 54; accept and dialog paging through the stock UI) | Chain 6303 at 14:44:22: button 8 on dialog 4357, mission accepted at step 3953, removed set 5149 (slot 54), added set 5151 on Ba'al (template 42). Chain 6304 at 14:44:47 (CmdCenter_Baal, one pass): dialog 4363, advance 3953 to 3954 ("Return to Moh'katan."), removed 5151, added set 120001 on Moh'Katan. Chain 6305 at 14:45:14 (CmdCenter_Mohkatan, one pass): dialog 4365, removed 120001, complete_mission 1324, "Mission completed directly". sgw_mission row player 110 / mission 1324: status 2 (completed), completed steps 3953 and 3954, current step null. Entities: Moh'Katan (template 54, spawn 343) at (-5, 0.37, -8); Ba'al (template 42, spawn 340) at (2, 1.95, 88). Pose rows: radius 4 m for all three clicks (limit 5.0). No WARN or ERROR in content, interaction or mission scopes in the window. | H20-after-baal, H20-complete (looker not run) | none |

## Not run

- Reward grading: chain 6305 carries no reward action in the seed (display_dialog, remove_dialog_set, complete_mission), and the inventory read after the run shows no new items; nothing was expected, nothing asserted.
- Haiku lookers for H20-after-baal and H20-complete: the screenshot files could not be located by the report's short names within the time budget. Server clauses are graded without them.

## Resume hint

Nothing to redo. If rerun: START LEG 3 from the fixture row (world 68 Harset_CmdCenter, [-5, 0.37, -12], mission 1324 `not_active`).

## Recipe

1. server_stage_fixture world 68, [-5, 0.37, -12], mission 1324 not_active; lab_ensure_in_world.
2. `.bug SPU SP-DHA1-p2 h20 b start`.
3. client_world_click right-click template 54 (range 4 m): BlurbWin opens. client_window_click Blurb_AcceptButton (server received button 8 on dialog 4357). lab_finish_dialog pages the Moh'Katan dialog.
4. `.gotolocation 2 1.95 84`; right-click template 42; lab_finish_dialog until finished (dialog 4363, step 3954).
5. `.gotolocation -5 0.37 -12`; right-click template 54; lab_finish_dialog (dialog 4365, mission completes).
6. `.bug SPU SP-DHA1-p2 h20 end`.

## SIGN-OFF SHEET

- Leg 3, H20-after-baal: is the dialog window closed and is an NPC visible? Screenshot lab_screenshot-20261103-084503.469.png. Driver: tracker shows "Return to Moh'katan." Looker: not run. Person to review.
- Leg 4, H20-complete: is the dialog window closed and is an NPC visible? Screenshot lab_screenshot-20261103-084529.634.png. Driver: tracker empty, no prompts. Looker: not run. Person to review.

## Findings

1. Coordinator deviation (not a defect): the character was `Uat Fxdhajaffa` (player 110), not `Uat Fxdhajaffab`, because the 8-character cap was hit. Server evidence is for player 110.
2. Observation: the accept was posted_only on the client, but the server logged dialogButtonChoice button 8 and chain 6303 fired, so the accept is confirmed server-side.
3. Observation: the first Moh'Katan click used pose route `fallback` (dialog set path), later clicks used `chain_tag`; expected for the offer being a per-player dialog set. Other chains for 1326, 1361 and 1664 on the same tag were skipped on unmet step conditions (DEBUG), as designed.
4. Driver note: the driver's clock for the Ba'al click (14:44:44) differs from the server's (14:44:47) by about 3 s; not a defect.
