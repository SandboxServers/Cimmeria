PACKET SP-DHA2-p4 BLOCKED 2026-11-03 by the lane-runner (no driver run: every row depends on mission 1360, which the fixture cannot stage)

| Row | Grade | Tier | Server evidence | Screenshot | Issue |
|---|---|---|---|---|---|
| HA-1360a a5 (refusal at the wrong step 4037) | BLOCKED | n/a | The fixture needs `1360@4037` with item 3730. results/issues/SP-DL1-1.md records that exact shape refused at 14:16:55Z, 14:18:05Z and 14:22:57Z (`mission 1360 was not written: stored None, wanted Some((1, Some(4037), [], [], 0))`), and the 4038 shape refused again at 15:39:59Z and 15:43:46Z today by the same writer. Not re-staged for p4 (no server change since). | none | SP-DL1-1 (cited) |
| HA-1360a a6 (out of range, 12 m) | BLOCKED | n/a | Same fixture; the range leg runs on the same character after a5. | none | SP-DL1-1 (cited) |

## Not run
Everything (no driver).

## Resume hint
Once the fixture writer can stage 1360, run the brief unchanged.

## Findings
1. Cascade of SP-DL1-1; no new issue.
