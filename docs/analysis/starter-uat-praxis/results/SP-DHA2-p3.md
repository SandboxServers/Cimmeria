PACKET SP-DHA2-p3 BLOCKED 2026-11-03 by the lane-runner (no driver run: the fixture cannot be staged); server evidence: local lab server log (dr-lab logs/server.log), 15:43:46Z

| Row | Grade | Tier | Server evidence | Screenshot | Issue |
|---|---|---|---|---|---|
| HA-1360a a4 (refusal without the letter) | BLOCKED | n/a | Single fixture check: `world Harset_CmdCenter, [-45.5,-0.59,-14], 1360@4038, no items` staged for a Praxis Soldier (player 107), Play applied it: ERROR `lab fixture not applied`, reason `write_failed`, error `mission 1360 was not written: stored None, wanted Some((1, Some(4038), [4037], [4650], 0))` at 15:43:46Z. Character loaded in Castle (world 8). A refusal test needs 1360 at 4038 without the letter; without the mission the click would prove nothing. | none | SP-DL1-1 (cited) |

## Not run
Everything (no driver): a4 and the Marsh refusal screenshot.

## Resume hint
Once the fixture writer can stage 1360 at 4038 (active objective 4650), run the brief unchanged.

## Findings
1. Same root cause as results/issues/SP-DL1-1.md; no new issue.
