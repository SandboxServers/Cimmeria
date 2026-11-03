# DR-L2-soldier-0: Soldier abilities, tree 0 (armory-abilities-soldier-0)

> **Type:** results. **Audience:** the orchestrator and architect. **Verified:** 2026-11-02, read-only (run bundle, `lab_uat_report`, SigNoz `cimmeria.lab_run = 'dr-lab'`, lab-server `server_db_query`). **Build:** `service.version` 6b50d9131417b8cf93feb61a7c640577398db567 (read from SigNoz; not attested, see Pending attests). Method: [lab-run-plan.md § Verifier steps](../lab-run-plan.md), contract C5/C6/C8, [DR-L2-canary.md](DR-L2-canary.md).

**Recommendation (final, after the rerun): findings only, no lab defect blocks the next file.** Merged totals: PASS 15 (incl. setup), PARTIAL 5, FAIL 3, BLOCKED 0. The 3 FAIL rows are a seed gap (three damage abilities whose effects carry no damage NVPs, so the server refuses them as `no_mechanics`); the 5 PARTIAL rows are passive abilities the server refuses by design. First run (superseded): 5 non-passing rows, of which 1 was an unexplained client-side press miss (1475, passes in the rerun after DR-07l/m) and 1 a session-end bridge blip (`_session_end` passes in the rerun).

## Runs

| Call | Spec or split file | plan_only | run_dir | Outcome |
|---|---|---|---|---|
| 1 | armory-abilities-soldier-0 (Soldier-00, 598, 717, 715, 716, 714) | no | 2026-11-02-unknown-cgbqjljvxcsrgh | 6 PASS. Fresh character Cgbqjljvxcsrghdj created, world 1300 reached. |
| 2 | same (657, 719, 891, 718, 720, 713) | no | same | 5 PASS, 718 FAIL. |
| 3 | same (723, 725, 1474, 867, 722, 1879) | no | same | 5 PASS, 725 FAIL. |
| 4 | same (1446, 1475, 1445, 1476, 1477) + end of session | no | same | 1446, 1445, 1476 PASS; 1475 BLOCKED; 1477 FAIL; `_session_end` FAIL. |

Run window 19:23:07Z to 19:38:59Z. Report counts (`lab_uat_report`): PASS 19, FAIL 4, BLOCKED 1 (24 rows incl. `_session_end`; the run bundle holds 23 spec rows plus the end step). Spec blocked rows: 0.

## Per-row grades

Server truth = SigNoz scope `abilities`, `player_name` = the fresh character, events `use_ability_recv`, `ability_launched`, `ability_refused`, matched by `ability_id` and the row window. Each pressed row shows two presses (the spec presses twice, 9-11 s apart) with identical outcomes.

| Row | Ability | Runner | Server (first press) | Grade | Class |
|---|---|---|---|---|---|
| Soldier-00 | setup | PASS | world 1300 | PASS | |
| Soldier-598 | Quick Burst | PASS | `ability_launched` 19:24:23.272 | PASS | |
| Soldier-717 | Snare Shot | PASS | `ability_launched` 19:25:01.828 | PASS | |
| Soldier-715 | Hollow Point Ammunition | PASS ([FAILED] applied) | `ability_refused no_mechanics` gate 19:25:33.222 | PARTIAL | passive (design) |
| Soldier-716 | Wounding Shot | PASS | `ability_launched` 19:26:13.615 | PASS | |
| Soldier-714 | Stance: Mobility | PASS | `ability_launched` 19:26:44.870 | PASS | |
| Soldier-657 | Interrupting Shot | PASS | `ability_launched` 19:27:40.792 | PASS | |
| Soldier-719 | Armor Piercing Ammunition | PASS | `ability_launched` 19:28:11.351 | PASS | |
| Soldier-891 | Suppression Shot | PASS | `ability_launched` 19:28:45.589 | PASS | |
| Soldier-718 | Selective Fire | FAIL (applied) | `ability_refused no_mechanics` gate 19:29:22.834 | FAIL | seed gap (effect 778 has no NVPs) |
| Soldier-720 | Cover Fire | PASS | `ability_launched` 19:30:02.065 | PASS | |
| Soldier-713 | Draw Fire | PASS | `ability_launched` 19:30:39.373 | PASS | |
| Soldier-723 | Incendiary Ammunition | PASS ([FAILED] applied) | `ability_refused no_mechanics` gate 19:31:28.837 | PARTIAL | passive (design) |
| Soldier-725 | Sustained Fire | FAIL (applied) | `ability_refused no_mechanics` gate 19:32:08.432 | FAIL | seed gap (effects 757-759) |
| Soldier-1474 | Pinning Fire | PASS | `ability_launched` 19:32:47.623 | PASS | |
| Soldier-867 | Quickload | PASS ([FAILED] applied) | `ability_refused no_mechanics` gate 19:33:18.419 | PARTIAL | passive (design) |
| Soldier-722 | Spray and Pray | PASS | `ability_launched` 19:33:56.287 | PASS | |
| Soldier-1879 | Point Blank Shot | PASS | `ability_launched` 19:34:32.715 | PASS | |
| Soldier-1446 | Explosive Ammunition | PASS ([FAILED] applied) | `ability_refused no_mechanics` gate 19:35:25.490 | PARTIAL | passive (design) |
| Soldier-1475 | Flushing Fire | BLOCKED (setup press denied) | no server row at all (no `use_ability_recv`) | BLOCKED | client/lab, cause unknown |
| Soldier-1445 | EMP Ammunition | PASS ([FAILED] applied) | `ability_refused no_mechanics` gate 19:36:11.108 | PARTIAL | passive (design) |
| Soldier-1476 | No Rest for the Weary | PASS | `ability_launched` 19:36:51.103 | PASS | |
| Soldier-1477 | All Out Assault | FAIL (applied) | `ability_refused no_mechanics` gate 19:37:29.075 | FAIL | seed gap (effects 5267-5270, 4357, 3702) |
| _session_end | logout | FAIL (`lab_logout` bridge connect) | n/a | FAIL | lab defect (bridge blip) |

Totals by server truth: PASS 13 (+ Soldier-00), PARTIAL 5, FAIL 3 (+ `_session_end`), BLOCKED 1.

## Findings

### 1. Seed gap: three damage abilities have effects with no damage NVPs (real defect, 3 rows)

Soldier-718 Selective Fire (effect 778), Soldier-725 Sustained Fire (effects 759, 758, 757) and Soldier-1477 All Out Assault (effects 5270, 5269, 5268, 5267, 4357, 3702). All three are `ABILITY_TYPE_DD`, `passive_yn = false`, `event_set_id = 15`, `item_monikers = [3175425141]` (the same moniker as the launching rows 598 and 1476), `effect_ids` non-empty. The press reaches the server (`use_ability_recv`) and is refused at the gate with `no_mechanics`: `ability_effects_have_mechanics` finds no effect that carries a mechanic. `resources.effect_nvps` has no row at all for these effect ids, while the launching abilities' effects do (660 `FocusDamage 200`/`HealthDamage 20`, 1767, 1770, 1772). The `effect_desc` text of the missing ones states the intended numbers (778 "-200F / -20H, 3 ammo"; 759 "-300F / -30H, 5 ticks"; 5270 "F-300 H-30, 5 pulses"). The server did the right thing for the data it was given; the Armory/seed row is the defect. Issue: [DR-L2-soldier-0-damage-nvps-missing.md](issues/DR-L2-soldier-0-damage-nvps-missing.md). Fix route: `seed-change` skill, edit `db/resources/`, never a migration.

### 2. Passive abilities refused by design (5 rows, no defect)

Soldier-715, 723, 867, 1446, 1445 are `passive_yn = true`, `ABILITY_TYPE_Buff`, no moniker, no event set (867 has effect 934, no NVP). The runner passes them because their only required clause is "the press leaves the client", but the spec's optional "client applies the ability" clause is generated for them and shows [FAILED]. The server refuses every press (`no_mechanics`, the passive path; reason string is the same). Not an Armory defect. Spec/generator observation for the architect (no issue): do not emit the `applied` clause for `passive_yn = true` rows, or mark the expected server outcome as a refusal, so the ledger stops carrying a false [FAILED] on a PASS row. Carry the 5 ids to DR-99 as "passive, nothing to launch".

### 3. Soldier-1475 Flushing Fire: press ignored by the client, server saw nothing (lab/client, unclassified)

Server side the ability is sound: `ABILITY_TYPE_DD`, effect 1767 with `FocusDamage 100`/`HealthDamage 10`, moniker 728213066 which the passing rows 713, 716 and 722 share. In the bundle the setup `place` call put the ability on hotbar button 10 (key `0`), then the press on key `0` was `ignored`: `ability_sent 0`, `verdict nothing_observed`, so `use_ability_recv` never happened and there is no SigNoz row. The row before it (1446) is a passive refusal at 19:35:34; nothing else points at the cause (a dialog, cast, stun or UI focus per the tool's hint; the preceding `select-slot1` F1 and both window clicks were `posted_only`). Not enough evidence to call it the Armory, the spec or the tool. Re-run Soldier-1475 alone; if it repeats, it is a spec defect in the select-slot or place step. Issue (lab, unclassified): [DR-L2-soldier-0-flushing-fire-press-ignored.md](issues/DR-L2-soldier-0-flushing-fire-press-ignored.md).

### 4. `_session_end`: `lab_logout` bridge connect failed (lab defect)

`lab_logout failed at step 'character_select' after 56486 ms: bridge: connect to bridge at 127.0.0.1:8770`. Same family as the canary's GW-3128 cold-start blip (`lab_login` focus, bridge connect). All 23 rows ran before it; the next file (soldier-1) started at 19:39:21Z and drove normally. Flag, not a stop. Issue: [DR-L2-soldier-0-logout-bridge-connect.md](issues/DR-L2-soldier-0-logout-bridge-connect.md).

## Not run / UNVERIFIED

- Nothing was spec-blocked or skipped in this file.
- UNVERIFIED: none; the file has no PENDING SigNoz clauses (those are gear files only).
- Soldier-1475: not graded on the server (it never pressed); needs a re-run.
- `Build: unknown` in `run.json` and the ledger: the version is read from SigNoz but not attested (lease held, see below).
- Damage, hit and target results were not examined: every press carried `client_target_id 0` (as in the canary), so launches say nothing about damage.

## Pending attests

The lab lease was held at verification time (owner `lab_uat_run`, purpose "UAT run, sections armory-abilities-soldier-1, ...", renewed 19:39:28Z), so no lease was taken (R5). One attest is pending, for the orchestrator's gap between runs:

- `lab_uat_attest {run_dir: "%LOCALAPPDATA%\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqjljvxcsrgh", server_version: "6b50d9131417b8cf93feb61a7c640577398db567"}` (with the then-current lease). Evidence query (SigNoz logs, window 2026-11-02T19:23:07Z to 19:38:59Z): `service.name = 'cimmeria-server' AND cimmeria.lab_run = 'dr-lab' AND scope_name = 'abilities' AND player_name CONTAINS 'Cgbqjljvxcsrgh'` and read resource `service.version` (209 rows, all one version).
- No clause attests: the file has no PENDING clause.
- Row grades above come from this server query, run over the window: `scope_name = 'abilities' AND player_name CONTAINS 'Cgbqjljvxcsrgh'`, events `use_ability_recv`, `ability_launched`, `ability_refused`.

## Recipe

SigNoz via `signoz_search_logs` with `filter`, `start 1793647380000`, `end 1793648340000`, limit 400 (209 rows returned, none truncated). Seed facts from `resources.abilities`, `resources.effects`, `resources.effect_nvps` through `server_db_query` (read-only). `no_mechanics` logic: `crates/entity/src/abilities/implemented.rs` `ability_effects_have_mechanics` and `crates/cell-combat/src/cell/abilities/use_ability/no_mechanics.rs` `lacks_mechanics`.

## Recommendation

Findings-only. Take the seed gap (finding 1) to a `seed-change` packet and re-run Soldier-718, 725, 1477 after it; re-run Soldier-1475 once; give the architect the passive-row `applied` clause observation.

verify DR-L2-soldier-0 docs/analysis/debug-armory/results/DR-L2-soldier-0.md

## Rerun 2026-11-02 (post DR-07l/m/n)

Full-file rerun after the ground-click fix. Run `%LOCALAPPDATA%\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqoputibbuew`, window 21:54:34Z to 22:10:02Z, fresh character Cgbqoputibbuewdj, 23 rows plus `_session_end`, 4 `lab_uat_run` calls. Runner: PASS 21 (incl. setup and `_session_end`), FAIL 3 (718, 725, 1477). `service.version` 6b50d9131417b8cf93feb61a7c640577398db567 attested for this run under a short lease (none was held), released. Server truth: SigNoz `service.name = 'cimmeria-server' AND cimmeria.lab_run = 'dr-lab' AND scope_name = 'abilities' AND player_name CONTAINS 'Cgbqoputibbuew' AND event IN ('use_ability_recv','use_ability_on_ground_recv','ability_launched','ability_refused')`, grouped by `ability_id, event, reason`, 1793656440000 to 1793657460000. Every pressed row shows exactly 2 presses with identical outcomes.

Changes against the first run: Soldier-1475 now presses (2x `use_ability_on_ground_recv`, 2x `ability_launched`) and is PASS; `_session_end` is PASS (no bridge blip). Everything else matches the first run.

| Row | Ability | Runner | Server (2 presses) | Grade | Class |
|---|---|---|---|---|---|
| Soldier-00 | setup | PASS | world 1300 | PASS | |
| Soldier-598 | Quick Burst | PASS | 2x `ability_launched` | PASS | |
| Soldier-717 | Snare Shot | PASS | 2x `ability_launched` | PASS | |
| Soldier-715 | Hollow Point Ammunition | PASS ([FAILED] applied) | 2x `ability_refused no_mechanics` | PARTIAL | passive (design) |
| Soldier-716 | Wounding Shot | PASS | 2x `ability_launched` | PASS | |
| Soldier-714 | Stance: Mobility | PASS | 2x `ability_launched` | PASS | |
| Soldier-657 | Interrupting Shot | PASS | 2x `ability_launched` | PASS | |
| Soldier-719 | Armor Piercing Ammunition | PASS | 2x `ability_launched` | PASS | |
| Soldier-891 | Suppression Shot | PASS | 2x `ability_launched` | PASS | |
| Soldier-718 | Selective Fire | FAIL (applied) | 2x `ability_refused no_mechanics` (1 effect, 0 NVPs) | FAIL | seed gap |
| Soldier-720 | Cover Fire | PASS | 2x `ability_launched` | PASS | |
| Soldier-713 | Draw Fire | PASS | 2x `ability_launched` | PASS | |
| Soldier-723 | Incendiary Ammunition | PASS ([FAILED] applied) | 2x `ability_refused no_mechanics` | PARTIAL | passive (design) |
| Soldier-725 | Sustained Fire | FAIL (applied) | 2x `ability_refused no_mechanics` (3 effects, 0 NVPs) | FAIL | seed gap |
| Soldier-1474 | Pinning Fire | PASS | 2x `ability_launched` | PASS | |
| Soldier-867 | Quickload | PASS ([FAILED] applied) | 2x `ability_refused no_mechanics` | PARTIAL | passive (design) |
| Soldier-722 | Spray and Pray | PASS | 2x `ability_launched` | PASS | |
| Soldier-1879 | Point Blank Shot | PASS | 2x `ability_launched` | PASS | |
| Soldier-1446 | Explosive Ammunition | PASS ([FAILED] applied) | 2x `ability_refused no_mechanics` | PARTIAL | passive (design) |
| Soldier-1475 | Flushing Fire | PASS (SGHC 6 SMG, applied ok) | 2x `use_ability_on_ground_recv` + 2x `ability_launched` | PASS | was BLOCKED; fixed by DR-07l/m |
| Soldier-1445 | EMP Ammunition | PASS ([FAILED] applied) | 2x `ability_refused no_mechanics` | PARTIAL | passive (design) |
| Soldier-1476 | No Rest for the Weary | PASS | 2x `ability_launched` | PASS | |
| Soldier-1477 | All Out Assault | FAIL (applied) | 2x `ability_refused no_mechanics` (6 effects, 0 NVPs) | FAIL | seed gap |
| _session_end | logout | PASS | n/a | PASS | the first run's bridge blip did not repeat |

Notes. The five PASS rows with a FAILED `applied` clause (715, 723, 867, 1446, 1445) are the passives the server refuses by design (runner PASS, no launch: PARTIAL, as before). 718, 725, 1477 are the known seed gap (`server_db_query`: 0 `effect_nvps` rows for their effects), issue [DR-L2-soldier-0-damage-nvps-missing.md](issues/DR-L2-soldier-0-damage-nvps-missing.md); no new issue body. [DR-L2-soldier-0-flushing-fire-press-ignored.md](issues/DR-L2-soldier-0-flushing-fire-press-ignored.md) is resolved by the rerun (the press now reaches the server) and can be closed. [DR-L2-soldier-0-logout-bridge-connect.md](issues/DR-L2-soldier-0-logout-bridge-connect.md) did not repeat; keep it as a one-off lab flake.

## Merged final grades

The rerun covers all 23 rows and supersedes the first run (the first run's server evidence agrees on every row except 1475 and `_session_end`). Final grades are the Grade column above.

**Merged totals soldier-0: PASS 15 (incl. setup), PARTIAL 5, FAIL 3, BLOCKED 0 (23 rows).** `_session_end` PASS.
