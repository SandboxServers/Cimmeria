# SP-DHA5-p1 result (FZ-U2e)

PACKET SP-DHA5-p1 VERIFIED 2026-11-09 by Sonnet; driver attempts: 2; server evidence window 06:47:34Z-06:54:40Z (dr-lab server.log, character Fxdcatwo, player_id 105)

| Row | Result | Tier | Server evidence (time, event or query, key fields) | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| FZ-U2e, abandon at the Kaerlia step (5224) | PARTIAL | G/typed (/abandonmission, no downgrade) | a1 06:47:43.243Z and a2 06:51:49.774Z: `mission_abandoned` mission 1664, `fire_mission_abandoned: matched` (4 actions). Chain 6016 `removing dialog set` x4 both runs: 7016 (template 260), 120301 (261), 120302 (10), 120303 (54). DB: no 1664 row, no 1361 row for player 105. Relog a1 06:48:31Z and a2 06:52:56Z: `missions_restored`, no chain 6014, no `adding dialog set`, no `mission.accept`, no chain 6020. a2 fixture at 06:50:47Z showed 6014 adding 7016 (expected, restage at step 5224). No WARN or ERROR in the window. | gmMissionList before: `#1664 active step 5224`; after: `no active missions` (both runs, before and after relog); no abandon reply line in chat | Skipped (no looker; NEEDS_HUMAN) | Server clauses PASS: abandon accepted, cues removed, no active row, not offered again (incl. after relog). UNVERIFIED: Kaerlia bark (chain 6010, screen 117158, no dialog 6169). No chain 6010, npc_bark or display_dialog lines in either run, because the Kaerlia click never landed (hover_timeout / read_mouse_over, lab harness gap, not a product defect). No 6169 dialog seen either. Cue clauses E-KAERLIA-CUE, E-KAERLIA-AFTER, E-RELOG are NEEDS_HUMAN. Reused character, leftover objective counts ignored. |

FINDINGS:
1. None. No WARN or ERROR in the window.

NOT RUN / BLOCKED: Kaerlia bark clause UNVERIFIED (click harness gap in both runs; a2 e4b READ not run, budget). Haiku looker skipped.

RESUME HINT: to close the bark clause, a driver must land a native click on Sgt. Kaerlia (entity 100205) after the abandon and check the server log for chain 6010 and screen 117158 with no dialog 6169; needs the camera on the NPC (on_screen was false at 4 m).

SIGN-OFF SHEET:
FZ-U2e | E-KAERLIA-CUE | lab_screenshot-20261109-004738.026.png (a1), lab_screenshot-20261109-005143.175.png (a2) | Kaerlia cue present before the abandon | not run
FZ-U2e | E-KAERLIA-AFTER | lab_screenshot-20261109-004749.465.png, lab_screenshot-20261109-004817.309.png (a1), lab_screenshot-20261109-005158.704.png, lab_screenshot-20261109-005221.258.png (a2) | Kaerlia cue gone after the abandon | not run
FZ-U2e | E-RELOG | lab_screenshot-20261109-004920.812.png (a1), lab_screenshot-20261109-005347.014.png (a2) | cues still gone after relog | not run
