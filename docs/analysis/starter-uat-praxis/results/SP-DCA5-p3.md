PACKET SP-DCA5-p3 VERIFIED 2026-11-03 by Sonnet; driver attempts: 2 (attempt 1 had the native click and target fail, a lab harness gap; attempt 2 clicked from a spot off the avatar line); server evidence window 13:30:46Z-13:36:09Z (local lab server, run dr-lab)

Substitution: instance p2; Fxdcathree (player 106, GM) replaces Uat FxDcaffb. Looker skipped; screenshots by file name only.

| Row | Result | Tier | Server evidence | Client evidence | Notes |
|---|---|---|---|---|---|
| FZ-U1b U4 (cabinet reachable, Romney idle) | PASS | G (.aggro off, .gotolocation), N1 click | 13:30:46 npc_ai.aggro gm_toggle aggro_off=true for player 106, before the 13:35:25 teleport to (232.0,66.79,1031.5). No Romney aggro or combat row in the window. | hover entity, in_interact_range true | verifier notes the cabinet position in the DB may differ from the brief (232.0 vs 227.5 x); fixture position noted for brief fix |
| FZ-U1b U5 (cabinet clicks, chains 1397/1400) | PASS | N1 | 13:35:42.6 interact target 100390, fire_interact_tag Castle_FileCabinet, chain 1397 granted item 2698, mission 567 step 2000 -> 2012, dialog 10733. 13:36:03.9 chain 1400 dialog 10734, no grant. DB: exactly one 2698 row (client item 10627, container 2). | SplashTextWin + DialogWin, then DialogWin with the second text | attempt 1: interact reached the server with target_entity_id 0 (client never sent setTargetID): harness gap, not a product defect |

FINDINGS: none. No WARN or ERROR for player 106 in the window. No issues filed.
NOT RUN / BLOCKED: none. `.aggro on` is sent in SP-DCA5-p4.

SIGN-OFF SHEET
FZ-cabinet-1 | lab_screenshot-20261103-073548.925.png | dialog, one short paragraph; item shows the IconMissing placeholder | looker skipped | cannot tell (placeholder icon is a look-and-feel item)
FZ-cabinet-2 | lab_screenshot-20261103-073605.327.png | second dialog, "nothing else of use" | looker skipped | cannot tell
