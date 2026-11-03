PACKET SP-DHA1-p8 VERIFIED 2026-11-03 by Sonnet; driver attempts: 4 (a1-a3 Haiku, a4 Sonnet takeover); server evidence window 15:34:07Z-15:39:35Z (local lab server log, dr-lab logs/server.log, plus lab-server DB read; SigNoz not used)
| Row | Grade | Tier | Server evidence | Screenshot | Issue |
|---|---|---|---|---|---|
| H41b leg 3 (baskets) | NATIVE_SHORTFALL (server clauses held; step 2505 reached) | FirstBug, ThirdBug: N1 (real world right-click, hover_verified); SecondBug: N3/N2 (client_target stock-Lua fallback + /interact) | player 111 Fxdhagoauld, mission 742. 15:34:22.923 fire_interact_tag matched tag FirstBug (2 actions); chain 6104 complete objective 2913 and RemoveItem 2820 (stack 3->2). 15:35:10.346 fire_interact_tag matched tag ThirdBug (2 actions); chain 6106 complete objective 2915 and RemoveItem 2820 (2->1). 15:39:08.762 fire_interact_tag matched tag SecondBug (4 actions); chain 6108 "advancing step" 2505 (Report to Anat), step_anchor advancer chain 6110, add dialog set 3130 (Anat, template 43, dialog 2639); chain 6105 complete objective 2914 and RemoveItem 2820 (1->0). DB public.sgw_mission player 111 mission 742: status 1, current_step_id 2505, completed_step_ids [2502,2503,2504], completed_objective_ids [2911,2912,2913,2915,2914], active_objective_ids [2916]. Entities with the three tags existed (interact matched for each; no separate server_entity_query run). Each tag fired once in the window. | H41-after-baskets: lab_screenshot-20261103-093512.978.png (NEEDS_HUMAN, not looked at) | none (finding 1 is benign) |
| H41b leg 4 (abandon) | BLOCKED | n/a (would be G) | Not run. /gmmissionclear arrives with an empty mission_arg: results/issues/SP-DHA1-p1-1.md (known issue, not retried). No Mission abandoned, no chain 6120 row in the window. | none | SP-DHA1-p1-1 (existing) |

Row H41b overall: PARTIAL (leg 3 held at NATIVE_SHORTFALL, leg 4 BLOCKED).

## Not run
- Leg 4 (abandon, chain 6120, dsm 3130 -> 3127 slot 43, Anat offer dialog 2636): BLOCKED by SP-DHA1-p1-1.
- Chain 6111 (Nerus) not run by design. Looker pre-screens not run per coordinator.

## Resume hint
Leg 3 needs no redo for server clauses. If a person wants an all-N1 pass, redo SecondBug (306) with a native right-click: a1/a2 failed with hover_timeout (read_mouse_over transport timeout, hover_probe "nothing" at all points on entity 100186), a harness gap. Leg 4 resumes from mission 742 step 2505 once SP-DHA1-p1-1 is fixed: .bug start, /gmmissionclear 742, check chain 6120, click Anat (template 43).

## Recipe
Carry-over at 742 step 2504, world 57 (Harset). FirstBug: .gotolocation Harset -176.7 -41.25 121.3, right-click template 164 (entity 100120) at 3.97 m. ThirdBug: .gotolocation Harset -147 -41.28 100.5, right-click entity 100187 at 4.0 m (native_camera). SecondBug (works only via fallback): stand at Harset -186 -41.28 114.5, camera pitch -10 facing entity 100186 at 4 m, client_target on 100186 (stock targetUnit), then chat line /interact. The all-N1 SecondBug click is the open harness gap. Note the last basket fired is SecondBug, so chain 6108 (not 6107/6109) did the advance.

## SIGN-OFF SHEET
- H41-after-baskets: is the player in a cave/tunnel area, and is any dialog open? Driver: screenshot after the first and third basket (SecondBug not yet done at the time). Screenshot lab_screenshot-20261103-093512.978.png. Looker: not run. NEEDS_HUMAN. Note it predates the SecondBug interact (15:39:08), so it cannot show the step-2505 state.
- H41-offer-back: not captured (leg 4 blocked).

## Findings
1. Row H41b leg 3, WARN (unexpected, benign): 15:39:08.764945 "complete_objective: objective not on the current step's roster - no-op" for objective 2914 (current step already 2505). Chains 6108 (advance step, adds dsm 3130) and 6105 (complete 2914, remove item) co-fire on interact_tag SecondBug and 6108 ran first, so 6105's complete_objective found the step already advanced. The DB nonetheless lists 2914 under completed_objective_ids and the item was consumed, so the end state is correct; only an ordering warning. No issue file; mention to the chain author if WARN-free runs matter.
2. Harness gaps (no issue): client_world_click hover_timeout on SecondBug (a1, a2); client_target missing from the Haiku lab-driver tool set (a3); a4 recipe lacked the verify-click-recipe doc. SecondBug interact therefore ran at N3/N2.
3. Unrelated WARN/ERROR at 15:39:42-15:40:30 belong to another character (Fxdcafour, player 107, fixture for a different packet) and are out of this packet's scope.
