# DR-L3-commando-2: Commando abilities, tree 2 (armory-abilities-commando-2)

> **Type:** results. **Audience:** the orchestrator and architect. **Verified:** 2026-11-02, read-only (run bundle, SigNoz `cimmeria.lab_run = 'dr-lab'`, lab-server `server_db_query`). **Build:** `service.version` 6b50d9131417b8cf93feb61a7c640577398db567 (as attested for commando-0 and -1; this run's attest is pending, see Attests). Method: [lab-run-plan.md, Verifier steps](../lab-run-plan.md), [DR-L3-commando-1.md](DR-L3-commando-1.md).

**Recommendation: mostly good; one seed fix.** 17 of 20 rows PASS on server truth (incl. setup). 1 FAIL (Commando-640, the gate refuses it `no_mechanics`), 2 PARTIAL (passives 870, 871 refused by design), 0 BLOCKED. The runner called all 20 PASS except 640; its FAIL has the same seed reason. 870 and 871 were runner PASS with the `applied` clause FAILED, which fits the passive refusal.

## Runs

| Call | Spec or split file | plan_only | run_dir | Outcome |
|---|---|---|---|---|
| 1-4 (6 rows each, `end_session: false`) | armory-abilities-commando-2 | no | `C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqrbvtowfscz` | runner PASS 20 (incl. setup, `_session_end`), FAIL 1 (Commando-640) |

Window 23:06:43Z to 23:19:51Z, fresh character Cgbqrbvtowfsczqr (Commando).

## Per-row grades

Server truth = SigNoz scope `abilities` over the window (every event twice, each row presses twice). Seed columns from `resources.abilities` and `resources.effect_nvps`.

| Row | Ability | Runner | Server evidence | Grade | Class |
|---|---|---|---|---|---|
| Commando-00 | setup | PASS | world 1300 reached | PASS |  |
| Commando-640 | Aimed Shot: Torso | FAIL (applied) | no_mechanics_refused no_mechanics, ability_refused no_mechanics x2 | FAIL | seed gap (effect 703, 0 NVPs) |
| Commando-655 | Snap Shot | PASS | ability_launched x2 (fire_target_skipped no_target) | PASS |  |
| Commando-847 | Call Target | PASS | ability_launched x2 | PASS |  |
| Commando-848 | Repeated Fire | PASS | ability_launched x2 | PASS |  |
| Commando-849 | Sight In | PASS | ability_launched x2, beneficial_cast_applied, stat_buff_applied | PASS |  |
| Commando-850 | Scattershot | PASS | ability_launched x2 | PASS |  |
| Commando-855 | Aimed Shot: Leg | PASS | ability_launched x2 | PASS |  |
| Commando-857 | Leading the Target | PASS | ability_launched x2, stat_buff_applied, toggle_pressed | PASS |  |
| Commando-859 | Concentration | PASS | ability_launched x2, stat_buff_applied, toggle_pressed | PASS |  |
| Commando-862 | Impeccable Aim | PASS | ability_launched x2, stat_buff_applied | PASS |  |
| Commando-870 | Sniper | PASS ([FAILED] applied) | no_mechanics_refused passive_ability x2 | PARTIAL | passive (design) |
| Commando-871 | Rapid Aim | PASS ([FAILED] applied) | no_mechanics_refused passive_ability x2 | PARTIAL | passive (design) |
| Commando-877 | Forward Observer | PASS | use_ability_on_ground_recv, ability_launched x2, stat_buff_applied | PASS |  |
| Commando-1242 | Aimed Shot: Arm | PASS | ability_launched x2 | PASS |  |
| Commando-1331 | Multi Pump | PASS | ability_launched x2 | PASS |  |
| Commando-1332 | Point Blank Fire | PASS | ability_launched x2 | PASS |  |
| Commando-1354 | Disabling Shot | PASS | ability_launched x2 | PASS |  |
| Commando-1358 | Lethal Shot | PASS | ability_launched x2 | PASS |  |
| Commando-1359 | Red Mist | PASS | ability_launched x2 | PASS |  |

Totals: PASS 17 (incl. setup), FAIL 1, PARTIAL 2, BLOCKED 0, 20 rows.

## Findings

1. **Seed gap, 1 row (real defect):** Commando-640 (effect 703) has no NVP; refused `no_mechanics` both by the gate and the runner's own FAIL. Same class as commando-0 and -1; issue [DR-L3-commando-2-no-mechanics.md](issues/DR-L3-commando-2-no-mechanics.md). Fix all three packets' rows in one `seed-change` pass.
2. **Passives, 2 rows:** 870 Sniper and 871 Rapid Aim are `passive_yn = true`, refused `passive_ability` by design. PARTIAL.
3. **Working abilities, 16 rows:** 655, 847-850, 855, 857, 859, 862, 877, 1242, 1331, 1332, 1354, 1358, 1359 all show `ability_launched`; toggles and buffs (849, 857, 859, 862, 877) also show `stat_buff_applied`. The two weapon-gated sets (SR1 .50-Cal Rifle, Pump Shotgun) worked. `fire_target_skipped no_target` appears on the targeted rows while the launch still happens, as in commando-1.
4. **Noise:** one `ability_refused unknown_ability_id` for id 95 (plus `use_ability_on_ground_recv`) in the window comes from the setup `/gmgiveallabilities` flow, not a graded row.

## Not run / UNVERIFIED

No PENDING clauses, no UNVERIFIED rows, nothing not run.

## Rerun list

- After the seed fix (`seed-change`): 640.
- No rerun for the 2 passives or the 17 PASS rows.

## Attests

PENDING: `lab_uat_attest {server_version: 6b50d9131417b8cf93feb61a7c640577398db567}` for run dir `C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqrbvtowfscz`. Not run by this verifier because a driver is running the Scientist file (R5: no attest while the lab is in use). The orchestrator re-dispatches the attest step in the next idle-lab gap. No clause attests are pending.

## Recipe

SigNoz `signoz_aggregate_logs`, filter `service.name = 'cimmeria-server' AND cimmeria.lab_run = 'dr-lab' AND scope_name = 'abilities' AND player_name CONTAINS 'Cgbqrbvtowfsczqr' AND event EXISTS AND event != 'warmup_pending'`, start 1793660750000, end 1793661700000, count grouped by ability_id, event, reason. Seed: `server_db_query` on `resources.abilities` (`effect_ids`, `item_monikers`, `passive_yn`, `target_type_id`) with a count of `resources.effect_nvps` rows whose `effect_id` is in `effect_ids`.

## DR-L3 totals (commando-0, -1, -2)

| File | Rows | PASS | FAIL | PARTIAL | BLOCKED |
|---|---|---|---|---|---|
| commando-0 | 24 | 5 | 15 | 4 | 0 |
| commando-1 | 23 | 7 | 7 | 5 | 4 |
| commando-2 | 20 | 17 | 1 | 2 | 0 |
| **Total** | **67** | **29** | **23** | **11** | **4** |

PASS counts include each file's setup row. All 23 FAILs are seed gaps (`no_mechanics`); the 4 BLOCKED are 1246, 1250, 1355 (D-DR3 / data gap) and 880 (press ignored).
