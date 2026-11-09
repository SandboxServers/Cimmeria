# SP-DHA5-p2 result (FZ-U2f)

PACKET SP-DHA5-p2 VERIFIED 2026-11-09 by Sonnet; driver attempts: 1; server evidence window 06:55:13Z-06:58:30Z (dr-lab server.log, character Fxdcathree, player_id 106)

| Row | Result | Tier | Server evidence (time, event or query, key fields) | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| FZ-U2f, abandon at door-control step 5225 | PARTIAL | G/typed (/abandonmission, no downgrade) | 06:55:13Z login restore: active `567:4039,1664:5225`; chain 6015 added door set 120301 (fixture). Bookmarks 06:56:04Z start, 06:56:13Z abandon. 06:56:17.91Z `mission_abandoned` mission 1664 (Harset), `fire_mission_abandoned: matched` actions 4, then chain 6016 `removing dialog set` 7016 (tpl 260), 120301 (tpl 261), 120302 (tpl 10), 120303 (tpl 54). DB at grading: no 1664 row for player 106 (only 567 active plus completed). Relog restore 06:57:29Z: active `567:4039` only; no `adding dialog` (6015/6012/6020), no `mission.accept` 1664 after the abandon. No WARN or ERROR in the window. | gmMissionList showed 1664 step 5225 before and absent after the abandon and after the relog (driver, matches server). | Skipped (NEEDS_HUMAN, file names only) | Server clauses PASS: abandon accepted, cues removed, no 1664 row, not re-granted after relog (P0 used). UNVERIFIED: Kaerlia keep-moving bark (chain 6010, screen 117158); Kaerlia click failed on read_mouse_over transport timeout (lab harness gap, same as DHA5-p1), no bark in logs. The f4 post-relog Kaerlia click was never run (budget). Door control is the D-PU6 placeholder; look-and-feel clauses are NEEDS_HUMAN. Mission 567 is a leftover, ignored. |

FINDINGS:
1. None. No product bug. Kaerlia click failures are harness gaps, no issue filed.

NOT RUN / BLOCKED: Haiku looker (skipped by instruction). Kaerlia bark clause UNVERIFIED (both pre- and post-relog).

RESUME HINT: rerun only the Kaerlia TALK before and after the relog once the mouse-over transport timeout is fixed; expect chain 6010 / npc_bark 117158 and no `display_dialog 6169`.

SIGN-OFF SHEET:
FZ-U2f | F-DOOR-CUE | lab_screenshot-20261109-005610.349.png | door-control cue present before the abandon (D-PU6 placeholder) | not run
FZ-U2f | F-DOOR-AFTER | lab_screenshot-20261109-005627.152.png | door cue gone after the abandon | not run
FZ-U2f | F-KAERLIA | lab_screenshot-20261109-005634.342.png | Kaerlia with no offer cue; keep-moving bark not captured | not run
FZ-U2f | F-KAERLIA-FAIL | lab_screenshot-20261109-005715.074.png | failure shot of the mouse-over timeout, harness gap | not run
FZ-U2f | F-RELOG | (not taken, budget) | no door or offer cue after relog | not run
