PACKET SP-DHA1-p4 VERIFIED 2026-11-03 by Sonnet; driver attempts: 3 (attempt 3 updated 2026-11-03 by Sonnet); server evidence windows 15:04:12-15:05:55 UTC (attempt 1), 15:09:37-15:10:43 UTC (attempt 2) and 15:13:46-15:14:32 UTC (attempt 3) (local dr-lab server.log; character Fxdcathree, player 106, instance p2)

Log source: dr-lab `logs\server.log` (SigNoz/ClickHouse helper not used). Bookmarks `SPU SP-DHA1-p4 h31 b start` 15:04:17.9Z and `h31 end` 15:05:33.6Z found in the log. Mission table `public.sgw_mission` for player 106, mission 1361: after attempt 3: status 2 (complete), current_step_id null, completed steps 4040-4043, 4693, 4694, completed objectives 4654-4657, 5572, 5573.

| Row | Grade | Tier | Server evidence | Screenshot | Issue |
|---|---|---|---|---|---|
| H31b fixture login | PASS | G (setup) | chain 6519 `adding dialog set` 6398 (Moh'Katan) at 15:03:57Z, just before the window (the login itself) | none | none |
| H31b leg 4 Moh'katan | PASS | N1 (driver); server shows chain_tag route | 15:04:26Z `fire_interact_tag: matched` CmdCenter_Mohkatan, chain 6520, dialog 4466, step 4042 -> 4043 | none | none |
| H31b leg 5 Ba'al | PASS | N1 (driver) | 15:04:39Z chain 6522, dialog 4461, step 4043 -> 4693, dialog set 6396 (Anat, dialog 4462) added | none | none |
| H31b leg 6 Anat (attempt 2) | PASS | N1 (driver: right-click, Dialog_GenericButton1 click) | b2 window 15:09:42Z-15:10:23Z, player 106: 15:09:48Z interact Anat (dialog 4462, set 6396); 15:09:51.305Z dialogButtonChoice 4462 button 194, then fire_dialog_choice matched (4 actions); chain 6525 displays dialog 4463, removes set 6396, advances step 4693 -> 4694 (Return to Marsh), adds Marsh dialog set 5253 (dialog 4465); 15:09:56Z dialogButtonChoice 4463 button -1 (close). sgw_mission 1361: status 1, current_step_id 4694, completed steps 4040-4043 and 4693. No WARN/ERROR in window | none | none |
| H31b leg 7 Marsh (attempt 3) | PASS | N1 (driver: right-click via client_world_click, lab_finish_dialog paging) | window 15:13:50Z (`h31 c start`) to 15:14:10.9Z (`h31 c end`), player 106: 15:14:01.267Z interact Colonel Marsh (entity 100199), `fire_interact_tag: matched` CmdCenter_Marsh (3 actions), displays dialog 4465, removes Marsh dialog set 5253, chain 6527 `Content: completing mission` 1361 (`Mission completed directly`, event mission_completed); 15:14:09.75Z dialogButtonChoice 4465 button -1 (close). sgw_mission 1361 for 106: status 2, current_step_id null, completed steps include 4694. No WARN/ERROR in the leg window (one unrelated WARN at 15:13:25Z, mission 1360 generic update refused, before the relog/window) | lab_screenshot-20261103-091413.816.png (client-read: mission-complete chat line and next-mission tracker, driver only; looker not run) | none |

Findings on the dispatch of leg 6: the Anat dialog was shown through the per-player dialog-set fallback route, which is the normal display path; chain 6525 only fires on the dialog choice, which the driver never clicked (the Anat dialog had 7 pages, no choice button seen by the driver). Whether a choice button exists on that page is unverified.

Overall grade: PASS (H31b legs 4, 5, 6 and 7 PASS; mission 1361 complete). Before leg 7 the player relogged: restore binds 6526 (Marsh dialog set 5253) and 6504 fired at 15:13:30Z, as designed; other restore binds not checked. No product bug found.

## Not run
- Leg 7 Marsh: ran in attempt 3 (PASS). Earlier: attempt 1 failed at the harness on_screen step; attempt 2 stood 6.8 m away.
- Restore binds 6514, 6521 and 6523: not checked (6526 did fire on the pre-leg-7 relog).
- Looker H31-complete: skipped.

## Resume hint
None. Mission 1361 is complete (status 2); H31b needs no further attempt.

## Recipe
Passed calls (driver-reported, server-confirmed): fixture login into world 68 (6519 bind), `client_world_click` right-click on Moh'Katan (entity 54) at 4 m then `lab_finish_dialog` (chain 6520); `.gotolocation Harset_CmdCenter 2 1.95 84`, right-click Ba'al (42) at 4 m, `lab_finish_dialog` 4 pages (chain 6522). Anat right-click at `61.97 1.77 73.2` opens dialog 4462 (display only).

## SIGN-OFF SHEET
None required (no NEEDS_HUMAN items). Screenshot file: lab_screenshot-20261103-091413.816.png

## Findings
1. Row H31b leg 6: attempt 1 driver gap (choice not clicked); attempt 2 clicked button 194 and chain 6525 fired. Resolved, no product bug.
2. Row H31b leg 7: attempt 1 lab gap (camera on_screen), attempt 2 coordinator spot out of range, attempt 3 PASS at 3.91 m (chain 6527, complete_mission 1361, sgw_mission status 2). No issue file written.
