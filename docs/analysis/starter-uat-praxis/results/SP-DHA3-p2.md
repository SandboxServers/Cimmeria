PACKET SP-DHA3-p2 BLOCKED 2026-11-03 by the lane-runner (fixture applied; the door control has no client name, so the brief cannot target it, and the hover path failed on the sibling part); driver: Haiku a1 (.claude/reports/starter-uat-praxis/drv-SP-DHA3-p2-a1.md); no takeover (same hover/target harness gap as SP-DHA3-p1)

| Row | Grade | Tier | Server evidence | Screenshot | Issue |
|---|---|---|---|---|---|
| FZ-U2b Fixture applied | PASS | G | gmMissionList at login: `#1664 active step Some(5225) obj 1/3` (seq 338875); character Fxdcathree, world 57 | none | none |
| FZ-U2b D-PU6 door control found at the placeholder | PARTIAL | G | client_entity_find at the stand point (0, -67.6, -231): unnamed entity 100206 at server (0, -67.6, -235), rendered, targetable, 4 m; it matches spawn 380 (the placeholder). It has no name, and two Praxis Jaffa Lieutenants stand 4.3 m off. Whether it is the door control and reachable is the owner's call. | lab_screenshot-20261103-095321.548.png (B-NOTFOUND, NEEDS_HUMAN) | none |
| FZ-U2b use cursor, click -> 5226 and World 68, chain 6012, `.location` pin, journal, relog (6033/6034) | BLOCKED | n/a | Not run: the brief clicks the control by name; the control is unnamed, and name or entity hovers failed in SP-DHA3-p1 on this instance (hover_timeout). | none | none |

## Not run
b3 to b6.

## Resume hint
Click by entity id (client_world_click with the id, or client_target after client_camera) when the hover path works; the brief's step b2 should say the control may be unnamed (id 100206 at the placeholder).

## Findings
1. Brief defect b2/b3: the control has an empty client name; the "name FIND returned" route cannot work. Lab harness gap, no product issue filed.
