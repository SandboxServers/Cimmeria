# SP-DHA5-p3 result (FZ-U2g)

PACKET SP-DHA5-p3 VERIFIED 2026-11-09T07:1xZ by Sonnet; driver attempts: 2 (a1 BLOCKED, bridge timeout, not a product matter); server evidence window 07:04:20-07:05:45Z (dr-lab logs/server.log, instance p2)

Deviations: character Fxdcatwo (player_id 105, no "Uat " prefix); fixture staged with 1361 not_active. Evidence from the dr-lab server.log (the in-memory lab-server buffer had rolled out; SigNoz returned no rows for the window). Typed /abandonmission leaves no chat or log line of its own; the server-side `mission_abandoned` event is the evidence.

| Row | Result | Tier | Server evidence (time, event or query, key fields) | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| FZ-U2g abandon at 5226 after one report | PARTIAL (server clauses PASS; Kaerlia clauses BLOCKED-known "Kaerlia click") | G/typed for abandon; Marsh TALK native_camera (verdict inconclusive) | 07:04:47.322 `Content: complete objective` 6061 mission 1664 chain 6018 (Marsh bark screen 200180, dsm 120302 removed) - exactly one, none after. 07:05:00.830 `mission_abandoned` 1664 player 105, world Harset_CmdCenter (68), after the abandon bookmark. Chain 6016 removed dsm 7016 (Kaerlia), 120301 (door), 120302, 120303 immediately after. Chain 6511 added dsm 5254 on Marsh (expected "?"). DB: `sgw_mission` has no row for 1664 (player 105 holds only 704, 708, 1520, 10300, all status 2). No chain 6033/6034 for player 105, no `mission.accept`, no chain 6020, no add_dialog_set 7016/120301 after the abandon. | gmMissionList before: #1664 step 5226 obj 3/5; after: "no active missions". Marsh chat "Report received. Good work out there." | skipped | Not run: Kaerlia keep-moving (chain 6010, bark 117158, no display_dialog 6169) - BLOCKED-known "Kaerlia click"; relog "not offered again" check not done. Re-offer clause graded under P0 (abandon final). 6033/6034 seen in the log belong to player 106 (another run), not this one. |

FINDINGS:
1. Minor, harness: WARN `Inbound client message dropped` (unhandled_account_method, msg 221 baseMethod, player 105) at 07:05:44Z, after logoff. Not part of the row; no issue filed.
2. Chain 6018 (not 6061 under 6016) is the Marsh report chain; the brief names only 6016 for drops, which still matches.
3. objective 6061 credited once; objective count 3/5 before, abandon cleared the row. No product bug.

NOT RUN / BLOCKED: step g5 (Kaerlia click): BLOCKED-known "Kaerlia click" (hover_timeout). Attempt 1: bridge timeout before any step.

RESUME HINT: once Kaerlia right-click works, redo only g5 on a fresh 1664@5226 fixture (abandon, then click Kaerlia, expect chain 6010 npc_bark 117158 and no display_dialog 6169), plus a relog to confirm no dsm 7016/120301 returns.

SIGN-OFF SHEET:
FZ-U2g | G-MARSH-CUE | lab_screenshot-20261109-010438.571.png | "?" or marker over Marsh before abandon | skipped (NEEDS_HUMAN)
FZ-U2g | G-KATAN-CUE | lab_screenshot-20261109-010456.196.png | marker over Moh'Katan before abandon | skipped (NEEDS_HUMAN)
FZ-U2g | G-KATAN-AFTER | lab_screenshot-20261109-010508.327.png | Moh'Katan cue gone after abandon | skipped (NEEDS_HUMAN)
FZ-U2g | G-MARSH-AFTER | lab_screenshot-20261109-010512.806.png | Marsh shows the Praxis "?" (chain 6511) | skipped (NEEDS_HUMAN)
FZ-U2g | G-KAERLIA | not taken | Kaerlia says keep moving | BLOCKED-known
