# DR-L5-archeologist-2: Archeologist abilities, tree 2 (armory-abilities-archeologist-2)

> **Type:** results. **Audience:** the orchestrator and architect. **Verified:** 2026-11-03, read-only (run bundle, SigNoz `cimmeria.lab_run = 'dr-lab'`, lab-server `server_db_query`). **Build:** `service.version` 6b50d9131417b8cf93feb61a7c640577398db567 (attested, see Attests). Method: [lab-run-plan.md, Verifier steps](../lab-run-plan.md), [DR-L5-archeologist-1.md](DR-L5-archeologist-1.md).

**Recommendation: findings, then rerun of 815 after a seed fix.** 26 rows: 2 PASS (setup, session end), 23 PARTIAL (passives, refused by design), 1 FAIL (815, active, refused `no_mechanics`), 0 BLOCKED. The runner graded every ability row PASS, but the server launched none of them: there is no `ability_launched` event in the whole window, and every ability row shows `ability_refused no_mechanics`. The runner PASS only proves the press left the client.

## Runs

| Call | Spec or split file | plan_only | run_dir | Outcome |
|---|---|---|---|---|
| 1 (one run, instance p2) | armory-abilities-archeologist-2 | no | `C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-03-unknown-cgbqztyvkwlksr` | runner PASS 26 (24 ability rows, 00, session end); every ability row also shows `[FAILED] client applies ability after press (events [])` inside a PASS |

Window 03:21:28Z to 03:35:16Z, fresh character Cgbqztyvkwlksrjk (Archeologist), instance p2. Earlier attempts (run dirs ...cgbqzhyqonvlqrjk and ...cgbqzhzvccgvip) are infrastructure noise and were ignored.

## Per-row grades

Server truth = SigNoz scope `abilities`, one aggregate by `ability_id`, `event`, `reason`. Each row presses twice, so counts are 2. Every ability row has `use_ability_recv` x2, `ability_refused no_mechanics` x2 and `no_mechanics_refused` x2. The `passive` flag is the attribute on `no_mechanics_refused`; the passive flag and effect lists are confirmed from `resources.abilities`.

| Row | Ability | Runner | Server evidence | Grade | Class |
|---|---|---|---|---|---|
| Archeologist-00 | setup | PASS | world 1300 reached (runner tool clause) | PASS |  |
| Archeologist-677 | Mini-game: Delay | PASS | use_ability_recv x2, ability_refused no_mechanics x2, passive=true, 0 effects | PARTIAL | passive by design |
| Archeologist-678 | Mini-game: In Situ Context | PASS | same, passive=true, 0 effects | PARTIAL | passive by design |
| Archeologist-679 | Mini-game: Effigy Transfer | PASS | same, passive=true, 0 effects | PARTIAL | passive by design |
| Archeologist-680 | Mini-game: Contextual Guide | PASS | same, passive=true, 0 effects | PARTIAL | passive by design |
| Archeologist-681 | Mini-game: Grave Goods | PASS | same, passive=true, 0 effects | PARTIAL | passive by design |
| Archeologist-682 | Mini-game: Totem Swap | PASS | same, passive=true, 0 effects | PARTIAL | passive by design |
| Archeologist-683 | Mini-game: Old Rite | PASS | same, passive=true, 0 effects | PARTIAL | passive by design |
| Archeologist-684 | Mini-game: Re-Survey | PASS | same, passive=true, 0 effects | PARTIAL | passive by design |
| Archeologist-685 | Mini-game: Epiphany | PASS | same, passive=true, 0 effects | PARTIAL | passive by design |
| Archeologist-686 | Mini-game: Ethnographic Drift | PASS | same, passive=true, 0 effects | PARTIAL | passive by design |
| Archeologist-802 | Reveal Mini-Games | PASS | same, passive=true, 0 effects | PARTIAL | passive by design |
| Archeologist-809 | Mental Fortitude | PASS | same, passive=true, effect 854 | PARTIAL | passive by design |
| Archeologist-811 | Concentration: Focused | PASS | same, passive=true, effect 856 | PARTIAL | passive by design |
| Archeologist-813 | Impressive Victory | PASS | same, passive=true, effect 859 | PARTIAL | passive by design |
| Archeologist-815 | Battle Avoidance | PASS | same events, passive=false, effect_count 3 (4383, 4382, 861), 0 NVPs, no ability_launched | FAIL | seed gap |
| Archeologist-817 | Ancient Recognition | PASS | same, passive=true, effect 863 | PARTIAL | passive by design |
| Archeologist-1193 | Concentration: Intense | PASS | same, passive=true, effect 1361 | PARTIAL | passive by design |
| Archeologist-1194 | Concentration: Unbreakable | PASS | same, passive=true, effect 1362 | PARTIAL | passive by design |
| Archeologist-1195 | Flawless Victory | PASS | same, passive=true, effect 1363 | PARTIAL | passive by design |
| Archeologist-1196 | Unacceptable Defeat | PASS | same, passive=true, effect 1364 | PARTIAL | passive by design |
| Archeologist-1197 | Ancient Understanding | PASS | same, passive=true, effect 1365 | PARTIAL | passive by design |
| Archeologist-1552 | Reveal Research Targets | PASS | same, passive=true, 0 effects | PARTIAL | passive by design |
| Archeologist-2507 | Reduce Distractions | PASS | same, passive=true, effect 3663 | PARTIAL | passive by design |
| Archeologist-2508 | Remove Distractions | PASS | same, passive=true, effect 3664 | PARTIAL | passive by design |
| _session_end | none | PASS | not an ability row (runner clause only; server session.end not queried: UNKNOWN) | PASS |  |

Totals: PASS 2, PARTIAL 23, FAIL 1, BLOCKED 0 = 26 rows. No weapon-moniker DATA-GAP: no row needs a weapon.

## Observation: client-applied event missing

All 24 ability rows carry `[FAILED] the client applies the ability after the press` with `ability.applied` `matching_events: 0` and `events: []`, while the press itself left the client (`useAbility` seen, server `use_ability_recv` x2 for every row). For 23 rows the server accepted the press and refused it by design; for 815 it refused `no_mechanics`. So the server never accepted an ability here, and the missing client-applied event is consistent with that. Whether the p2 event capture would record `ability.applied` for an accepted ability is UNKNOWN from this run (archeologist-1's accepted rows on the default instance are the comparison, not re-checked). Candidate p2 event-capture issue only if accepted rows on p2 also show no applied event; no accepted row exists in this run to tell.

Every row also flagged `teardown uat_clear_effects failed: client_target ... no rendered entity matches` (seen in Archeologist-677.json; not read for every row). Not graded.

## Server-side defects

One defect: Battle Avoidance (815) refused `no_mechanics`. Effects 4383 Battle Avoidance, 4382 Remove Stance, 861 Check for Mini-Game stance: no effect NVPs, no script_name. Issue: [DR-L5-archeologist-2-no-mechanics.md](issues/DR-L5-archeologist-2-no-mechanics.md).

The 23 passives refused `no_mechanics` / `passive_ability` is by design (a passive has nothing to press); that no passive is applied some other way (equip/learn) is UNKNOWN and not tested here.

## NEEDS_HUMAN

None.

## Not run

None. All 26 rows were pressed.

## Resume hint

Nothing to resume. Archeologist files 0, 1, 2 are all graded.

## Rerun list

- After the seed fix (`seed-change`): 815.
- Passives (677-686, 802, 809, 811, 813, 817, 1193-1197, 1552, 2507, 2508): no rerun; a spec that presses passives cannot launch (spec question for the architect, not an issue).

## Attests

DONE 2026-11-03 03:36Z under lease `dr-verify-archeologist-2` on instance p2 (no lease was held), released at once: `lab_uat_attest {server_version: 6b50d9131417b8cf93feb61a7c640577398db567}` for this run dir. Returned the version. Only p2 touched. No clause attests pending. `lab_uat_report` (`ledger: true`) was not run, so the counts here are from the ledger and my own grading and are not cross-checked against the runner's report (incomplete).

## Recipe

SigNoz `signoz_aggregate_logs`, filter `service.name = 'cimmeria-server' AND cimmeria.lab_run = 'dr-lab' AND scope_name = 'abilities' AND player_name CONTAINS 'Cgbqztyvkwlksrjk' AND event EXISTS AND event != 'warmup_pending'`, start 1793676080000, end 1793676930000, count grouped by ability_id, event, reason (no `ability_launched` row; no `ability_refused` other than no_mechanics). Second query: same window, `event = 'no_mechanics_refused'`, grouped by ability_id, ability_name, passive, effect_count. `service.version` read from a 815 log row (resource attribute). Seed: `server_db_query` on `resources.abilities` (`passive_yn`, `effect_ids`) with a count of `resources.effect_nvps` and `string_agg` of `resources.effects` names and `script_name`. Seed file and line: UNKNOWN (graded from the live DB). `warmup_complete` not queried.

## Driver reports

Not supplied to the verifier; graded from the bundle only.
