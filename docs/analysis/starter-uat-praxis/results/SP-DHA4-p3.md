# SP-DHA4-p3 result (FZ-U2h part h2)

PACKET SP-DHA4-p3 VERIFIED 2026-11-09 by Sonnet; driver attempts: 1; server evidence window 06:42:38Z-06:43:23Z (SigNoz, lab run dr-lab, character Fxdcatwo, player_id 105)

| Row | Result | Tier | Server evidence (time, event or query, key fields) | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| FZ-U2h part h2, abandon in CC | PASS | G/typed (/abandonmission, no downgrade) | Start bookmark 06:42:38Z: 1664 status 1 step 5226, objectives 6061/6062 status 0, world Harset_CmdCenter, no offered dialog. 06:43:02.60Z: cell `abandonMission` (mission 1664), `mission_abandoned` INFO (world Harset_CmdCenter), `fire_mission_abandoned: matched` (5 actions), then `Abandoned mission row deleted` (rows 1). End bookmark 06:43:22Z: 1664 absent from the mission list; recent_events show `6016:RemoveDialogSet` x4 then `6511:AddDialogSet`, then `6512:DisplayDialog`, `dialog=4457 chain=6512`; `offered_dialog_ids [4457]`. DB: no 1664 row, no 1361 row for player 105. | Driver: gmMissionList "no active missions" after abandon; Marsh TALK native_level native_camera, N1, DialogWin opened; dialog closed with Done, not Accept | Skipped (NEEDS_HUMAN, file names only) | Server clauses PASS: offer returned after abandon (6016 then 6511 in one event, 6512 display_dialog 4457). 1361 never active. No WARN or ERROR in the window. Look-and-feel clauses H2-CUE and H2-DIALOG are NEEDS_HUMAN. |

## /abandonmission route

`/abandonmission 1664` WORKS (unlike /gmmissionclear, known broken, SP-DHA1-p1-1). Server logged `abandonMission`, `mission_abandoned` and `fire_mission_abandoned: matched` for 1664, and the mission row was deleted from the DB (read via server_db_query: no 1664 row afterwards). Note: the driver saw no abandon reply line in channel 9; the effect is proven server-side only.

FINDINGS:
1. None. No WARN or ERROR in the window.

NOT RUN / BLOCKED: Haiku looker skipped by instruction.

RESUME HINT: none; h2 complete. The FZ-U2h grade still combines -p2, -p3 and -p4.

SIGN-OFF SHEET:
FZ-U2h2 | H2-CUE | lab_screenshot-20261109-004311.083.png | "?" offer cue over Marsh after the abandon | not run
FZ-U2h2 | H2-DIALOG | lab_screenshot-20261109-004319.877.png | dialog 4457 open with an Accept button (client showed Done) | not run
