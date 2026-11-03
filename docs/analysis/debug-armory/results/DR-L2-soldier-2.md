# DR-L2-soldier-2: Soldier abilities, tree 2 (armory-abilities-soldier-2)

> **Type:** results. **Audience:** the orchestrator and architect. **Verified:** 2026-11-02, read-only (run bundle, SigNoz `cimmeria.lab_run = 'dr-lab'`, lab-server `server_db_query`). **Build:** `service.version` 6b50d9131417b8cf93feb61a7c640577398db567 (attested under a short lease, released). Method: [lab-run-plan.md, Verifier steps](../lab-run-plan.md), [DR-L2-soldier-1.md](DR-L2-soldier-1.md), [canary.md](../worknotes/canary.md).

**Recommendation (final, after the rerun): findings; rerun only after the seed fix.** Merged totals: PASS 15 (incl. setup), FAIL 8, PARTIAL 3, BLOCKED 0 of 26 rows. First run (superseded): 14 of 26 rows PASS on server truth, 6 real seed gaps (the gate refuses them `no_mechanics`), 4 PARTIAL (3 passives refused by design, 1 unexplained press path), 2 BLOCKED by hotbar capacity.

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

## Rerun 2026-11-02 (post DR-07l/m/n)

Run `C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqokntohiosu`, window 21:48:38Z to 21:54:28Z, fresh character Cgbqokntohiosujp, rows Soldier-00, 1329, 856, 2864, 1894, 1893 plus `_session_end`. Runner: PASS 6 (setup, `_session_end`, 1329, 856, 1894, 1893), FAIL 1 (2864). `service.version` 6b50d9131417b8cf93feb61a7c640577398db567 attested for this run under a short lease (none was held), released. Server truth: SigNoz `service.name = 'cimmeria-server' AND cimmeria.lab_run = 'dr-lab' AND scope_name = 'abilities' AND player_name CONTAINS 'Cgbqokntohiosu' AND event EXISTS AND event != 'warmup_pending'`, 1793656080000 to 1793656500000 (32 rows). Seed from `resources.abilities` / `resources.effect_nvps` (`server_db_query`).

The hotbar fix (DR-07n) worked: all five rows now reach the server, including 1894 and 1893, which were BLOCKED before. Every ground row sent `use_ability_on_ground_recv` (DR-07l/m), two presses each except 2864 (one).

| Row | Ability | Runner | Server evidence | Grade | Class |
|---|---|---|---|---|---|
| Soldier-1329 | Incendiary Grenade | PASS ([FAILED] applied) | 2x `use_ability_on_ground_recv` + `ability_refused no_mechanics` (`effect_count 0`) 21:50:09, 21:50:34 | FAIL | seed gap (`effect_ids` empty); runner PASS refused by the server |
| Soldier-856 | Takedown | PASS | 2x `use_ability_recv` + `ability_launched` (cast 1, 2) + `fire_target_skipped no_target` 21:50:57, 21:51:08 | PASS | launch proven; damage not examined (no target) |
| Soldier-2864 | EMP Grenade | FAIL (`ground_confirm` `hover_timeout (10 s)`) | 1x `use_ability_on_ground_recv` + `ability_refused no_mechanics` (`effect_count 7`) 21:51:44 | FAIL | seed gap (7 effects 4200-4206, 0 NVPs); runner hover timeout on a later press is a lab flake |
| Soldier-1894 | Phase Grenade | PASS ([FAILED] applied) | 2x `use_ability_on_ground_recv` + `ability_refused no_mechanics` (`effect_count 0`) 21:53:05, 21:53:29 | FAIL | seed gap (`effect_ids` empty); runner PASS refused |
| Soldier-1893 | Stance: Commander | PASS ([FAILED] applied) | 2x `use_ability_recv` + `ability_refused no_mechanics` (`effect_count 2`) 21:53:52, 21:54:01 | FAIL | seed gap (2 effects, 0 NVPs); runner PASS refused |

### Soldier-2864: ability or runner?

Both, but the ability verdict stands on its own. The first press reached the server (ground recv at 250.049, 6.585, -870.954, the dummy spot, so that press had a good ground point) and the gate refused it `no_mechanics`: the seed row has 7 effects and no NVPs. The runner then failed at `ground_confirm` on a later press: it asked for a ground click at (-87107.18, 25000, 662.15), which is not a Debug Area coordinate, and the hover probe timed out at 10 s. That is a runner/ground-point flake (BLOCKED-infra for the second press: no second server row). It does not rescue the row: the ability is an ability issue (seed gap), so the grade is FAIL, and the FAIL would stay after a clean second press. No separate issue body for the flake (one occurrence); the rerun after the seed fix shows whether it repeats.

## Findings (rerun)

1. **Seed gap, 8 rows in all:** 854, 858, 1455, 1891, 1329, 2864 (first run) plus 1894 and 1893 (new, pressed for the first time). New issue body: [DR-L2-soldier-2-no-mechanics-stances.md](issues/DR-L2-soldier-2-no-mechanics-stances.md). 1329 and 2864 stay in [DR-L2-soldier-2-no-mechanics.md](issues/DR-L2-soldier-2-no-mechanics.md).
2. **Resolved:** the unexplained "server logged a press despite the runner error" path from the first run is replaced by clean ground presses (two per row). 856 launches twice.
3. **Lab flake:** 2864 `hover_timeout` with an out-of-world ground point, one occurrence.

## Merged final grades (first run plus rerun)

| Row | Ability | Final | Basis |
|---|---|---|---|
| Soldier-00 | setup | PASS | |
| Soldier-1642 | Stance: Soldier | PASS | first run |
| Soldier-637 | Aim | PASS | first run |
| Soldier-1454 | Hunker Down | PASS | first run |
| Soldier-523 | Concussive Grenade | PASS | first run |
| Soldier-1452 | Duck and Cover | PASS | first run |
| Soldier-858 | Leadership | FAIL | seed gap, first run |
| Soldier-1455 | Quick Launch | FAIL | seed gap, first run |
| Soldier-1619 | Combat Sprint | PASS | first run |
| Soldier-1453 | Heroism | PASS | first run |
| Soldier-868 | Flashbang Grenade | PASS | first run |
| Soldier-1450 | Cover Penetration | PARTIAL | passive (design) |
| Soldier-869 | Morale Boost | PASS | first run |
| Soldier-861 | High-Explosive Grenade | PASS | first run |
| Soldier-1891 | Stance: Rally | FAIL | seed gap, first run |
| Soldier-1457 | Steadfast | PARTIAL | passive (design) |
| Soldier-854 | Smoke Grenade | FAIL | seed gap, first run |
| Soldier-1479 | Throw Mastery | PARTIAL | passive (design) |
| Soldier-1451 | Cover Stance | PASS | first run |
| Soldier-1458 | Stance: Ranged Specialist | PASS | first run |
| Soldier-1364 | Shockwave Grenade | PASS | first run |
| Soldier-1329 | Incendiary Grenade | FAIL | seed gap, rerun |
| Soldier-856 | Takedown | PASS | rerun, 2 launches |
| Soldier-2864 | EMP Grenade | FAIL | seed gap, rerun (runner hover flake on a later press) |
| Soldier-1894 | Phase Grenade | FAIL | seed gap, rerun |
| Soldier-1893 | Stance: Commander | FAIL | seed gap, rerun |

**Merged totals soldier-2: PASS 15 (incl. setup), FAIL 8, PARTIAL 3, BLOCKED 0 (26 rows).**

## Rerun list (after the rerun)

Rerun only after the seed fix (`seed-change`): 854, 858, 1455, 1891, 1329, 2864, 1894, 1893. Nothing is BLOCKED any more.
