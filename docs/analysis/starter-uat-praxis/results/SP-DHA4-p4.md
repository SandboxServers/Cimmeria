PACKET SP-DHA4-p4 VERIFIED 2026-11-09 by Sonnet; driver attempts: 1; server evidence window 07:06:51Z-07:10:40Z (dr-lab logs/server.log + sgw_mission; lab-server buffer not used)

Deviations: instance p2, reused character Fxdcathree (player_id 106, no "Uat " prefix), leftover mission 567 ignored.

| Row | Result | Tier | Server evidence | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| FZ-U2h3 abandon elsewhere (h3) | NEEDS_HUMAN | G/typed (abandon, .gotolocation); N1 Marsh click (native_camera) | 07:06:57 bookmark "FZ-U2h3 start" in world 57 (Harset). 07:08:52 bookmark "abandon". 07:08:53.608 mission_abandoned mission 1664, world Harset; fire_mission_abandoned matched, chain 6016 (4 actions), dialog sets removed. NO chain 6511 or 6512 line anywhere between 07:06:57 and 07:09:02 (6511 did not bind in world 57). 07:09:00.49 .gotolocation to world 68; 07:09:02.794 fire_player_loaded Harset_CmdCenter (2 actions); 07:09:02.795 adding dialog set chain 6511 "Offer indicator on Marsh" dialog 4456 on template 10. 07:09:55.69 chain 6512 display_dialog 4457 npc 100199. DB: no 1664 row, no 1361 row (never active), 567 active step 4039 (leftover). | gmMissionList shows 1664 absent after abandon; TALK Marsh opened DialogWin, no Accept pressed | not run (skipped) | Server clauses PASS. Only H3-CUE ("?" marker) and H3-DIALOG look-and-feel remain. |

FINDINGS:
1. No mission or content WARN/ERROR in the window. One WARN at 07:10:40Z (inbound_message_dropped, unhandled_account_method, msg 221 baseMethod, Account), about 40 s after the last row action; outside the row window, not a mission or content target. Incidental, likely known client onPhysics/Account method noise; not filed.
2. Driver saw two bridge dispatch timeouts (-32603) on utc ops; lab/driver issue, no server effect.
3. The lab-server buffer and SigNoz were empty for this window; grading used the dr-lab server.log file plus DB.

NOT RUN / BLOCKED: none. Looker skipped by instruction.

RESUME HINT: none; row complete. User only needs to eyeball the two screenshots.

SIGN-OFF SHEET:
FZ-U2h3 | H3-CUE | lab_screenshot-20261109-010947.388.png | "?" offer marker over Col Marsh after returning to the Command Center | none (looker skipped)
FZ-U2h3 | H3-DIALOG | lab_screenshot-20261109-011000.970.png | Meet The Praxis dialog (4457) open with an Accept button | none (looker skipped)
