# DR-L5-archeologist-0: Archeologist abilities, tree 0 (armory-abilities-archeologist-0)

> **Type:** results. **Audience:** the orchestrator and architect. **Verified:** 2026-11-03, read-only (run bundle, SigNoz `cimmeria.lab_run = 'dr-lab'`, lab-server `server_db_query`). **Build:** `service.version` 6b50d9131417b8cf93feb61a7c640577398db567 (attest pending, see Attests). Method: [lab-run-plan.md, Verifier steps](../lab-run-plan.md), [DR-L4-scientist.md](DR-L4-scientist.md).

**Recommendation: findings, then rerun after a seed fix.** 4 of 22 rows PASS on server truth (setup + 3 launches: 788, 789, 2099). 14 are FAIL (all refused `no_mechanics`), 4 PARTIAL (passives), 0 BLOCKED. The runner called 17 PASS (incl. the session-end row) and 6 FAIL (746, 781, 791, 1081, 1191, 1192); 8 of its PASS rows are server refusals, and the client `applied` clause FAILED on 13 of its PASS rows.

## Runs

| Call | Spec or split file | plan_only | run_dir | Outcome |
|---|---|---|---|---|
| 1-4 (6 rows each, then session end) | armory-abilities-archeologist-0 | no | `C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-03-unknown-cgbqyvjnrgvlvq` | runner PASS 17, FAIL 6 |

Window 02:53Z to 03:06Z, fresh character Cgbqyvjnrgvlvqsi (Archeologist).

## Per-row grades

Server truth = SigNoz scope `abilities` over the window, one aggregate by `ability_id`, `event`, `reason` (each row presses twice, so counts are 2). Seed from `resources.abilities`, `resources.effects`, `resources.effect_nvps`.

| Row | Ability | Runner | Server evidence | Grade | Class |
|---|---|---|---|---|---|
| Archeologist-00 | setup | PASS | world 1300 reached | PASS |  |
| Archeologist-788 | Restore: Concentration | PASS | ability_launched x2, beneficial_cast_applied x2, heal_focus x2 | PASS |  |
| Archeologist-789 | Restore: Will | PASS | ability_launched x2, beneficial_cast_applied x2, heal_focus x2 | PASS |  |
| Archeologist-2099 | Clear: Mind | PASS | ability_launched x2, beneficial_cast_applied x2, remove_effects_applied x2 | PASS |  |
| Archeologist-778 | Conversation: Mediator | PASS | no_mechanics_refused passive_ability x2 (also ability_refused no_mechanics x2) | PARTIAL | passive (design) |
| Archeologist-779 | Conversation: Negotiator | PASS | no_mechanics_refused passive_ability x2 | PARTIAL | passive (design) |
| Archeologist-792 | Conversation: Diplomat | PASS | no_mechanics_refused passive_ability x2 | PARTIAL | passive (design) |
| Archeologist-1184 | Indistinguishable | PASS | no_mechanics_refused passive_ability x2 | PARTIAL | passive (design) |
| Archeologist-746 | Conversation: Humanoid: Experienced | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (1 effect, 0 NVPs) |
| Archeologist-781 | Enhance: Humanoid: Focus | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (2 effects, 0 NVPs) |
| Archeologist-791 | Enhance Group: Humanoid: Focus | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (3 effects, 0 NVPs) |
| Archeologist-1081 | Disguise: Humanoid: Mark I | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (3 effects, 0 NVPs) |
| Archeologist-1191 | Rethink Aggression | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (1 effect, 0 NVPs) |
| Archeologist-1192 | Clear: Disruptions | FAIL (applied) | ability_refused no_mechanics x2 | FAIL | seed gap (1 effect, 0 NVPs) |
| Archeologist-784 | Non-Threatening Visage | PASS | ability_refused no_mechanics x2 | FAIL | seed gap (1 effect, 0 NVPs) |
| Archeologist-1082 | Disguise: Mark II | PASS | ability_refused no_mechanics x2 | FAIL | seed gap (3 effects, 0 NVPs) |
| Archeologist-1083 | Disguise: Mark III | PASS | ability_refused no_mechanics x2 | FAIL | seed gap (3 effects, 0 NVPs) |
| Archeologist-1183 | Blend In | PASS | ability_refused no_mechanics x2 | FAIL | seed gap (1 effect, 0 NVPs) |
| Archeologist-1187 | Disguise: Alien: Mark II | PASS | ability_refused no_mechanics x2 | FAIL | seed gap (2 effects, 0 NVPs) |
| Archeologist-1188 | Disguise: Alien: Mark III | PASS | ability_refused no_mechanics x2 | FAIL | seed gap (2 effects, 0 NVPs) |
| Archeologist-1190 | Goldam's Gift | PASS | ability_refused no_mechanics x2 | FAIL | seed gap (2 effects, 0 NVPs) |
| Archeologist-2074 | Motivation | PASS | ability_refused no_mechanics x2 | FAIL | seed gap (5 effects, 0 NVPs) |

Totals: PASS 4, PARTIAL 4, FAIL 14, BLOCKED 0 = 22 rows. The 8 runner-PASS FAIL rows (784, 1082, 1083, 1183, 1187, 1188, 1190, 2074) were refused by the server.

## Server-side defects

One distinct defect: 14 abilities refused `no_mechanics`. Every one has effects but no `effect_nvps` rows and no `script_name` on any effect, so the gate finds no mechanics. Issue: [DR-L5-archeologist-0-no-mechanics.md](issues/DR-L5-archeologist-0-no-mechanics.md). What each effect lacks, for the seed plan:

| Ability | Effects (name, all `TCM_Single` unless noted; no NVPs, no script) | Needs |
|---|---|---|
| 746 | 792 Engagement State (pulse_count 25) | conversation-state mechanic (script) |
| 781 | 1414 Enhance Focus Buff, 824 Remove Focus Buffs | focus buff NVPs, script |
| 791 | 4477 Enhance Focus Group Buff, 1415 Remove Stance, 837 Check for mechanicals (`TCM_Group`) | group focus buff NVPs, script |
| 784 | 830 Disguise Heal | heal NVPs (HealFocus style) or disguise state |
| 1081 | 3652 VFX, 1230 Disguise I, 1229 Disguise rating +100 | disguise state/rating NVPs |
| 1082 | 4550 VFX, 1232 Disguise II, 1231 rating +200 | same |
| 1083 | 4551 VFX, 1234 Disguise III Activate, 1233 rating +300 | same |
| 1183 | 1346 Disguise Heal | disguise/heal NVPs |
| 1187 | 1352 rating +200 (pulse_count 0), 1351 Disguise II | disguise NVPs |
| 1188 | 1354 rating +300 (pulse_count 0), 1353 Disguise III Activate | disguise NVPs |
| 1190 | 1358 Revive Focus Heal, 1357 Revive Health Heal | HealFocus / health heal NVPs (HealPercentage) |
| 1191 | 1359 Induce Calm: Humanoid Threat Wipe | threat wipe script |
| 1192 | 1360 Focus Degeneration Heal | HealFocus-style NVPs |
| 2074 | 5306 Motivation, 4479 and 4478 Remove Stance, 2758 Motivation, 2759 Aura (`TCM_Aura`) | aura/stance buff NVPs |

Reference for what works: 834 and 835 carry `HealPercentage` with `script_name HealFocus`; 2827 carries `RemoveCategories`, `RemovePolarity` with `RemoveEffects`. Passive rows 778, 779, 792 have empty `effect_ids`; 1184 is `passive_yn` with effect 1347 (Disguise Heal, 0 NVPs).

## NEEDS_HUMAN

None.

## Not run

None. All 22 rows were pressed.

## Resume hint

Nothing to resume. Next file in this packet: armory-abilities-archeologist-1 (drive with the same template, a fresh run).

## Rerun list

- After the seed fix (`seed-change`): the 14 FAIL rows 746, 781, 784, 791, 1081, 1082, 1083, 1183, 1187, 1188, 1190, 1191, 1192, 2074.
- No rerun for 788, 789, 2099, 00 (PASS) or the 4 passives (778, 779, 792, 1184).
- The client `applied` clause failing on server-launched rows is not a rerun reason; for the 3 PASS rows the client did apply (cooldown outcome seen).

## Attests

DONE 2026-11-03 03:22Z. Not run in this file (a driver was running archeologist-1, R5); the archeologist-1 verifier ran `lab_uat_attest {server_version: 6b50d9131417b8cf93feb61a7c640577398db567}` for `%LOCALAPPDATA%\cimmeria-lab\uat-runs\2026-11-03-unknown-cgbqyvjnrgvlvq` under lease `dr-verify-archeologist-1` and it returned the version ([DR-L5-archeologist-1.md](DR-L5-archeologist-1.md#attests)). No clause attests pending (no PENDING SigNoz clauses in this file).

## Recipe

SigNoz `signoz_aggregate_logs`, filter `service.name = 'cimmeria-server' AND cimmeria.lab_run = 'dr-lab' AND scope_name = 'abilities' AND player_name CONTAINS 'Cgbqyvjnrgvlvqsi' AND event EXISTS AND event != 'warmup_pending'`, start 1793674400000, end 1793675250000, count grouped by ability_id, event, reason. Seed: `server_db_query` on `resources.abilities` (`effect_ids`, `passive_yn`, `target_type_id`, `required_ammo`) with a count of `resources.effect_nvps` whose `effect_id` is in `effect_ids`, then `resources.effects` (`name`, `script_name`, `target_collection_method`, `pulse_count`) with its NVPs aggregated. `service.version` was not read from SigNoz for this run's window; the attest above returned the same build for this run dir, and that is the only evidence for it here.

## Driver reports

Not supplied to the verifier; graded from the bundle only.
