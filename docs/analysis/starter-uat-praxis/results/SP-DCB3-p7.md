PACKET SP-DCB3-p7 RUN 2026-11-09 by runner-DCA-t (Haiku driver a1 + Sonnet verifier); grade PARTIAL
Local lab p2, Fxdcatwo (Soldier, fixture 689 completed, 687 at step 2355, at (-130,24.7,-110)). Driver: .claude/reports/starter-uat-praxis/drv-SP-DCB3-p7-a1.md. Server evidence: srv-SP-DCB3-p7.log (08:56:24-08:57:44Z).

| Row | Grade | Tier | Server evidence | Screenshot | Issue |
|---|---|---|---|---|---|
| CB-U11b no line during 2354 | NOT-RUN | G | fixture staged 687 straight at 2355 | none | none |
| CB-U11b activation | NATIVE_SHORTFALL (+ NEEDS_HUMAN H-CB-U11-1) | G | 08:56:39.531Z npc bark chain 11020 screen 200170 once, from login-time region replay (already_inside_on_step_activation), not walking in; driver chat read empty (gap) | none | none |
| CB-U11b no repeat | PARTIAL | G | no second chain 11020 row to 08:57:44Z; no re-entry or relog run | none | none |
| CB-U11b fight unchanged | PARTIAL | G | start bookmark lists G25/G26/G36 only; only G26 killed (/gmkilltarget 100830, 08:57:23.8Z) | lab_screenshot-20261109-025741.885.png (H-CB-U11-2, not viewed; only one guard died) | none |
| CB-U11b after | NOT-RUN | none | 687 not completed; no screen_not_cached WARN; unexpected WARNs 08:57:25.457Z (chain 1073, mission 680 not found, 681 accepted at 2348), likely fixture-induced, unconfirmed | none | none |
| CB-U09 | NOT-RUN | none | timing only; GM shortcuts used | none | none |

Position jump explained: the player was killed by a NID Guard (ability 559) at 08:56:54.9Z, about 6 s before the start bookmark; the driver ran on a corpse; respawn at 08:57:25Z reanchored to (-79.2,45.2,-163.8).

## Brief defects
- verify names Dcbdh/Commando; fixture was Fxdcatwo/Soldier.
- FIND max_distance_m 10 cannot find the guards (G26 7.1 m, G25 about 14.3 m, G36 about 25.7 m); use 30 m.
- Staging a level-1 player 7 m from hostile guards contradicts gotcha 5 (stand 4-6 m off); player dies in about 15 s.
- start bookmark comes after GOTO and wait, so it misses the death.
- Nothing tests "after 687 completes" or relog repeat though verify grades them.

## Resume hint
Stage outside aggro range or kill G25/G26/G36 by /gmkilltarget within seconds of login (find at 30 m), read chat for the bark, relog for no-repeat, complete 687 and check for screen_not_cached WARN. Check chain 1073 gating on 680.

---
ATTEMPT 0 (earlier notes by runner-DCA-s, coordinator-graded, not Sonnet verified):

PACKET SP-DCB3-p7 RUN 2026-11-09 by runner-DCA-s (coordinator, no Haiku driver, no Sonnet verify); grade PARTIAL
Local lab instance p2, character Fxdcafour in the barracks with 687 at step 2355.

| Row | Grade | Tier | Server evidence | Screenshot | Issue |
|---|---|---|---|---|---|
| CB-U11b target guard natively | PASS | N1 | client_target left-click on NID Guard 100634 confirmed Unit.Target = 100634 (native_camera) | none | none |
| CB-U11b guard kill (spawn 26) | PASS (deviation) | G | /gmkilltarget 100634 -> "gmKillTarget: killed npc 100634"; entity_get ai_state Dead, health 0, spawn_id 26, tag Barracks_Guard2 | none | none |
| CB-U11b guards 25 and 36 | PARTIAL | G | /gmkilltarget sent for 100633, 100643, 100636; DB shows 687 still at 2355 (no completion expected until all guards die); per-spawn ids not read back | none | none |
| CB-U09 | NOT-RUN | n/a | timing only, per brief | none | none |

## Not run
Spawn ids 25 and 36 not individually confirmed; no combat-based kill (tier G only); step 2355 completion not observed.

## Resume hint
Read entity_get for 100633, 100636, 100637, 100643 spawn_id and ai_state; /gmkilltarget needs an entity id argument (bare form returns Not enough parameters).

## Recipe
Brief defect: the brief uses bare /gmkilltarget; it requires the id.
