PACKET SP-DHA3-p1 BLOCKED 2026-11-03 by the lane-runner (fixture applied; the Kaerlia click cannot be landed by the lab harness); drivers: Haiku a1 (.claude/reports/starter-uat-praxis/drv-SP-DHA3-p1-a1.md), Sonnet takeover a2 (drv-SP-DHA3-p1-a2.md); server evidence: gmMissionList replies and local lab log

| Row | Grade | Tier | Server evidence | Screenshot | Issue |
|---|---|---|---|---|---|
| FZ-U2a Fixture applied | PASS | G | gmMissionList at login: `#1664 active step Some(5224) obj 0/1` (seq 330211); fixture staged for Fxdcatwo (player 105), world 57 | none | none |
| FZ-U2a Kaerlia click -> 5225, chain 6008, dialog 6169 | BLOCKED | n/a | Never reached: client_world_click hit hover_timeout twice (Haiku) and client_target, camera turns, Lua targetUnit and `/interact` all failed for the takeover although client_entity_find shows Kaerlia rendered and targetable at 4 m. Mission stayed at 5224 after every attempt (seq 330904, 333084). No click reached the server, so no chain 6008 row exists. | A-DIALOG screenshots show no dialog | none (lab harness gap, like SP-DHA1-p8 SecondBug hover) |
| FZ-U2a Step 5225 survives relog (chain 6015) | BLOCKED | n/a | Depends on the click. Relog by the Haiku run restored 5224 with the fixture state, as expected without a click. | none | none |
| FZ-U2a Name plate, facing, cue, tracker line | NEEDS_HUMAN | n/a | server clauses not reached | lab_screenshot-20261103-094558.955.png (A-KAERLIA-BEFORE), lab_screenshot-20261103-094637.951.png (A-TRACKER) | none |

## Not run
Chain 6008 click, step 5225 check, relog restore of the door cue, takeover relog.

## Resume hint
Retry when the hover/target path works on p2 (client may be CPU-starved), or with a small reposition. Re-stage first only if the mission advanced.

## Findings
1. Hover/target failure on a rendered NPC is a harness gap; no product issue filed.
