# DR-L3-commando-1: Commando abilities, tree 1 (armory-abilities-commando-1)

> **Type:** results. **Audience:** the orchestrator and architect. **Verified:** 2026-11-02, read-only (run bundle, SigNoz `cimmeria.lab_run = 'dr-lab'`, lab-server `server_db_query`). **Build:** `service.version` 6b50d9131417b8cf93feb61a7c640577398db567, attested for this run and for commando-0 (see Attests). Method: [lab-run-plan.md, Verifier steps](../lab-run-plan.md), [DR-L3-commando-0.md](DR-L3-commando-0.md).

**Recommendation: findings, then rerun after a seed fix.** 7 of 23 rows PASS on server truth (incl. setup). 7 are FAIL (the gate refuses them `no_mechanics`), 5 are PARTIAL (passives refused by design), 4 are BLOCKED (3 DATA-GAP, 1 press ignored). The runner called 19 rows PASS; only Commando-860 failed, and for the same seed reason.

## Runs

| Call | Spec or split file | plan_only | run_dir | Outcome |
|---|---|---|---|---|
| 1-4 (6 rows each, `end_session: false`) | armory-abilities-commando-1 | no | `C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqqpcwwamins` | runner PASS 19, FAIL 1 (Commando-860), BLOCKED 4 (880, 1246, 1250, 1355) |

Window 22:52Z to 23:04Z, fresh character Cgbqqpcwwaminsis (Commando). The ledger has 23 ability rows plus `_session_end`.

## Per-row grades

Server truth = SigNoz scope `abilities` over the window (count grouped by `ability_id`, `event`, `reason`; every event twice because each row presses twice). Seed columns from `resources.abilities` and `resources.effect_nvps`.

| Row | Ability | Runner | Server evidence | Grade | Class |
|---|---|---|---|---|---|
| Commando-00 | setup | PASS | world 1300 reached | PASS |  |
| Commando-641 | Surprise Attack | PASS | use_ability_recv, ability_launched x2 (fire_target_skipped no_target x2) | PASS |  |
| Commando-651 | Wounding Strike | PASS | ability_launched x2 (fire_target_skipped no_target x2) | PASS |  |
| Commando-863 | Blinding Strike | PASS ([FAILED] applied) | ability_launched x2 (fire_target_skipped no_target x2); client applied clause failed | PASS | server launched; client apply not seen |
| Commando-873 | Sap | PASS ([FAILED] applied) | ability_launched x2 (fire_target_skipped no_target x2) | PASS | server launched; client apply not seen |
| Commando-1356 | Evisceration | PASS | ability_launched x2 (fire_target_skipped no_target x2) | PASS |  |
| Commando-1357 | Execution | PASS | ability_launched x2 (fire_target_skipped no_target x2) | PASS |  |
| Commando-649 | Reveal I | PASS ([FAILED] applied) | ability_refused no_mechanics x2 | FAIL | seed gap (2 effects, 0 NVPs) |
| Commando-860 | Marked Prey | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (1 effect 923, 0 NVPs) |
| Commando-872 | Fade | PASS ([FAILED] applied) | ability_refused no_mechanics x2 | FAIL | seed gap (1 effect 942, 0 NVPs) |
| Commando-874 | Reveal II | PASS ([FAILED] applied) | ability_refused no_mechanics x2 | FAIL | seed gap (2 effects, 0 NVPs) |
| Commando-875 | Reveal III | PASS ([FAILED] applied) | ability_refused no_mechanics x2 | FAIL | seed gap (2 effects, 0 NVPs) |
| Commando-1244 | Reveal IV | PASS ([FAILED] applied) | ability_refused no_mechanics x2 | FAIL | seed gap (2 effects, 0 NVPs) |
| Commando-1253 | Pulse Sensor | PASS ([FAILED] applied) | ability_refused no_mechanics x2 | FAIL | seed gap (effect_ids empty) |
| Commando-646 | Stealth I | PASS ([FAILED] applied) | no_mechanics_refused passive_ability x2 | PARTIAL | passive (design) |
| Commando-1518 | Stealth II | PASS ([FAILED] applied) | no_mechanics_refused passive_ability x2 | PARTIAL | passive (design) |
| Commando-865 | Stealth III | PASS ([FAILED] applied) | no_mechanics_refused passive_ability x2 | PARTIAL | passive (design) |
| Commando-866 | Stealth IV | PASS ([FAILED] applied) | no_mechanics_refused passive_ability x2 | PARTIAL | passive (design) |
| Commando-1254 | Blind Sense | PASS ([FAILED] applied) | no_mechanics_refused passive_ability x2 | PARTIAL | passive (design) |
| Commando-1246 | Stealthed Strike | BLOCKED | no events (not pressed) | BLOCKED | DATA-GAP: requires ITEM_Melee, 0 items carry it |
| Commando-1355 | Lethal Strike | BLOCKED | no events (not pressed) | BLOCKED | DATA-GAP: requires ITEM_Melee, 0 items carry it |
| Commando-1250 | Escape | BLOCKED | no events (not pressed) | BLOCKED | DATA-GAP/audit: needs the five ITEM_Stealth_* armor pieces, not weapons |
| Commando-880 | Sweep Detector | BLOCKED (setup press ignored) | no events at all | BLOCKED | press ignored; see Findings 4 |

Totals: PASS 7 (incl. setup), FAIL 7, PARTIAL 5, BLOCKED 4, 23 rows.

## Findings

1. **Seed gap, 7 rows (real defect):** 649, 860, 872, 874, 875, 1244 have effects but `resources.effect_nvps` holds no NVP for any of them; 1253 (Pulse Sensor) has empty `effect_ids`. The gate refuses every one `no_mechanics`. The runner PASSed 6 of them because only the `sent` clause counted. Issue: [DR-L3-commando-1-no-mechanics.md](issues/DR-L3-commando-1-no-mechanics.md). Same class as [DR-L3-commando-0-no-mechanics.md](issues/DR-L3-commando-0-no-mechanics.md); fix both in one `seed-change` pass.
2. **Passives, 5 rows:** the Stealth I-IV ability chain (646, 1518, 865, 866) and Blind Sense (1254) are `passive_yn = true`, refused `passive_ability` by design. PARTIAL.
3. **DATA-GAP, ITEM_Melee (1246 Stealthed Strike, 1355 Lethal Strike):** both list `item_monikers = [3756430796]` (`ITEM_Melee`). `resources.monikers` has the name, but `resources.items.moniker_ids` has 0 items carrying it. The working blade abilities (641, 651, 1356, 1357) require `ITEM_Blade` (830336901), carried by 53 items (the Heavy Combat Knife among them), and launched. So the two rows can never pass until a weapon gets the `ITEM_Melee` moniker or the abilities' requirement is corrected to `ITEM_Blade`: a data decision (like GW-3128 in the plan), not a lab fault. Both abilities have effects and NVPs (2 and 4), so they would otherwise run. Issue: [DR-L3-commando-1-item-melee-data-gap.md](issues/DR-L3-commando-1-item-melee-data-gap.md).
4. **Escape 1250:** `item_monikers` are the five `ITEM_Stealth_Chest/Feet/Hands/Head/Legs` monikers. Each is carried by 27 items in `resources.items`, and the sample is armor (Veiled Stealth Vest, tier 2), not a weapon. The runner's "weapon-requirement audit" looks at weapons only, so BLOCKED is a runner audit mismatch rather than a missing item: the row can run with a full stealth armor set equipped (grant and equip, as the DR-L10 armor rows do). Escape also has 0 NVPs for effects 1418 and 1417, so it would very likely be refused `no_mechanics` as well; it is in the issue 1 rerun list. Needs an architect ruling on how the runner should audit armor requirements.
5. **Sweep Detector 880 (press ignored):** `type_id ABILITY_TYPE_Undefined`, `target_type_id 3` (ground/area target), empty `effect_ids`. The setup `client_use_ability` was refused by the client `ignored: ... nothing changed ... something is blocking it`, with zero server events, so the press never left the client. Cause is not proven: the ground-target press is the same shape as the soldier "flushing fire press ignored" case ([DR-L2-soldier-0-flushing-fire-press-ignored.md](issues/DR-L2-soldier-0-flushing-fire-press-ignored.md)), but commando-0 ground rows (523, 647, 861, 1364) pressed fine. Rerun it alone first. If it fires, expect `no_mechanics` (no effects) and add it to the seed issue.
6. **Working abilities:** 641, 651, 1356, 1357 (blade strikes) and 863, 873 launch. `fire_target_skipped no_target` appears on all of them while the launch still happens. 863 and 873 had the client `applied` clause FAILED but the server launched, so they are graded PASS on server truth with that note.

## Not run / UNVERIFIED

No PENDING clauses. No row is UNVERIFIED. 1246, 1250, 1355 were never pressed (runner audit), 880 was never sent.

## Rerun list

- After the seed fix (`seed-change`): 649, 860, 872, 874, 875, 1244, 1253, plus 1250 (Escape) once the runner can equip the stealth armor set.
- After a data decision on `ITEM_Melee`: 1246, 1355.
- Rerun alone now: 880 (press ignored).
- No rerun for the 5 passives or the 7 PASS rows.

## Attests

Done 2026-11-02 23:06Z: the lab lease was free (last holder `lab_uat_run`, released 23:04:39Z). The verifier took a 120 s lease, ran `lab_uat_attest {server_version: 6b50d9131417b8cf93feb61a7c640577398db567}` for both run dirs (commando-0 `2026-11-02-unknown-cgbqpzpxnjqkjl`, commando-1 `2026-11-02-unknown-cgbqqpcwwamins`), then released the lease at once. No clause attests were pending.

## Recipe

SigNoz `signoz_aggregate_logs`, filter `service.name = 'cimmeria-server' AND cimmeria.lab_run = 'dr-lab' AND scope_name = 'abilities' AND player_name CONTAINS 'Cgbqqpcwwamins' AND event EXISTS AND event != 'warmup_pending'`, start 1793659900000, end 1793660800000, count grouped by ability_id, event, reason. Seed: `server_db_query` on `resources.abilities` (`effect_ids`, `item_monikers`, `passive_yn`, `target_type_id`) with a count of `resources.effect_nvps` rows whose `effect_id` is in `effect_ids`; `resources.items.moniker_ids` counted per moniker id.
