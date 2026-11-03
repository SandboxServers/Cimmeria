# DR-L4-scientist: Scientist abilities (armory-abilities-scientist)

> **Type:** results. **Audience:** the orchestrator and architect. **Verified:** 2026-11-03, read-only (run bundle, SigNoz `cimmeria.lab_run = 'dr-lab'`, lab-server `server_db_query`). **Build:** `service.version` 6b50d9131417b8cf93feb61a7c640577398db567, attested for this run and for commando-2 (see Attests). Method: [lab-run-plan.md, Verifier steps](../lab-run-plan.md), [DR-L3-commando-1.md](DR-L3-commando-1.md).

**Recommendation: findings, then rerun after a seed fix.** 25 of 60 rows PASS on server truth (incl. setup). 27 are FAIL (24 refused `no_mechanics`, 3 turret repairs refused `owner_pet_refused`), 5 PARTIAL (3 passives, 1005 empty gun, 1012 one `deployable_refused`), 3 BLOCKED (997 DATA-GAP, 1223 and 1224 press ignored). The runner called 45 PASS (it counts the session-end row), 13 FAIL, 3 BLOCKED; 17 of its PASS rows had the client `applied` clause failed (as reported by the runner), and 18 runner-PASS rows are server refusals.

## Runs

| Call | Spec or split file | plan_only | run_dir | Outcome |
|---|---|---|---|---|
| 1-11 (6 rows each, `end_session: false`, then session end) | armory-abilities-scientist | no | `C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqrovhriypya` | runner PASS 45, FAIL 13, BLOCKED 3 |

Window 23:21Z to 00:01Z, fresh character Cgbqrovhriypyahi (Scientist).

## Per-row grades

Server truth = SigNoz scope `abilities` over the window, one aggregate by `ability_id`, `event`, `reason` (every event twice: each row presses twice). Seed from `resources.abilities`, `resources.effect_nvps`.

| Row | Ability | Runner | Server evidence | Grade | Class |
|---|---|---|---|---|---|
| Scientist-00 | setup | PASS | world 1300 reached | PASS |  |
| Scientist-1000 | Dart Rifle: Multi-Shot | PASS | ability_launched x2 (or beneficial_cast_applied) | PASS |  |
| Scientist-1013 | shield area | PASS | ability_launched x2 (or beneficial_cast_applied) | PASS |  |
| Scientist-1016 | toggle | PASS | ability_launched x2 (or beneficial_cast_applied) | PASS |  |
| Scientist-1017 | toggle | PASS | ability_launched x2 (or beneficial_cast_applied) | PASS |  |
| Scientist-1018 | toggle | PASS | ability_launched x2 (or beneficial_cast_applied) | PASS |  |
| Scientist-1235 | toggle | PASS | ability_launched x2 (or beneficial_cast_applied) | PASS |  |
| Scientist-1228 |  | PASS | ability_launched x2 (or beneficial_cast_applied) | PASS |  |
| Scientist-1229 | CC ground | PASS | ability_launched x2 (or beneficial_cast_applied) | PASS |  |
| Scientist-1230 | CC ground | PASS | ability_launched x2 (or beneficial_cast_applied) | PASS |  |
| Scientist-1220 |  | PASS | ability_launched x2 (or beneficial_cast_applied) | PASS |  |
| Scientist-1217 |  | PASS | ability_launched x2 (or beneficial_cast_applied) | PASS |  |
| Scientist-1474 |  | PASS | ability_launched x2 (or beneficial_cast_applied) | PASS |  |
| Scientist-1952 |  | PASS | ability_launched x2 (or beneficial_cast_applied) | PASS |  |
| Scientist-598 |  | PASS | ability_launched x2 (or beneficial_cast_applied) | PASS |  |
| Scientist-713 |  | PASS | ability_launched x2 (or beneficial_cast_applied) | PASS |  |
| Scientist-891 |  | PASS | ability_launched x2 (or beneficial_cast_applied) | PASS |  |
| Scientist-946 | heal | PASS | ability_launched x2 (or beneficial_cast_applied) | PASS |  |
| Scientist-948 | heal | PASS | ability_launched x2 (or beneficial_cast_applied) | PASS |  |
| Scientist-988 |  | PASS | ability_launched x2 (or beneficial_cast_applied) | PASS |  |
| Scientist-989 |  | PASS | ability_launched x2 (or beneficial_cast_applied) | PASS |  |
| Scientist-990 |  | PASS | ability_launched x2 (or beneficial_cast_applied) | PASS |  |
| Scientist-992 |  | PASS | ability_launched x2 (or beneficial_cast_applied) | PASS |  |
| Scientist-993 |  | PASS | ability_launched x2 (or beneficial_cast_applied) | PASS |  |
| Scientist-998 |  | PASS | ability_launched x2 (or beneficial_cast_applied) | PASS |  |
| Scientist-943 | Conversation: Mechanical: Basic | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (effects, 0 NVPs) |
| Scientist-944 | Conversation: Mechanical: Experienced | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (effects, 0 NVPs) |
| Scientist-945 | Conversation: Mechanical:Advanced | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (effects, 0 NVPs) |
| Scientist-962 | Summon Turret: Prototype | PASS | ability_refused no_mechanics x2 | FAIL | seed gap (effects, 0 NVPs) |
| Scientist-963 | Summon Turret: Offensive | PASS | ability_refused no_mechanics x2 | FAIL | seed gap (effects, 0 NVPs) |
| Scientist-964 | Summon Turret: Defensive | PASS | ability_refused no_mechanics x2 | FAIL | seed gap (effects, 0 NVPs) |
| Scientist-965 | Summon Turret: Detection | PASS | ability_refused no_mechanics x2 | FAIL | seed gap (effect_ids empty) |
| Scientist-966 | Summon Turret: Advanced | PASS | ability_refused no_mechanics x2 | FAIL | seed gap (effects, 0 NVPs) |
| Scientist-971 | Enhance Turret: Increased RoF | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (effects, 0 NVPs) |
| Scientist-972 | Enhance Turret: Energy Damage | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (effects, 0 NVPs) |
| Scientist-994 | Deployable: Stim Pack | PASS | ability_refused no_mechanics x2 | FAIL | seed gap (effects, 0 NVPs) |
| Scientist-1014 | Deployable: Detection | PASS | ability_refused no_mechanics x2 | FAIL | seed gap (effect_ids empty) |
| Scientist-1015 | Deployable: Gravity Well | PASS | ability_refused no_mechanics x2 | FAIL | seed gap (8 effects, 2 NVPs) |
| Scientist-1019 | Shield: Interruption | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (effects, 0 NVPs) |
| Scientist-1206 | Enhance Turret: Contamination Damage | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (effects, 0 NVPs) |
| Scientist-1212 | Enhance Turret: Shield | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (effects, 0 NVPs) |
| Scientist-1214 | Repair Turret: Restoration | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (effects, 0 NVPs) |
| Scientist-1215 | Dart Type: Hazardous: Interruption | PASS | ability_refused no_mechanics x2 | FAIL | seed gap (effect_ids empty) |
| Scientist-1225 | Deployable: Health Regen | PASS | ability_refused no_mechanics x2 | FAIL | seed gap (effects, 0 NVPs) |
| Scientist-1226 | Dart Type: Hazardous: Confusion | PASS | ability_refused no_mechanics x2 | FAIL | seed gap (effect_ids empty) |
| Scientist-1231 | Shield: Adaptive | PASS | ability_refused no_mechanics x2 | FAIL | seed gap (effects, 0 NVPs) |
| Scientist-1232 | Shield: Reflective | PASS | ability_refused no_mechanics x2 | FAIL | seed gap (effects, 0 NVPs) |
| Scientist-1236 | Deployable: Aggression Inducer | PASS | ability_refused no_mechanics x2 | FAIL | seed gap (effects, 0 NVPs) |
| Scientist-2955 | Deployable: Shield Generator | PASS | ability_refused no_mechanics x2 | FAIL | seed gap (effect_ids empty) |
| Scientist-967 | Repair Turret: Percentage | FAIL (applied) | ability_refused owner_pet_refused x2 | FAIL | dependency: no turret exists (962-966 refused no_mechanics); effect and NVP exist |
| Scientist-968 | Repair Turret: Regenerate | FAIL (applied) | ability_refused owner_pet_refused x2 | FAIL | dependency: no turret exists (962-966 refused no_mechanics); effect and NVP exist |
| Scientist-1207 | Repair Turret: Full | FAIL (applied) | ability_refused owner_pet_refused x2 | FAIL | dependency: no turret exists (962-966 refused no_mechanics); effect and NVP exist |
| Scientist-999 | Dart Type: Hazardous: EMP | PASS | no_mechanics_refused passive_ability x2 | PARTIAL | passive (design) |
| Scientist-1213 | Summon Turret: Dual Turrets | PASS | no_mechanics_refused passive_ability x2 | PARTIAL | passive (design) |
| Scientist-1227 | Dart Type: Hazardous: Contagion | PASS | no_mechanics_refused passive_ability x2 | PARTIAL | passive (design) |
| Scientist-1005 | Aimed Burst | FAIL (applied) | use_ability_no_ammo + ability_refused no_ammo x2; weapon_draw_queued x1 | PARTIAL | required_ammo 6, gun empty by design (R7 analogue); press reached server |
| Scientist-1012 | Deployable: Microwave Emitter | PASS | use_ability_on_ground_recv x2; ability_launched x1; ability_refused deployable_refused x1 | PARTIAL | one launch, one deployable_refused |
| Scientist-997 | Full Mag: Dart Pistol | BLOCKED | no events (not pressed) | BLOCKED | DATA-GAP: requires ITEM_Dart_Rifle, 0 items carry it; names look swapped with ITEM_DartPistol |
| Scientist-1223 | Deployable: Focus Regen | BLOCKED (press ignored) | no events at all | BLOCKED | ground-target (tt3) deployable, no effects; press ignored client-side |
| Scientist-1224 | Deployable: Med Station | BLOCKED (press ignored) | no events at all | BLOCKED | ground-target (tt3) deployable, no effects; press ignored client-side |

Totals: PASS 25, FAIL 27, PARTIAL 5, BLOCKED 3 = 60 rows. Rows the runner failed only on the client `applied` clause but the server launched are graded PASS.

## Findings

1. **Seed gap, 24 rows (real defect):** effects with no `effect_nvps` rows, or empty `effect_ids`; the gate refuses all `no_mechanics`. The turret family (962-966, 971, 972, 1206, 1212, 1214) is the largest block. Issue: [DR-L4-scientist-no-mechanics.md](issues/DR-L4-scientist-no-mechanics.md). One `seed-change` pass with the commando issues.
2. **Turret repair 967, 968, 1207:** effect and NVP exist; refused `owner_pet_refused` because no turret was ever summoned (all Summon Turret rows were refused). Dependent on finding 1.
3. **Passives:** 999, 1213, 1227 (`passive_yn` true), refused `passive_ability` by design. PARTIAL.
4. **997 DATA-GAP:** requires `ITEM_Dart_Rifle` (3257416555), 0 items carry it. 1000 "Dart Rifle: Multi-Shot" requires `ITEM_DartPistol` (25 items) and launched, so the monikers look swapped. Added to [DR-weapon-audit-fail.md](issues/DR-weapon-audit-fail.md) (created here; the DR-L3 rows 1246, 1250, 1355 are listed by reference and the DR-L3 owner must add them).
5. **1223 Focus Regen and 1224 Med Station:** ground-target deployables (`target_type_id` 3, `ABILITY_TYPE_DD`, empty `effect_ids`). The setup press was ignored by the client; zero server events. Other ground deployables (962-966, 1012, 1015, 1236) pressed fine, so the cause is not proven. Issue: [DR-L4-scientist-ground-press-ignored.md](issues/DR-L4-scientist-ground-press-ignored.md). If they send, expect `no_mechanics`.
6. **1005 Aimed Burst:** `required_ammo` 6; refused `no_ammo`, `weapon_draw_queued` seen; empty gun by design. PARTIAL. The run plan says only 997 has `required_ammo > 0`; 1005 does too. No issue.
7. **1012 Microwave Emitter:** one launch and one `deployable_refused` across the two presses. PARTIAL.
8. **Working:** 24 abilities launched or applied (darts, shield area 1013, toggles 1016-1018 and 1235, CC 1229/1230, heals 946/948, others). `fire_target_skipped no_target` appears on many; the launch still happens.

## Not run / UNVERIFIED

No PENDING clauses, no UNVERIFIED rows. 997 never pressed; 1223 and 1224 never sent.

## Rerun list

- After the seed fix (`seed-change`): the 24 no_mechanics rows, then 967, 968, 1207 (a Summon Turret row must succeed first).
- After the dart moniker data decision: 997.
- Rerun alone now: 1223, 1224 (press ignored); 1012 (second press).
- 1005 only after an empty-gun solution (DR-07c).
- No rerun for the 25 PASS rows or the 3 passives.

## Attests

Done 2026-11-03 00:02Z: lease free (last holder `lab_uat_run`, released 00:00:54Z). Took a 120 s lease, ran `lab_uat_attest {server_version: 6b50d9131417b8cf93feb61a7c640577398db567}` for commando-2 (`2026-11-02-unknown-cgbqrbvtowfscz`) and scientist (`C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqrovhriypya`), released at once. No clause attests pending.

## Recipe

SigNoz `signoz_aggregate_logs`, filter `service.name = 'cimmeria-server' AND cimmeria.lab_run = 'dr-lab' AND scope_name = 'abilities' AND player_name CONTAINS 'Cgbqrovhriypyahi' AND event EXISTS AND event != 'warmup_pending'`, start 1793661600000, end 1793664100000, count grouped by ability_id, event, reason. Seed: `server_db_query` on `resources.abilities` (`effect_ids`, `item_monikers`, `passive_yn`, `target_type_id`, `required_ammo`) with a count of `resources.effect_nvps` whose `effect_id` is in `effect_ids`; `resources.monikers` joined to `resources.items.moniker_ids`.
