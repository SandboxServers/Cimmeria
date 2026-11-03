PACKET SP-DCA3-p1 VERIFIED 2026-11-03 by Sonnet; driver attempts: 1; server evidence window 12:44:06Z-12:48:02Z (dr-lab server.log, local lab; SigNoz not used)

Character Fxdcathree (player_id 106, account lab2, instance p2). Entity ids 71 (first login) and 70 (after relog).

| Row | Result | Tier | Server evidence (time, scope, event, key fields) | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| D6a | PASS (server); glyph look NEEDS_HUMAN | N1 | 12:44:12.279 Content: adding dialog set 5827 template 179 Castle_Ogilvie chain 1381 "Login at 4452: rebind Ogilvie" (fixture login, missions_restored active 1520:4452). 12:45:27.711 interact target 100102 Ogilvie, per-player dialog set to onDialogDisplay dialog 4983. 12:45:34.460 dialogButtonChoice 4983 button -1, fire_dialog_choice matched (3 actions). 12:45:34.466 chain 1378 removes set 5827; advancing step 4453; "Mission step advanced" mission 1520 for player 106. | verdict confirmed, DialogWin opened | not run (PNG not located) | .bug bookmark "d6a start" at 12:45:04.5. |
| D6b | PASS (server); glow look NEEDS_HUMAN | N1 | Logout snapshot 12:45:49.03; second login 12:46:02.25 (missions_restored active 1520:4453). 12:46:02.257 adding dialog set 5828 template 412 Castle_MedDispenser chain 1382 "Login at 4453: rebind dispenser". 12:47:15.790 interact target 100387 Med Dispenser; fire_interact_tag matched Castle_MedDispenser; chain 1371 starting minigame Livewire; Starting minigame session; 12:47:16.10 Minigame login successful and Minigame started (room 1015). DB at verify time: mission 1520 status 1, current_step_id 4453 at 12:47:59, later restaged by the next packet to 4454. | verdict confirmed, MinigameWin opened | not run (PNG not located) | "d6b start" bookmark 12:45:44.8. Logins in the part: 12:44:12 and 12:46:02, one logout between (a 12:48:24 login belongs to the next fixture stage). Driver's stated click time (12:46:46-12:47:10) was approximate; server shows the click at 12:47:15. |

FINDINGS:
1. No WARN or ERROR rows in the window 12:44:06-12:48:02Z.
2. Driver noted `client_window_read kind:"mission"` is unknown (harness gap, not a product defect). No issues filed.

NOT RUN / BLOCKED: Haiku looker not run; the screenshot PNGs could not be located from the verifier side (search timed out). Look clauses go to the sign-off sheet.

SIGN-OFF SHEET (NEEDS_HUMAN):
| Row | Clause | Screenshot | Driver note | Looker answer |
|---|---|---|---|---|
| D6a | Ogilvie glyph visible at 4452 | lab_screenshot-20261103-064516.482.png | not viewed by driver | not run |
| D6b | Dispenser glow visible at 4453 | lab_screenshot-20261103-064700.846.png | not viewed by driver | not run |

RESUME HINT: none needed; both rows' server clauses are final.
