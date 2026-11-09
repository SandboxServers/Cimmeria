PACKET SP-DCB4-p5 VERIFIED 2026-11-09 by Sonnet; driver attempts: 1; server evidence window 09:55:20-09:59:00Z (character Fxdlone, player 108; the brief says Dcbeh)

| Row | Result | Tier | Server evidence | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| T07 Livewire | NATIVE_SHORTFALL | N3 (client_target) + N2 (/interact), Livewire via client_minigame_play N1 | 09:56:51 `interact` target 101317 (329_CellDoorButton); `Content: starting minigame` chain 1016 Livewire; 09:56:52 `Minigame started` room 1002; 09:56:58 `Minigame victory` (validation=server), `Minigame result` result_code 1, chain 1017 | smartfox_victory true | none (no looker) | Cancel/timeout not exercised before the win; the win is present. Native click on the panel failed (BLOCKED-known). |
| T07 Timeout retry | NOT RUN | n/a | no timeout in the window; two post-victory `Minigame cancel handled canceled:false` (player_closed_window), benign | n/a | n/a | Not run, so PARTIAL for the row as a whole. |
| T07 Follow-up | NATIVE_SHORTFALL | N3 + N2 | 09:56:58 `advancing step` 638 -> 2116 (chain 1017). 09:58:17 interact Prisoner 329, dialog 2300 "Free Prisoner 329". 09:58:29 `displaying dialog` 2299 (chain 1021, Human). 09:58:30 `displaying dialog` 2298 (chain 1019), `accepting mission` 639, `completing mission` 638, dialog sets added (CB-N05, CB-N06) and 2794 removed. DB: 638 status 2, completed steps 2114/2115/2116, repeats 1. 639 status 1, step 2117. | DialogWin text read, 5 pages | none | Jaffa 5020 not exercised (Human path only). |

OVERALL T07: PARTIAL (Livewire win and follow-up chain verified on the server at NATIVE_SHORTFALL tier; timeout retry not run; Jaffa path not run).

Dialog text note: "You feel uncomfotable..." is the opening topic 2300 (Free Prisoner 329). It is not a missing follow-up. Finishing 2300 fired chain 1021 which showed 2299, then 2298 completed 638 and accepted 639. The staged step 2115 was advanced to 2116 by the Livewire win (chain 1017) before the prisoner was spoken to.

SIGN-OFF SHEET (NEEDS_HUMAN):
- T07, H-T07-1, door actually opening: not captured. Driver screenshots client_minigame_play-20261109-035658.663.png and .666.png (minigame window) and lab_screenshot-20261109-035745.470.png (taken after the prisoner click, door state not examined). Server ran sequence 1749 Prisoner329CellDoor at 09:56:58.

FINDINGS: none. No WARN or ERROR in the window. No product bug, so no issue body.

NOT RUN / BLOCKED:
- Timeout retry (step 2114 topic after a timeout and after a relog): not run. Needs a fixture reset to 638 step 2114 (or 2115), then let the Livewire window time out or cancel, re-open the door panel, and also relog.
- Jaffa follow-up 5020: needs a Jaffa character (case c, scope).
- Native clicks on 101317 and 101306 failed at observe with mouse_over 0 (BLOCKED-known). The panel is "Door control panel", tag 329_CellDoorButton.
- H-T07-1 door-open screenshot after the 09:56:58 win.

RESUME HINT: for full PASS, redo T07 with N1 clicks if possible, run one timeout/cancel before the win, relog and retry, and screenshot the open door. The driver's `fields` array arg was passed as a string by the harness (harness gap, case c).
