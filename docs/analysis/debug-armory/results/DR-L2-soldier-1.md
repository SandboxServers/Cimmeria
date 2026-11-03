# DR-L2-soldier-1: Soldier abilities, tree 1 (armory-abilities-soldier-1)

> **Type:** results. **Audience:** the orchestrator and architect. **Verified:** 2026-11-02, read-only (run bundle, `lab_uat_report`, SigNoz `cimmeria.lab_run = 'dr-lab'`, lab-server `server_db_query`). **Build:** `service.version` 6b50d9131417b8cf93feb61a7c640577398db567 (attested). Method: [lab-run-plan.md § Verifier steps](../lab-run-plan.md), [DR-L2-soldier-0.md](DR-L2-soldier-0.md), [canary.md](../worknotes/canary.md).

**Recommendation (final, after the reruns): findings.** Merged totals: PASS 21, FAIL 5, BLOCKED 0 of 26 rows (see Merged final grade below). All 5 FAIL rows are seed gaps (no effects or no NVPs): 656, 1483, 1456, 1882, 2102. Row 852 passes in the ground rerun. First run (superseded): only 10 of 26 rows reached a verdict and 16 rows were BLOCKED by infrastructure (SGW.exe crash on every runner logout, issue #1680, and a separate ignored-press fault), not by Armory defects. Two rows are real seed gaps.

## Runs

Run dir `C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqjzujpscktw`, window 19:39:21Z to 19:52:03Z, 6 `lab_uat_run` calls with `end_session: false`, fresh character Cgbqjzujpscktwzf. `lab_uat_report`: PASS 9, FAIL 1, BLOCKED 16 (runner). Server-truth grading below: PASS 8, FAIL 2, BLOCKED 16 (Soldier-1483 is a runner PASS the server refused).

## Per-row grades

Server truth = SigNoz scope `abilities` (`use_ability_recv`, `ability_launched`, `ability_refused`), 81 rows over the window, one `service.version`. Each pressed row shows two presses with identical outcomes.

| Row | Ability | Grade | Server evidence | Class |
|---|---|---|---|---|
| Soldier-00 | setup | PASS | world 1300 reached |  |
| Soldier-612 | Area Burst | PASS | `ability_launched` 19:40:38.809 |  |
| Soldier-632 | Long Burst | PASS | `ability_launched` 19:41:17.291 |  |
| Soldier-851 | Brace | PASS | `ability_launched` 19:41:48.130 |  |
| Soldier-658 | Heavy Suppression | PASS | `ability_launched` 19:42:24.049 |  |
| Soldier-819 | Steady Aim | PASS | `ability_launched` 19:42:54.858 |  |
| Soldier-656 | Sustained Bursts | FAIL | `ability_refused no_mechanics` 19:43:50.786 | seed gap (effects 4336, 721 have no NVPs) |
| Soldier-1883 | Incinerating Stream | PASS | `ability_launched` 19:44:43.675 |  |
| Soldier-1483 | Steady Launch | FAIL | `ability_refused no_mechanics` 19:45:41.090 | seed gap (Buff effect 1779 has no NVPs); runner said PASS (sent clause only) |
| Soldier-853 | Fire Zone | PASS | `ability_launched` 19:48:11.701 |  |
| Soldier-2419 | Launch Grenade: Single | BLOCKED | no server row (never pressed) | setup press ignored |
| Soldier-808 | Cover Denial | BLOCKED | no server row (never pressed) | setup press ignored |
| Soldier-2103 | Launch Grenade: Concussion | BLOCKED | no server row (never pressed) | setup press ignored |
| Soldier-818 | Sustained Sweep | BLOCKED | no server row (never pressed) | logout, bridge connect (client dead) |
| Soldier-1882 | Launch Grenade: Tear Gas | BLOCKED | no server row (never pressed) | setup press ignored |
| Soldier-2102 | Incinerate Ground | BLOCKED | no server row (never pressed) | setup press ignored |
| Soldier-724 | Anti-Personnel Mortar | BLOCKED | no server row (never pressed) | setup press ignored |
| Soldier-1885 | Launch Grenade: Shockwave | BLOCKED | no server row (never pressed) | setup press ignored |
| Soldier-852 | High Explosive Mortar | BLOCKED | no server row (never pressed) | logout, dispatch timeout |
| Soldier-1889 | Incinerate Area | BLOCKED | no server row (never pressed) | logout, dispatch timeout |
| Soldier-2143 | Intimidation Fire | BLOCKED | no server row (never pressed) | logout, dispatch timeout |
| Soldier-1485 | Cluster Mortars | BLOCKED | no server row (never pressed) | logout, bridge connect |
| Soldier-2105 | Launch Grenade: Barrage | BLOCKED | no server row (never pressed) | setup press ignored |
| Soldier-1487 | Penetrating Barrage | BLOCKED | no server row (never pressed) | setup press ignored |
| Soldier-1456 | Shock and Awe | BLOCKED | no server row (never pressed) | logout, bridge connect (56 s) |
| Soldier-1486 | Rain of Steel | BLOCKED | no server row (never pressed) | login focus, bridge connect |

Totals: PASS 8 (incl. setup), FAIL 2, BLOCKED 16, 26 rows.

## Findings

1. **Seed gap, 2 rows (real defect).** 656 Sustained Bursts (DD, effects 4336, 721) and 1483 Steady Launch (Buff, effect 1779) have zero `effect_nvps`, so the gate refuses them with `no_mechanics`. Issue: [DR-L2-soldier-1-nvps-missing.md](issues/DR-L2-soldier-1-nvps-missing.md). Fix via `seed-change`; same family as soldier-0 finding 1.
2. **SGW.exe crash on every runner logout (infra, 7 BLOCKED rows: 818, 852, 1889, 2143, 1485, 1456, 1486).** Each batch boundary makes a `lab_logout` (`ensure_state` assumes the lab-account character), the client crashes (issue #1680), the heartbeat reads `unreachable` while status still says `in_world`, and later rows fail on the bridge until the watchdog relaunches. Full diagnosis and proposed code fix: [canary.md, soldier-1 degradation](../worknotes/canary.md). No new issue (tracked as #1680).
3. **`client_use_ability denied: ignored` on setup (9 BLOCKED rows: 2419, 808, 2103, 1882, 2102, 724, 1885, 2105, 1487).** Also happens on a healthy client, after the equip steps. Cause unknown (equip still running, cooldown, GCD); canary.md flags it for its own look. Not graded FAIL: the press never left the client.

## Not run / UNVERIFIED

- No PENDING SigNoz clauses in this file. Damage and targeting were not examined (`client_target_id 0` on every press).
- The `applied` client clause shows [FAILED] on 656 and 1483, consistent with the server refusals.

## Rerun list (16 rows, BLOCKED)

All 16 BLOCKED rows in the table above, reasons per row. Rerun after #1680 is fixed, or with the mitigation `character: "Cgbqjzujpscktwzf"` on each `lab_uat_run` call (canary.md, untested). Run the 9 ignored-press rows on a healthy client to see whether they repeat. Rerun Soldier-656 and 1483 after the seed fix.

## Attests

`lab_uat_attest {server_version: 6b50d9131417b8cf93feb61a7c640577398db567}` was applied under a short lease (no lease was held; released). No clause attests: the file has no PENDING clause.

## Recipe

SigNoz `signoz_search_logs`, filter `service.name = 'cimmeria-server' AND cimmeria.lab_run = 'dr-lab' AND scope_name = 'abilities' AND player_name CONTAINS 'Cgbqjzujpscktw'`, start 1793648361000, end 1793649540000. Seed facts from `resources.abilities` and `resources.effect_nvps` (read-only).

## Rerun 2026-11-02 (post DR-07k)

**Verified:** 2026-11-02, read-only (run bundle, SigNoz `cimmeria.lab_run = 'dr-lab'`, scope `abilities`). **Build:** `service.version` 6b50d9131417b8cf93feb61a7c640577398db567 (attest pending, see Attests).

Run dir `C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqkzlpgajqrl`, window 20:08:16Z to 20:15:41Z, fresh character Cgbqkzlpgajqrlzf, 17 rows (setup plus 16). Runner counts: PASS 4 (incl. setup), FAIL 1 (+ `_session_end`), BLOCKED 11, plus 1456 runner PASS. Server-truth grading: PASS 4, FAIL 2, BLOCKED 11. The `_session_end` logout failure (bridge connect) is infra, issue #1680, not a row.

| Row | Ability | Runner | Server evidence (SigNoz) | Rerun grade | Merged grade | Class |
|---|---|---|---|---|---|---|
| Soldier-00 | setup | PASS | world 1300 reached | PASS | PASS | |
| Soldier-2419 | Launch Grenade: Single | BLOCKED | no server row | BLOCKED | BLOCKED | setup press ignored |
| Soldier-808 | Cover Denial | BLOCKED | no server row | BLOCKED | BLOCKED | setup press ignored |
| Soldier-2103 | Launch Grenade: Concussion | BLOCKED | no server row | BLOCKED | BLOCKED | setup press ignored |
| Soldier-818 | Sustained Sweep | PASS | `ability_launched` 20:10:11.854 and 20:10:22.331 | PASS | PASS | first run BLOCKED (infra) |
| Soldier-1882 | Launch Grenade: Tear Gas | BLOCKED | no server row | BLOCKED | BLOCKED | setup press ignored |
| Soldier-2102 | Incinerate Ground | BLOCKED | no server row | BLOCKED | BLOCKED | setup press ignored |
| Soldier-724 | Anti-Personnel Mortar | BLOCKED | no server row | BLOCKED | BLOCKED | setup press ignored |
| Soldier-1885 | Launch Grenade: Shockwave | BLOCKED | no server row | BLOCKED | BLOCKED | setup press ignored |
| Soldier-852 | High Explosive Mortar | FAIL | no server row (press ignored: `client_use_ability denied: ignored`, ability.sent 0) | FAIL | FAIL | lab/client defect, press never left the client |
| Soldier-1889 | Incinerate Area | BLOCKED | no server row | BLOCKED | BLOCKED | setup press ignored |
| Soldier-2143 | Intimidation Fire | PASS | `ability_launched` 20:12:27.034 and 20:12:37.616 | PASS | PASS | first run BLOCKED (infra) |
| Soldier-1485 | Cluster Mortars | BLOCKED | no server row | BLOCKED | BLOCKED | setup press ignored |
| Soldier-2105 | Launch Grenade: Barrage | BLOCKED | no server row | BLOCKED | BLOCKED | setup press ignored |
| Soldier-1487 | Penetrating Barrage | BLOCKED | no server row | BLOCKED | BLOCKED | setup press ignored |
| Soldier-1456 | Shock and Awe | PASS (applied clause FAILED) | `ability_refused no_mechanics` 20:13:33.436 and 20:13:42.264; `no_mechanics_refused effect_count 0` | FAIL | FAIL | seed gap (no effects), runner PASS refused by server |
| Soldier-1486 | Rain of Steel | PASS | `ability_launched` 20:14:13.479 and 20:14:24.013 (`fire_target_skipped no_target`, client_target_id 0) | PASS | PASS | first run BLOCKED (infra) |

Rerun totals: PASS 4, FAIL 2, BLOCKED 11 (17 rows).

### Merged totals for the whole file (first run + rerun, 26 rows)

PASS 11 (00, 612, 632, 851, 658, 819, 1883, 853, 818, 2143, 1486), FAIL 4 (656 and 1483 seed gap from the first run, 1456 seed gap, 852 press ignored), BLOCKED 11 (2419, 808, 2103, 1882, 2102, 724, 1885, 1889, 1485, 2105, 1487; all setup press ignored, no server row).

### Findings

1. The #1680 logout crash no longer blocked rows: 818, 2143, 1456, 1486 ran after batch boundaries (DR-07k effect). Only `_session_end` still fails (infra, #1680).
2. The setup `client_use_ability denied: ignored` fault persists: 11 rows (10 setup-ignored plus 852's press) never reached the server. It is the main remaining blocker; root cause unknown.
3. Soldier-1456 Shock and Awe has `effect_count 0`: a new seed gap in the same family as 656 and 1483 (needs an issue body via `seed-change`).
4. Every launched ability shows `fire_target_skipped no_target` (client_target_id 0): launches proven, damage and targeting not examined.

### Rerun list (11 BLOCKED plus 852)

2419, 808, 2103, 1882, 2102, 724, 1885, 1889, 1485, 2105, 1487, 852: rerun only after the ignored-press cause is found. 656, 1483, 1456 after the seed fix.

### Attests (pending)

- `lab_uat_attest {server_version: 6b50d9131417b8cf93feb61a7c640577398db567}` for run_dir above (run.json `service_version` null). Not applied: read-only run, no lease acquired. No clause attests (no PENDING clauses).

## Ground-row rerun 2026-11-02 (post DR-07l/m)

**Verified:** 2026-11-02, read-only (run bundle, SigNoz `cimmeria.lab_run = 'dr-lab'` scope `abilities`, lab-server `server_db_query`). **Build:** `service.version` 6b50d9131417b8cf93feb61a7c640577398db567 (run.json `service_version` null; attest not applied, see Attests).

Run dir `C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqmyhigfcmah`, window 21:05:32Z to 21:21:08Z, fresh character Cgbqmyhigfcmahzf, 13 rows plus `_session_end` (all PASS, 14 PASS by `lab_uat_report`). No client crash, no ignored press: the ground-click path (`use_ability_on_ground_recv`) reached the server for every row (DR-07l/m worked). Server-truth grading: PASS 11 (incl. setup), FAIL 2 (runner PASS refused by the server), BLOCKED 0.

| Row | Ability | Runner | Server evidence (SigNoz) | Grade | Class |
|---|---|---|---|---|---|
| Soldier-00 | setup | PASS | world 1300 reached | PASS | |
| Soldier-2419 | Launch Grenade: Single | PASS | `use_ability_on_ground_recv` + `ability_launched` 21:07:12.052, 21:07:38.534 | PASS | |
| Soldier-808 | Cover Denial | PASS | `ability_launched` 21:08:20.936, 21:08:56.922 | PASS | |
| Soldier-2103 | Launch Grenade: Concussion | PASS | `ability_launched` 21:09:45.846, 21:10:13.768 | PASS | |
| Soldier-1882 | Launch Grenade: Tear Gas | PASS (applied clause FAILED) | `ability_refused no_mechanics` 21:11:07.765, 21:11:34.022; `no_mechanics_refused effect_count 0` | FAIL | seed gap: abilities.effect_ids empty, 0 effects |
| Soldier-2102 | Incinerate Ground | PASS (applied clause FAILED) | `ability_refused no_mechanics` 21:12:24.296, 21:12:51.504; `effect_count 0` | FAIL | seed gap: abilities.effect_ids empty, 0 effects |
| Soldier-724 | Anti-Personnel Mortar | PASS | `ability_launched` 21:13:30.877, 21:13:56.914 | PASS | |
| Soldier-1885 | Launch Grenade: Shockwave | PASS | `ability_launched` 21:14:42.918, 21:15:08.496 | PASS | |
| Soldier-852 | High Explosive Mortar | PASS | `ability_launched` 21:15:43.265, 21:16:12.363 | PASS | fixed (was ignored press) |
| Soldier-1889 | Incinerate Area | PASS | `ability_launched` 21:16:55.648, 21:17:19.888; `effect_routed resolve_on_ability_user` x2 each | PASS | |
| Soldier-1485 | Cluster Mortars | PASS | `ability_launched` 21:17:53.291, 21:18:21.084 (target_id 0); `fire_target_skipped no_target` | PASS | max_range 0, launched |
| Soldier-2105 | Launch Grenade: Barrage | PASS | `ability_launched` 21:19:06.391, 21:19:33.683 | PASS | |
| Soldier-1487 | Penetrating Barrage | PASS | `ability_launched` 21:20:15.584, 21:20:43.026 (target_id 0); `fire_target_skipped no_target` | PASS | max_range 0, launched |

Note on 1485/1487: both have `max_range` 0 in the seed and launched with no target (`client_target_id 0`); launch proven, no QR roll or damage (`fire_target_skipped no_target`). Seed facts: 1485 has 4 effects/4 NVPs, 1487 has 2 effects/2 NVPs, 1882 and 2102 have none.

### Merged final grade, all 26 soldier-1 rows (first run + reruns)

PASS 21, FAIL 5, BLOCKED 0 of 26.

| Row | Ability | Final | Source run |
|---|---|---|---|
| Soldier-00 | setup | PASS | ground rerun |
| Soldier-612 | Area Burst | PASS | first run |
| Soldier-632 | Long Burst | PASS | first run |
| Soldier-851 | Brace | PASS | first run |
| Soldier-658 | Heavy Suppression | PASS | first run |
| Soldier-819 | Steady Aim | PASS | first run |
| Soldier-656 | Sustained Bursts | FAIL | first run, seed gap (no NVPs, `no_mechanics`) |
| Soldier-1883 | Incinerating Stream | PASS | first run |
| Soldier-1483 | Steady Launch | FAIL | first run, seed gap (no NVPs, `no_mechanics`) |
| Soldier-853 | Fire Zone | PASS | first run |
| Soldier-2419 | Launch Grenade: Single | PASS | ground rerun |
| Soldier-808 | Cover Denial | PASS | ground rerun |
| Soldier-2103 | Launch Grenade: Concussion | PASS | ground rerun |
| Soldier-818 | Sustained Sweep | PASS | rerun 1 (DR-07k) |
| Soldier-1882 | Launch Grenade: Tear Gas | FAIL | ground rerun, seed gap (no effects) |
| Soldier-2102 | Incinerate Ground | FAIL | ground rerun, seed gap (no effects) |
| Soldier-724 | Anti-Personnel Mortar | PASS | ground rerun |
| Soldier-1885 | Launch Grenade: Shockwave | PASS | ground rerun |
| Soldier-852 | High Explosive Mortar | PASS | ground rerun (rerun 1 FAIL superseded) |
| Soldier-1889 | Incinerate Area | PASS | ground rerun |
| Soldier-2143 | Intimidation Fire | PASS | rerun 1 |
| Soldier-1485 | Cluster Mortars | PASS | ground rerun |
| Soldier-2105 | Launch Grenade: Barrage | PASS | ground rerun |
| Soldier-1487 | Penetrating Barrage | PASS | ground rerun |
| Soldier-1456 | Shock and Awe | FAIL | rerun 1, seed gap (no effects, `no_mechanics`) |
| Soldier-1486 | Rain of Steel | PASS | rerun 1 |

### Findings

1. New real FAILs: 1882 Launch Grenade: Tear Gas and 2102 Incinerate Ground have empty `effect_ids` (0 effects) so the gate refuses them `no_mechanics`. The runner PASSed both because only the `sent` clause counts; its `applied` clause was [FAILED]. Same family as 656, 1483, 1456. Issue body: [issues/DR-L2-soldier-1-ground-no-effects.md](issues/DR-L2-soldier-1-ground-no-effects.md). Fix via `seed-change`.
2. All 5 remaining FAILs are seed gaps (no effects or no NVPs): 656, 1483, 1456, 1882, 2102. No BLOCKED rows remain.
3. Every launched ability shows `fire_target_skipped no_target` where logged (client_target_id 0): launch proven, damage and targeting not examined.
4. Ground rows: the server logged `use_ability_on_ground_recv` before each outcome.

### Attests (pending)

`lab_uat_attest {server_version: 6b50d9131417b8cf93feb61a7c640577398db567}` for the run dir above. Not applied: read-only grading. No PENDING clauses.

### Recipe

SigNoz `signoz_search_logs`, filter `cimmeria.lab_run = 'dr-lab' AND scope_name = 'abilities' AND player_name CONTAINS 'Cgbqmyhigfcmah' AND event != 'warmup_pending' AND event EXISTS`, start 1793653500000, end 1793654300000, plus an unfiltered 400-row pull to 1793654500000 for 2105 and 1487. Seed: `resources.abilities` joined to `resources.effects` and `resources.effect_nvps`.
