PACKET SP-DHA2-p1 BLOCKED 2026-11-03 by the lane-runner (no driver run: the fixture cannot be staged); server evidence: local lab server log (dr-lab logs/server.log), 15:39:59Z

| Row | Grade | Tier | Server evidence | Screenshot | Issue |
|---|---|---|---|---|---|
| HA-1360b b1 (relog keeps Marsh's rebind) | BLOCKED | n/a | Fixture `world Harset_CmdCenter, 1360@4038, item 3730 in "mission"` was staged (stage call ok, player 107 Fxdcafour) and the Play applied it: `lab fixture not applied`, reason `write_failed`, error `mission 1360 was not written: stored None, wanted Some((1, Some(4038), [4037], [4650], 0))` at 15:39:59Z. Character loaded in Castle (world 8) with no 1360. | none | SP-DL1-1 (cited; this adds the 4038 shape) |
| HA-1360a a1 (letter hand-in, dialog 4576) | BLOCKED | n/a | Same refusal: the mission the hand-in needs cannot be put at 4038 with active objective 4650. No driver was run; a hand-in without the mission would not be a valid test. | none | SP-DL1-1 |

## Not run
Everything: b1 relog, a1 hand-in, the non_gm check and the Marsh icon NEEDS_HUMAN item. No NEEDS_HUMAN screenshots exist.

## Resume hint
Once the fixture writer can stage 1360 (step 4038, completed step 4037, active objective 4650) run the brief unchanged: character any Praxis Soldier, world Harset_CmdCenter, position [-45.5, -0.59, -14], items 3730 in container "mission".

## Findings
1. Same root cause as results/issues/SP-DL1-1.md (`persist_active_letter_mission` accepts only the active snapshot). The 4038 shape (`completed [4037]`, `active [4650]`) is refused the same way; the evidence is appended to that issue. No new issue file.
