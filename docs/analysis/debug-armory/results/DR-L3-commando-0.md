# DR-L3-commando-0: Commando abilities, tree 0 (armory-abilities-commando-0)

> **Type:** results. **Audience:** the orchestrator and architect. **Verified:** 2026-11-02, read-only (run bundle, SigNoz `cimmeria.lab_run = 'dr-lab'`, lab-server `server_db_query`). **Build:** `service.version` not yet attested (`run.json` says `unknown`); attest pending, see Attests. Method: [lab-run-plan.md, Verifier steps](../lab-run-plan.md), [DR-L2-soldier-2.md](DR-L2-soldier-2.md).

**Recommendation: findings, then rerun after a seed fix.** 5 of 24 rows PASS on server truth. 15 are FAIL (the gate refuses them `no_mechanics`), 4 are PARTIAL (passives refused by design), 0 BLOCKED. The runner called 23 of the 24 rows PASS; only Commando-1259 failed, and for the same seed reason.

## Runs

| Call | Spec or split file | plan_only | run_dir | Outcome |
|---|---|---|---|---|
| 1-4 (6 rows each, `end_session: false`) | armory-abilities-commando-0 | no | `C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqpzpxnjqkjl` | runner PASS 24 (incl. setup, `_session_end`), FAIL 1 (Commando-1259) |

Window 22:34:55Z to 22:52:00Z, fresh character Cgbqpzpxnjqkjlqb (Commando). Server-truth grading: PASS 5 (incl. setup), FAIL 15, PARTIAL 4, BLOCKED 0 (24 rows). Eighteen runner PASS rows had the `applied` clause FAILED; the server refused all of them. Commando-1259 is the only runner FAIL and the server refused it too.

## Per-row grades

Server truth = SigNoz scope `abilities` over the window (count grouped by `ability_id`, `event`, `reason`). Each row shows two presses with identical outcomes. Only 4 abilities (523, 647, 861, 1364) have `ability_launched`.

| Row | Ability | Runner | Server evidence | Grade | Class |
|---|---|---|---|---|---|
| Commando-00 | setup | PASS | world 1300 reached | PASS |  |
| Commando-523 | Concussive Grenade | PASS | ground_recv, ability_launched x2, crowd_control_applied | PASS |  |
| Commando-647 | C-4: Time Delay | PASS | ground_recv, ability_launched x2, crowd_control_applied, stat_buff_applied | PASS |  |
| Commando-861 | High-Explosive Grenade | PASS | ground_recv, ability_launched x2 | PASS |  |
| Commando-1364 | Shockwave Grenade | PASS | ground_recv, ability_launched x2, crowd_control_applied | PASS |  |
| Commando-1259 | Sticky Bomb | FAIL (applied) | use_ability_recv x2, ability_refused no_mechanics x2 (3 effects 4363,1625,1431, 0 NVPs) | FAIL | seed gap (effects, no NVPs) |
| Commando-1363 | Incendiary Mine | PASS ([FAILED] applied) | ability_refused no_mechanics x2 | FAIL | seed gap (effect_ids empty) |
| Commando-1366 | Cluster Mine | PASS ([FAILED] applied) | ability_refused no_mechanics x2 | FAIL | seed gap (effect_ids empty) |
| Commando-1868 | Tactical Nuke | PASS ([FAILED] applied) | ability_refused no_mechanics x2 | FAIL | seed gap (effect_ids empty) |
| Commando-642 | Anti-Personnel Mine | PASS ([FAILED] applied) | ability_refused no_mechanics x2 | FAIL | seed gap (effect_ids empty) |
| Commando-886 | Flash Mine | PASS ([FAILED] applied) | ability_refused no_mechanics x2 | FAIL | seed gap (effect_ids empty) |
| Commando-888 | Claymore Mine | PASS ([FAILED] applied) | ability_refused no_mechanics x2 | FAIL | seed gap (effect_ids empty) |
| Commando-890 | Overload Mine | PASS ([FAILED] applied) | ability_refused no_mechanics x2 | FAIL | seed gap (effect_ids empty) |
| Commando-883 | Disarm Device I | PASS ([FAILED] applied) | ability_refused no_mechanics x2 | FAIL | seed gap (2 effects, 0 NVPs) |
| Commando-884 | Disarm Device II | PASS ([FAILED] applied) | ability_refused no_mechanics x2 | FAIL | seed gap (2 effects, 0 NVPs) |
| Commando-901 | Disarm Device III | PASS ([FAILED] applied) | ability_refused no_mechanics x2 | FAIL | seed gap (2 effects, 0 NVPs) |
| Commando-887 | Place C-4 Charge | PASS ([FAILED] applied) | ground_recv, ability_refused no_mechanics x2 | FAIL | seed gap (4 effects, 0 NVPs) |
| Commando-2173 | Stance: Demolitions | PASS ([FAILED] applied) | ability_refused no_mechanics x2 | FAIL | seed gap (2 effects, 0 NVPs) |
| Commando-2864 | EMP Grenade | PASS ([FAILED] applied) | ground_recv, ability_refused no_mechanics x2 | FAIL | seed gap (7 effects 4200-4206, 0 NVPs) |
| Commando-3358 | Detonate C-4 | PASS ([FAILED] applied) | ability_refused no_mechanics x2 | FAIL | seed gap (1 effect 4957, 0 NVPs) |
| Commando-1362 | C-4: Link Charge 6 | PASS ([FAILED] applied) | no_mechanics_refused passive_ability x2 | PARTIAL | passive (design) |
| Commando-885 | C-4: Link Charge 2 | PASS ([FAILED] applied) | no_mechanics_refused passive_ability x2 | PARTIAL | passive (design) |
| Commando-889 | C-4: Link Charge 4 | PASS ([FAILED] applied) | no_mechanics_refused passive_ability x2 | PARTIAL | passive (design) |
| Commando-2172 | Disarm Mastery | PASS ([FAILED] applied) | no_mechanics_refused passive_ability x2 | PARTIAL | passive (design) |

Totals: PASS 5 (incl. setup), FAIL 15, PARTIAL 4, BLOCKED 0, 24 rows. The spec-blocked rows 1246, 1250 and 1355 are in `-1`, not this file.

## Findings

1. **Seed gap, 15 rows (real defect):** 1259, 1363, 1366, 1868, 642, 886, 888, 890, 883, 884, 901, 887, 2173, 2864, 3358. Seven have empty `effect_ids` (642, 886, 888, 890, 1363, 1366, 1868: the mines and the nuke). Eight have effects but `resources.effect_nvps` holds no NVP for any of them (883, 884, 887, 901, 1259, 2173, 2864, 3358). The gate refuses every one `no_mechanics`. The runner PASSed 14 of them because only the `sent` clause counted. Issue: [DR-L3-commando-0-no-mechanics.md](issues/DR-L3-commando-0-no-mechanics.md). Fix via `seed-change`.
2. **Passives, 4 rows:** 1362, 885, 889 (C-4 Link Charge 6, 2, 4) and 2172 (Disarm Mastery) are refused `passive_ability` by design. PARTIAL.
3. **Working abilities:** the three grenades and C-4 Time Delay (523, 647, 861, 1364) launch, send ground clicks, and apply crowd control or buffs.
4. Same pattern as DR-L2 soldier-2: seed gaps hidden by runner PASS; the `applied` FAIL is the only runner sign.

## Not run / UNVERIFIED

No PENDING clauses. Nothing BLOCKED.

## Rerun list

- Rerun all 15 FAIL rows after the seed fix (`seed-change`): 1259, 1363, 1366, 1868, 642, 886, 888, 890, 883, 884, 901, 887, 2173, 2864, 3358.
- No rerun for the 4 passives or the 4 PASS rows.

## Attests

Pending, not run: a driver (`lab_uat_run`, commando-1) held the lab lease at 22:52Z (owner `lab_uat_run`, expiring about 23:02Z). Pending: `lab_uat_attest {run_dir: C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqpzpxnjqkjl, server_version: <service.version of the window>}`. No clause attests. Re-dispatch the attest step in the next idle-lab gap (R5). The grades above do not depend on it.

## Recipe

SigNoz `signoz_aggregate_logs`, filter `service.name = 'cimmeria-server' AND cimmeria.lab_run = 'dr-lab' AND scope_name = 'abilities' AND player_name CONTAINS 'Cgbqpzpxnjqkjl' AND event EXISTS AND event != 'warmup_pending'`, start 1793658800000, end 1793659950000, count grouped by ability_id, event, reason. Seed: `server_db_query` on `resources.abilities` with a count of `resources.effect_nvps` rows whose `effect_id` is in the ability's `effect_ids`. Runner `applied` clauses from `ledger.md`.
