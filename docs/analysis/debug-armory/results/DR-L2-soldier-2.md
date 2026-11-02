# DR-L2-soldier-2: Soldier abilities, tree 2 (armory-abilities-soldier-2)

> **Type:** results. **Audience:** the orchestrator and architect. **Verified:** 2026-11-02, read-only (run bundle, SigNoz `cimmeria.lab_run = 'dr-lab'`, lab-server `server_db_query`). **Build:** `service.version` 6b50d9131417b8cf93feb61a7c640577398db567 (attested under a short lease, released). Method: [lab-run-plan.md, Verifier steps](../lab-run-plan.md), [DR-L2-soldier-1.md](DR-L2-soldier-1.md), [canary.md](../worknotes/canary.md).

**Recommendation: findings plus a small rerun.** 14 of 26 rows PASS on server truth. 6 are real seed gaps (the gate refuses them `no_mechanics`), 4 are PARTIAL (3 passives refused by design, 1 unexplained press path), 2 are BLOCKED by hotbar capacity.

## Runs

| Call | Spec or split file | plan_only | run_dir | Outcome |
|---|---|---|---|---|
| 1-5 (6 rows each, `end_session: false`) | armory-abilities-soldier-2 | no | `C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqnmmsdfkvui` | runner PASS 22, FAIL 3, BLOCKED 2 |

Window 21:21:33Z to 21:37:38Z, fresh character Cgbqnmmsdfkvuijp. Server-truth grading: PASS 14, FAIL 6, PARTIAL 4, BLOCKED 2 (26 rows). Seven runner PASS rows (858, 1455, 1450, 1891, 1457, 854, 1479) had the `applied` clause FAILED; the server refused all seven.

## Per-row grades

Server truth = SigNoz scope `abilities`, 374 rows over the window, one `service.version`. Each pressed row shows two presses with identical outcomes unless noted.

| Row | Ability | Runner | Server evidence | Grade | Class |
|---|---|---|---|---|---|
| Soldier-00 | setup | PASS | world 1300 reached | PASS | |
| Soldier-1642 | Stance: Soldier | PASS | `ability_launched`, `toggle_pressed`, 3 `stat_buff_applied` 21:22:43 | PASS | |
| Soldier-637 | Aim | PASS | `ability_launched`, `stat_buff_applied` 21:23:11 | PASS | |
| Soldier-1454 | Hunker Down | PASS | `ability_launched`, `stat_buff_applied` 21:23:40 | PASS | |
| Soldier-523 | Concussive Grenade | PASS | `use_ability_on_ground_recv`, `ability_launched`, `crowd_control_applied` 21:24:25 | PASS | |
| Soldier-1452 | Duck and Cover | PASS | `ability_launched`, `stat_buff_applied` 21:25:08 | PASS | |
| Soldier-858 | Leadership | PASS ([FAILED] applied) | `ability_refused no_mechanics` 21:25:39, 21:25:48 | FAIL | seed gap (effects 1211, 921 have no NVPs) |
| Soldier-1455 | Quick Launch | PASS ([FAILED] applied) | `ability_refused no_mechanics` 21:26:11, 21:26:20 | FAIL | seed gap (effect_ids empty) |
| Soldier-1619 | Combat Sprint | PASS | `ability_launched`, `stat_buff_applied` 21:26:43 | PASS | |
| Soldier-1453 | Heroism | PASS | `ability_launched`, `stat_buff_applied` 21:27:10 | PASS | |
| Soldier-868 | Flashbang Grenade | PASS | `ability_launched`, `crowd_control_applied` 21:27:53 | PASS | |
| Soldier-1450 | Cover Penetration | PASS ([FAILED] applied) | `ability_refused no_mechanics` (`passive_ability`) 21:22:35 and 21:28:44 | PARTIAL | passive (design) |
| Soldier-869 | Morale Boost | PASS | `ability_launched`, `heal_focus` 21:29:10 | PASS | |
| Soldier-861 | High-Explosive Grenade | PASS | `use_ability_on_ground_recv`, `ability_launched` 21:29:57 | PASS | |
| Soldier-1891 | Stance: Rally | PASS ([FAILED] applied) | `ability_refused no_mechanics` 21:30:40, 21:30:49 | FAIL | seed gap (5 effects, no NVPs) |
| Soldier-1457 | Steadfast | PASS ([FAILED] applied) | `ability_refused` `passive_ability` 21:31:12, 21:31:21 | PARTIAL | passive (design) |
| Soldier-854 | Smoke Grenade | PASS ([FAILED] applied) | `ability_refused no_mechanics` 21:32:00, 21:32:24 | FAIL | seed gap (effect_ids empty) |
| Soldier-1479 | Throw Mastery | PASS ([FAILED] applied) | `ability_refused` `passive_ability` 21:32:47, 21:32:57 | PARTIAL | passive (design) |
| Soldier-1451 | Cover Stance | PASS | `ability_launched`, `effect_routed self_ability`, `cover_stance_applied` 21:33:21 | PASS | |
| Soldier-1458 | Stance: Ranged Specialist | PASS | `ability_launched`, `toggle_pressed`, `stat_buff_applied` 21:33:48 | PASS | |
| Soldier-1364 | Shockwave Grenade | PASS | `use_ability_on_ground_recv`, `ability_launched`, `crowd_control_applied` 21:34:32 | PASS | |
| Soldier-1329 | Incendiary Grenade | FAIL (button 21 unbound) | `use_ability_on_ground_recv`, `ability_refused no_mechanics` 21:35:30 (one press) | FAIL | seed gap (effect_ids empty); press path unexplained |
| Soldier-856 | Takedown | FAIL (button 22 unbound) | one `use_ability_recv`, `ability_launched`, `fire_target_skipped no_target` 21:36:03 | PARTIAL | runner could not press; one stray launch, not explained |
| Soldier-2864 | EMP Grenade | FAIL (button 23 unbound) | `use_ability_on_ground_recv`, `ability_refused no_mechanics` 21:36:55 (one press) | FAIL | seed gap (7 effects 4200-4206, no NVPs); press path unexplained |
| Soldier-1894 | Phase Grenade | BLOCKED (no visible empty hotbar button) | no server row | BLOCKED | infra, hotbar capacity (seed also has empty effect_ids) |
| Soldier-1893 | Stance: Commander | BLOCKED (same) | no server row | BLOCKED | infra, hotbar capacity |

Totals: PASS 14 (incl. setup), FAIL 6, PARTIAL 4, BLOCKED 2, 26 rows.

## Findings

1. **Seed gap, 6 rows (real defect):** 854, 858, 1455, 1891, 1329, 2864. The gate refuses them `no_mechanics` (no effect carries an NVP, or `effect_ids` is empty). Four runner PASS rows (854, 858, 1455, 1891) hid this because only the `sent` clause counted. Issue: [DR-L2-soldier-2-no-mechanics.md](issues/DR-L2-soldier-2-no-mechanics.md). Fix via `seed-change`.
2. **Passives, 3 rows:** 1450, 1457, 1479 are refused `passive_ability` by design; PARTIAL as in soldier-0.
3. **Hotbar capacity (infra):** the file has 26 rows but about 23 usable buttons. 1329, 856, 2864 failed the client press at buttons 21-23, 1894 and 1893 found no empty button. Not defects. Odd: the server logged a press for 1329, 856 and 2864 despite the runner error, with one press each rather than two. The path is unexplained (perhaps a ground-click fallback); 856 is held PARTIAL for that reason.
4. Where logged, launches show `fire_target_skipped no_target` (client_target_id 0): launch proven, damage not examined.

## Not run / UNVERIFIED

No PENDING clauses. 1893 and 1894 were never pressed.

## Rerun list (fresh hotbar)

- 1894, 1893: BLOCKED, rerun on a fresh hotbar (put them first in a short file). 1894 will then hit the seed gap.
- 856: rerun to confirm a press path and a second launch.
- 1329, 2864: only one press seen and the runner failed; rerun with a bound button after the seed fix.
- 854, 858, 1455, 1891, 1329, 2864: rerun after the seed fix.

## Attests

`lab_uat_attest {server_version: 6b50d9131417b8cf93feb61a7c640577398db567}` applied to the run dir above under a short lease (none was held), then released. No clause attests.

## Recipe

SigNoz `signoz_search_logs`, filter `service.name = 'cimmeria-server' AND cimmeria.lab_run = 'dr-lab' AND scope_name = 'abilities' AND player_name CONTAINS 'Cgbqnmmsdfkvui' AND event EXISTS AND event != 'warmup_pending'`, start 1793654400000, end 1793655540000, limit 500 (374 rows). Seed: `resources.abilities` with `resources.effects` and `resources.effect_nvps` counts per ability.
