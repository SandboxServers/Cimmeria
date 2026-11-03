PACKET SP-DCA5-p4 VERIFIED 2026-11-03 by Sonnet; driver attempts: 2 (a1 crossing; a2 coordinator relog completion after a1's relog hit an empty character list, a client-side lab harness gap, recovered by lab_client_restart on p2); server evidence window 13:38:26Z-13:44:40Z (local lab server, run dr-lab)

Substitution: instance p2; Fxdcathree (player 106) replaces Uat FxDcaffb. Tier G throughout (typed GM lines). Looker skipped.

| Row | Result | Server evidence | Notes |
|---|---|---|---|
| FZ-U1c U6 crossing | PASS (tier G) | 13:38:35 gmDHD dial accepted; 13:38:51.18 stargate crossing, fire_stargate_crossed matched (dest Harset), chain 10241, 567 step 2012 -> 4039; 13:38:52-58 world transition, player entered world 57. DB: 567 step 4039, completed [2000,2012], status 1. | client_move_to aborted as a teleport (expected) |
| FZ-U1c U3 relog restores | PASS (tier G) | logout 13:39:57.68; relog 13:43:44; 13:43:52.18 missions_restored active "567:4039", world Harset; session in_world | chain 6503/6504 produced no log row, so only the step restore is verified |
| Aggro switch | PASS | 13:44:30.08 gm_toggle aggro_off=false (sent in a2) | |
| Turn-in | BLOCKED | needs world 68, no route (as the brief states) | |
| U2, U7, U8 | NOT-RUN | not driven by this packet | |

FINDINGS (none filed):
1. a1 relog failure: the server sent a 5-character list at 13:39:59; the empty list was client-side (harness gap).
2. 13:38:35 WARN gmDHD "caller does not hold this stargate address", expected for the GM dial.
3. 13:38:52 two WARN npc_ai "chasing but has not closed on its target" (Castle standoff Jaffas); NPC noise.
4. 13:43:21 WARN "Minigame result notification dropped" for player 107, another character, unrelated.
5. Low-priority candidate: no log event shows the 6504 turn-in rebind after the relog.

SIGN-OFF SHEET: none (FZ-crossed = lab_screenshot-20261103-073944.536.png; server evidence settles every clause).
