# DR-L5-archeologist-1: Archeologist abilities, tree 1 (armory-abilities-archeologist-1)

> **Type:** results. **Audience:** the orchestrator and architect. **Verified:** 2026-11-03, read-only (run bundle, SigNoz `cimmeria.lab_run = 'dr-lab'`, lab-server `server_db_query`). **Build:** `service.version` 6b50d9131417b8cf93feb61a7c640577398db567 (attested, see Attests). Method: [lab-run-plan.md, Verifier steps](../lab-run-plan.md), [DR-L5-archeologist-0.md](DR-L5-archeologist-0.md).

**Recommendation: findings, then rerun after a seed fix.** 10 of 22 rows PASS on server truth (setup, 8 launches, session end). 12 are FAIL (all refused `no_mechanics`), 0 PARTIAL, 0 BLOCKED. The runner's 10 PASS rows all match the server; its 12 FAIL rows (client `client.ability.applied` missing) are all server refusals, so the runner and the server agree on every row.

## Runs

| Call | Spec or split file | plan_only | run_dir | Outcome |
|---|---|---|---|---|
| 1-4 (6 rows each, then session end) | armory-abilities-archeologist-1 | no | `C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-03-unknown-cgbqzhbkqryynk` | runner PASS 10, FAIL 12 |

Window 03:06:56Z to 03:20:53Z, fresh character Cgbqzhbkqryynksb (Archeologist), default instance.

## Per-row grades

Server truth = SigNoz scope `abilities` over the window, one aggregate by `ability_id`, `event`, `reason` (each row presses twice, so counts are 2). Seed from `resources.abilities`, `resources.effects`, `resources.effect_nvps` via `server_db_query`.

| Row | Ability | Runner | Server evidence | Grade | Class |
|---|---|---|---|---|---|
| Archeologist-00 | setup | PASS | world 1300 reached | PASS |  |
| Archeologist-742 | Field Medic I | PASS | ability_launched x2, beneficial_cast_applied x2, heal_health x2 | PASS |  |
| Archeologist-743 | Field Medic II | PASS | ability_launched x2, beneficial_cast_applied x2, heal_health x2 | PASS |  |
| Archeologist-1451 | Cover Stance | PASS | ability_launched x2, effect_routing_launch x2, effect_routing_applied x2; cover_stance_applied/removed x2 | PASS |  |
| Archeologist-706 | Half Mag | PASS | ability_launched x2 (fire_target_skipped no_target x2) | PASS |  |
| Archeologist-654 | Single Pump | PASS | ability_launched x2 (fire_target_skipped no_target x2) | PASS |  |
| Archeologist-598 | Quick Burst | PASS | ability_launched x2 (fire_target_skipped no_target x2) | PASS |  |
| Archeologist-1638 | Destruction Shot | PASS | ability_launched x2 (fire_target_skipped no_target x2) | PASS |  |
| Archeologist-2848 | Submit to your Lord | PASS | ability_launched x2 (fire_target_skipped no_target x2) | PASS |  |
| Archeologist-643 | Induce Caution: Humanoid | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (2 effects, only EffectCategory=Mental NVP) |
| Archeologist-751 | Induce Daze: Humanoid | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (2 effects, 0 NVPs) |
| Archeologist-753 | Induce Panic: Humanoid | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (2 effects, only EffectCategory=Mental NVP) |
| Archeologist-1178 | Induce Caution: Alien | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (2 effects, 0 NVPs) |
| Archeologist-1181 | Induce Daze: Alien | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (4 effects, 0 NVPs) |
| Archeologist-1179 | Induce Doubt: Alien | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (2 effects, 0 NVPs) |
| Archeologist-1182 | Induce Calm: Humanoid | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (2 effects, 0 NVPs) |
| Archeologist-752 | Impose Will: Humanoid | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (2 effects, 0 NVPs) |
| Archeologist-1180 | Induce Panic: Alien | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (2 effects, 0 NVPs) |
| Archeologist-754 | Induce Mass Caution: Humanoid | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (4 effects, only EffectCategory=Mental NVPs) |
| Archeologist-718 | Selective Fire | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (1 effect 778, 0 NVPs) |
| Archeologist-777 | Induce Mass Panic: Humanoid | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (4 effects, 0 NVPs) |
| _session_end | none | PASS | not an ability row | PASS |  |

Totals: PASS 10, PARTIAL 0, FAIL 12, BLOCKED 0 = 22 rows. No passive abilities in this file. No weapon-moniker DATA-GAP: the weapon rows 598, 654, 706 launched; 718 was refused by the server on no_mechanics, whether its SGHC 6 SMG equip worked is UNKNOWN (not graded; the refusal is independent of it, seed evidence below).

## Server-side defects

One distinct defect: 12 abilities refused `no_mechanics`. None of their effects has a mechanic NVP or a `script_name`; the only NVP present anywhere is `EffectCategory=Mental` (effects 1228, 1474, 1479, 1486), a category tag, not a mechanic. Issue: [DR-L5-archeologist-1-no-mechanics.md](issues/DR-L5-archeologist-1-no-mechanics.md). Per-effect lacks (all `script_name` NULL; target collection `TCM_Single` unless noted):

| Ability | Effects (id name) | Needs |
|---|---|---|
| 643 | 1474 Suppression (EffectCategory=Mental), 707 Mental Resist Roll | suppression/mental-resist mechanic |
| 751 | 1427 Induce Daze: Humanoid Disorient, 797 Mental Resist Check | disorient + resist mechanic |
| 752 | 2475 Induce Rebellion: Humanoid Confusion (pulse 4), 798 Mental Resist | confusion + resist mechanic |
| 753 | 1486 Fear (EffectCategory=Mental), 799 Mental Resist Roll | fear + resist mechanic |
| 754 | 1480 Mental Resist Roll (`TCM_AECone`), 1479 Suppression (Mental), 1228 Suppression (Mental), 800 Mental Resist Roll | cone suppression + resist mechanic |
| 777 | 2776 Fear, 2775 Cone Mental Resist (`TCM_AECone`), 2774 Mental resist, 821 Fear | cone fear + resist mechanic |
| 1178 | 2773 Induce Caution: Alien Suppression, 1341 Mental Resist | suppression + resist mechanic |
| 1179 | 2777 Induce Rebellion: Alien Confusion, 1342 Mental Resist | confusion + resist mechanic |
| 1180 | 4566 Induce Panic: Alien Fear (pulse 20), 1343 Mental Resist | fear + resist mechanic |
| 1181 | 2781 Disorient: AOE (`TCM_AERadius`), 2780 Mental resist (`TCM_AERadius`), 2779 Induce Daze: Alien Disorient (pulse 20), 1344 Mental resist | AoE disorient + resist mechanic |
| 1182 | 2782 Induce Calm: Humanoid Threat Wipe, 1345 Mental Resist | threat wipe script |
| 718 | 778 Selective Fire Target Damage | damage NVPs (as the working Quick Burst 660, 2 NVPs) |

Reference for what works in this file: 742/743 (effects 788/789, 1 NVP each), 598/654/706/1638/2848 (damage rows with 2-4 NVPs), and 1451 (effect routing, 0 NVPs on the ability but routed through `effect_routing_launch`, so stance rows do launch without NVPs).

## NEEDS_HUMAN

None.

## Not run

None. All 22 rows were pressed.

## Resume hint

Nothing to resume. Next file in this packet: armory-abilities-archeologist-2 (running on instance p2; not touched here).

## Rerun list

- After the seed fix (`seed-change`): 643, 751, 752, 753, 754, 777, 1178, 1179, 1180, 1181, 1182, 718.
- No rerun for 00, 742, 743, 1451, 706, 654, 598, 1638, 2848.
- `fire_target_skipped no_target` on the 5 damage rows and 1451 did not stop the launch; whether it matters for damage landing is UNKNOWN (no damage event was queried).

## Attests

DONE 2026-11-03 03:22Z under lease `dr-verify-archeologist-1` on the default instance (lease was free), released at once: `lab_uat_attest {server_version: 6b50d9131417b8cf93feb61a7c640577398db567}` for this run dir (`...\2026-11-03-unknown-cgbqzhbkqryynk`) and for the pending archeologist-0 run dir (`...\2026-11-03-unknown-cgbqyvjnrgvlvq`). Both returned the version. Instance p2 untouched (an archeologist-2 run held its lease). No clause attests pending.

## Recipe

SigNoz `signoz_aggregate_logs`, filter `service.name = 'cimmeria-server' AND cimmeria.lab_run = 'dr-lab' AND scope_name = 'abilities' AND player_name CONTAINS 'Cgbqzhbkqryynksb' AND event EXISTS AND event != 'warmup_pending'`, start 1793675190000, end 1793676080000, count grouped by ability_id, event, reason. `service.version` read from one SigNoz row in the window (resource attribute). Seed: `server_db_query` on `resources.abilities` (`effect_ids`, `passive_yn`, `target_type_id`, `required_ammo`) with a count of `resources.effect_nvps` for `effect_id = ANY(effect_ids)`, then `resources.effects` (`name`, `script_name`, `target_collection_method`, `pulse_count`) with its NVPs aggregated via `string_agg`. Seed file and line for the effects: UNKNOWN (graded from the live DB, not the seed SQL).

## Driver reports

Not supplied to the verifier; graded from the bundle only.
