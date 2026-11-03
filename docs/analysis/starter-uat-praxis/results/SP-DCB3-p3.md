PACKET SP-DCB3-p3 VERIFIED 2026-11-03 by Sonnet; driver attempts: 1; grade NATIVE_SHORTFALL
Re-drive with repaired briefs (runner-DCB3-b, local lab, character Dcbdh). Grades are the verifier grades per part; the runner report does not itemise server timestamps, so rows without a time say so.

| Row | Result | Tier | Server evidence (time, scope, event, key fields) | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| CB-U10b repeat click | PASS | N3 | no second `accepting mission` 62050 and no second `displaying dialog` 60150 | n/a | n/a | |
| CB-U10b relog | PASS | n/a | 62050 step retained, no new offer | n/a | n/a | |
| CB-U10b vial (tier G) | PASS | G | `/gmgiveitem 19 1` then `/gmmissionadvance 639 2343`, then the item use: chain 1034 completes 639 and accepts 640 | n/a | n/a | Tier G route; the native vial click is not drivable (issue SP-DCB3-p3-1) |
| CB-U10b ring switch | NATIVE_SHORTFALL | N3 | switch interact fell back to N3: StartMinigame, Livewire win, 640 advanced to step 2215 | `client_minigame_play` read `server_unreachable` on the win; the server recorded the victory | n/a | The harness shortfall was expected to block, but the INTERACT recipe worked at N3 |
| CB-U10b 640 completes | NOT-RUN | n/a | 640 not completed: chain 1044 (`teleport_in` region 2) not driven | n/a | n/a | The brief now adds a G-tier GOTO into Region2 (.14, INFERRED: not proven that `.gotolocation` raises `teleport_in`) |

Row grade: CB-U10b NATIVE_SHORTFALL. Repeat click, relog and vial held; the ring switch ran at N3; 640 stayed at step 2215.

SIGN-OFF SHEET (NEEDS_HUMAN): none (the minigame images are not graded; see SP-DCB3-p1).

FINDINGS:
(none new) The native-hover gap on small props (vial, ring switch) stays as the harness issue in docs/analysis/starter-uat-praxis/results/issues/SP-DCB3-p3-1.md.

NOT RUN / BLOCKED:
- 640 completion (chain 1044). SP-DCB3-p4 therefore records the "62050 completes" clause as C5-BLOCKED.

BRIEF DEFECTS (fixed in this change):
- .1: the recipe entity id (102307) is stale; the brief now FINDs Prisoner 329 first (the re-drive found 103140).
- Start state is 639 at step 2117, not 2145. The `/gmmissionadvance 639 2343` in .6 still works.
- .7: item 19 is a type id; the client instance id is 10599 (match by name).
- .13: the Livewire win reads `server_unreachable` although the server records victory; grade from the server.
- New .14 (INFERRED): GOTO -265 65.45 -135 so chain 1044 completes 640; the end bookmark is now .16.

RESUME HINT: Rerun only the new .14 GOTO if you need 640 completed; do not repeat .1 to .13.
