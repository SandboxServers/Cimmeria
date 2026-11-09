PACKET SP-DCB4-p4 VERIFIED 2026-11-09 by Sonnet; driver attempts: 1; server evidence window 09:49:20-09:51:53Z (login 09:48:48Z)
| Row | Result | Tier | Server evidence (time, scope, event, key fields) | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| T05/T06b | PASS | N1 click, G moves | player 110 (Uat Fxdhajaffa, Jaffa archetype 8, acct lab2). Login 1 (entity 43): no 329 set at login (638 not_active); 09:49:30.575 `accepting mission` 638 chain 1012 once, step 2114; 09:49:30.577 `adding dialog set` 5866 / dialog 5021 chain 1012 (one set). 09:49:55.916 `interact: per-player dialog set -> onDialogDisplay` dialog 5021 target 101208. 09:50:08.122 dialogButtonChoice -1 on 5021, `advancing step` chain 1014 to 2115 (once). Relog 09:50:24 (entity 44): journal restored active 638:2115; 09:50:24.560 `adding dialog set` 5866 / 5021 chain 1027 (Restore ... Jaffa), exactly one, no second set, no 638 re-accept. 09:51:27.340 interact -> onDialogDisplay 5021 target 101257. No dialog 2300 anywhere. | DialogWin "Prisoner 329" both times (driver text) | skipped (no screenshots) | Server clauses all held; one set per login/relog, 5021 not 2300, 638 accepted once and at 2115 on relog. UNEXPECTED WARN 09:50:19.979 abilities `tree_missing` (archetype 8 Jaffa has no ability-tree rows): not part of this row; likely a known seed gap, no issue filed. |
SIGN-OFF SHEET (NEEDS_HUMAN): none (driver took no SHOT; H-T05-1 not captured).
FINDINGS: none filed. (Observation: WARN archetype_has_no_tree for Jaffa on relog, outside row scope.)
NOT RUN / BLOCKED: none.
RESUME HINT: none.
