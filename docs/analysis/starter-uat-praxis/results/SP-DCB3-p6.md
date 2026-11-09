PACKET SP-DCB3-p6 RUN 2026-11-09 by runner-DCA-t (Haiku drivers a1-a3 + Sonnet takeover of the crate step + Sonnet verifier); grade PARTIAL
Local lab instance p2, character Fxdcafour (fresh staged fixture 689 completed, 687 at step 2354, at (-110,34.6,-95)). Driver files: .claude/reports/starter-uat-praxis/drv-SP-DCB3-p6-a1.md, -a2.md, -a3.md. Server evidence: dr-lab logs/server.log 08:49-08:56Z, excerpt srv-SP-DCB3-p6.log.

| Row | Grade | Tier | Server evidence | Screenshot | Issue |
|---|---|---|---|---|---|
| CB-U11a no bark during 2354 | PASS | n/a | no chain 11020 bark before 08:53:58Z; two logins during 2354 gave none | none | none |
| CB-U11a crate click advances 687 to 2355 and bark | PASS (+ NEEDS_HUMAN 1) | N1 after camera reset (pitch -10, face crate); Haiku a1 native click failed twice (hover_timeout, then range); a2 Haiku lacks client_target; Sonnet takeover N1 worked first try | 08:53:58.285Z Content: npc bark entity 37 screen 200170 chain 11020 once, 2 ms after advancing step 2355 | lab_screenshot-20261109-025545.581.png (H-CB-U11-1, driver did not view; a1 shot lab_screenshot-20261109-025211.017.png) | none |
| CB-U11a no repeat on re-entry and relog | PASS | n/a | re-entry 08:54:36-08:54:45Z no bark; one bark at 08:55:00Z during relog login restore (row allows at most one) | none | none |
| CB-U11a fight unchanged (spawns 25, 26, 36 only) | PARTIAL | n/a | chain 11020 has no spawn action (seed read); entity count not read live | none | none |
| CB-U11a after (no row after 687 completes, no screen_not_cached WARN) | PARTIAL | n/a | no screen_not_cached WARN or ERROR in window; 687 not completed here | none | none |

## Brief defects
- verify file names Dcbdh; this run used a staged Fxdcafour.
- Brief GOTO -130 24.7 -110 is about 18 m from the crate; use -130 24.7 -96.5.
- Native right-click needs a camera reset (client_camera pitch -10 facing the crate) before it works; not in the brief.
- client_target is not in the lab-driver tool list (recipe unusable by Haiku).
- Crate entity id changes per login; unnamed entity (level -1, hostility 3) at (-130.45,24.67,-92.07).
- server_stage_fixture world name is Castle_CellBlock (capital B).

## Resume hint
Fight-unchanged: run server_entity_query in the player space right after the crate loot, before logout.

---
ATTEMPT 0 (earlier notes by runner-DCA-s, coordinator-graded, not Sonnet verified):

PACKET SP-DCB3-p6 RUN 2026-11-09 by runner-DCA-s (coordinator takeover after 2 failed Haiku driver attempts; no Sonnet verify); grade PARTIAL
Local lab instance p2, character Fxdcafour (staged fixture, mission 687 at step 2354, spawn in Castle_CellBlock). Server log: dr-lab logs/server.log.

| Row | Grade | Tier | Server evidence | Screenshot | Issue |
|---|---|---|---|---|---|
| CB-U11a barracks entry during step 2354 (no bark expected) | PASS | N2 | no chain 11020 bark between 08:41:12Z and 08:43:10Z while 687 was at 2354 | none | none |
| CB-U11a crate click advances 687 to 2355 | NATIVE_SHORTFALL | N1 attempted | native right-click on crate 100622 failed twice (hover_timeout, mouse_over was the crate); client_target left-click also did not set a target; loot_all says loot window not open | lab_screenshot-20261109-024152.247.png | brief defect, see runner report |
| CB-U11a advance to 2355 | PASS (deviation) | G | /gmmissionadvance 687 2355; sgw_mission 107/687 current_step_id 2355, completed_step_ids [2354] | none | none |
| CB-U11a bark on entry during 2355 | PASS | G | 08:45:31Z Content: npc bark chain 11020 screen 200170 speaker NID Guard, once; player was already inside Region12 (-130,24.7,-96.5) when the step advanced | none | none |
| CB-U11a re-entry no repeat | PASS | G | after .gotolocation out (-110,34.6,-95) and back (-130,24.7,-110): count of screen_id 200170 barks still 1 | none | none |
| CB-U11a relog persistence | NOT-RUN | n/a | not driven (budget) | none | none |
| CB-U11a no extra guard | NOT-RUN | n/a | not counted | none | none |

## Not run
Relog persistence, extra guard count.

## Resume hint
Fxdcafour holds 687 at 2355. Re-run relog on p2 (logout, Play, confirm no bark and 2355 persists).

## Recipe
Brief defect: the crate step stood 18 m from the crate; GOTO -130 24.7 -96.5 (4.4 m) fixes range. Chat lines via CHATX with Inst1Chat_Input.
