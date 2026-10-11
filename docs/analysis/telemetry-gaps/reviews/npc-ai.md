# NPC AI and spawns telemetry review

The NPC system is well instrumented: every target reaches SigNoz, and transitions, aggro, leash, off-mesh and cover all write structured rows with names.
The problem is the opposite of a gap. About 22 patrolling and wandering NPCs in zones nobody is in write about 70% of all `cimmeria-server` rows, which buries the rows that matter.
The real negative gaps are a death that will never respawn, an NPC attack that does not fire, instance population, and a few Rule 5 key misuses.

Reviewer: npc-ai-spawn-advisor. Data: colo SigNoz, 7 days to 2026-10-10, `cimmeria-server` (15.9M rows).

## 1. Inventory

All of the targets below are exported. `OTEL_FILTER` (`crates/server/src/logging/filters.rs:290`) names `npc_ai=debug` (which covers every `npc_ai.*` child by prefix), `spawner=debug`, `cover=debug`, `movement.npc=debug`, `threat=info` and `loot` at the `info` default. The module-path rows `cimmeria_cell_{combat,world,catalog,cover}=debug` cover the untargeted rows. No NPC row is DEBUG-only-and-invisible.

What fired (rows in 7 days):

| Target / event | Level | Rows | Note |
|---|---|---|---|
| `npc_ai.tick` | DEBUG | 5,449,430 | 4.38M are Patrol/Wander (`patrol_continue` 1.90M, `patrol_dwell` 1.42M, `wander_dwell` 0.89M) |
| `movement.npc` `step` / `waypoint_reached` | DEBUG | 4.14M / 1.01M | |
| `npc_ai.aggro_scan` `no_candidates` | DEBUG | 1,719,535 | 1,709,836 (99.4%) have `witness_count = 0` |
| `npc_ai` `patrol_*` / `wander_*` | DEBUG | 1.25M | |
| `npc_ai.path` `request` ok | DEBUG | 564,799 | |
| `npc_ai.idle` `unticked` | DEBUG | 142,956 | the same per-world count every 30 s |
| `npc_ai.transition` `state_change` | DEBUG | 6,347 | |
| `npc_ai.aggro` `acquired` | INFO | 2,076 | proximity 1,728, damage 320, assist 19, content 9 |
| `npc_ai.leash` `enter` / `arrived` | INFO | 1,301 / 1,297 | |
| `spawner.npc_respawn` `npc_respawn_recreate` | INFO | 747 | |
| `cover.coverage` `space_summary` | INFO / WARN | 286 / 8 | WARN = SGC_W1 `no_usable_cover` |
| WARNs: `npc_off_mesh` 31, `leash loop` 31, `path_fail` 26, `spawn_off_mesh` 21, `stuck` 16 | WARN | | `loop` fired only on the 2026-10-05 build, before DA-F2. It is fixed |

Never fired: `npc_ai.idle_parked`, the `npc_ai.tick` zero-HEALTH WARN, `Failed to spawn NPC from DB`, and the fight attack-not-fired WARN.

**Volume:** the six rows above that start at `npc_ai.tick` and end at `npc_ai.idle` add up to about 14.3M of the 15.9M `cimmeria-server` rows. In one day of ticks, `NID Guard - Castle inside L2` alone wrote 172,776 Patrol rows, exactly one every 2 s per NPC, which means nobody was watching it.

## 2. Positive gaps

- `npc_ai.tick` (`crates/cell-combat/src/cell/service/npc_ai/dispatch.rs:477`) carries no `world` or `space_id`. Both are only on the debug span (`:208`), which is not exported to logs. The highest-volume NPC row can't be grouped by zone.
- `npc_ai.tick` writes an empty `decision_outcome` on the tick where the Idle handler acquires aggro: 1,728 rows read `ai_state=Fighting outcome=""`, the same count as `cause=proximity`. `idle_aggro.rs` never calls `record_decision_outcome`.
- `NPC death: respawn scheduled` (`crates/cell-combat/src/cell/combat/state.rs:63`) has no `event`, no `world` (empty on all 747 rows, although the function takes `world`), and no `space_id`.
- The cover decisions `move_to_cover`, `stay_in_cover` and `cover_no_shot` (`fight_cover.rs:129,144,256`) have no `world`, `space_id` or template pair, so 198 of the 7 days' cover decisions can't be attributed to a zone. The release and `no_cover` rows do carry `world`.
- `npc_ai.path_fail` (`path_failure/mod.rs`) has `world` but no `space_id` or `template_id` / `template_name`, so it can't be joined to the instance.
- `spawn_instance_npcs_from_records` (`crates/cell-world/src/cell/space_manager/npc_population.rs:73-120`) keeps its counts on an info span only. No event says "instance N of world W got K of M NPCs". The startup row (`:63`) logs `count` but not `record_count` or the skips.
- Rule 5 key misuse: `player_id` holds an **entity id** on `npc_ai.aggro_scan` `candidate_rejected` (`detectors/aggro_scan.rs:131`) and on `npc_ai.leash` `player_combat_exit` (`npc_ai/leash/begin.rs:30`). NT-23 fixed this shape elsewhere. A SigNoz `player_id = 72` filter matches these rows with the wrong meaning.
- `npc_ai.leash` `enter` / `loop` (`detectors/leash.rs:77,129`) and `npc_ai` `stuck` name the target but never carry the target player's `account_id` / `player_id`, so "why did the mob reset on account 6" can't be filtered. `damage_ignored` (`:229`) has the same gap for the attacker.
- `spawner.npc_behaviour` (`npc_population.rs:138-175`) uses sentinels and Debug formatting: `world = unwrap_or_default()` writes `""`, `space_id` and `template_id` use `unwrap_or(0)`, and `respawn_secs = ?Option` writes the string `"Some(120)"`, which SigNoz types as a string. That is the trap in the field-naming rule.

## 3. Negative gaps

| Site | Shape | Effect |
|---|---|---|
| `combat/state.rs:60` `mark_npc_dead` | `if let Some(secs)`, with no else branch | A corpse that will never respawn says nothing. 22 of the 769 deaths this week (Castle_CellBlock `ArmYourself_*` content spawns) went silent. On worlds whose `respawn_secs` is NULL, every kill does |
| `npc_ai/fight.rs:440` | WARN with no `event`, `reason = handle_use_ability_returned_false`, no throttle | It doesn't say which guard refused the attack (cooldown, range or reload), and the 500 ms fast-retry (`:461`) re-WARNs about 2 times a second per NPC while the cause persists. No Pattern D |
| `npc_population.rs:30` | `if !has_space_for_world { continue; }` | A spawnlist row with a misspelt `world_name` looks the same as an instanced-world row. Silent |
| `ticks/npc_respawn/mod.rs:277` | `let _ = tx.send(onLootDisplay)` | Pattern A. The stale loot window stays open, with no row |
| `ticks/npc_respawn/mod.rs:329` | `if let Some(pos) = spawn_pos` | A respawn with no spawn position skips the snap. Only the `?spawn_pos = None` Debug string on the INFO row shows it |
| `effects/pulsing/pulse.rs:138-145` | `let Some(tag) else return` / `let Some(credited) else return` | A DoT kill of a tagged mob whose credit resolves to no player is silent. The direct-hit twin (`use_ability/kill_credit.rs:171`) WARNs `kill_credit_no_player` |
| `combat/threat/aggro.rs:247-273` | not `preemptable` | Threat accrues on an NPC in Submit, Despawning or Error, and `enter_player_combat` still runs, with no row naming the state |

## 4. Noise

1. **Unwitnessed Patrol and Wander NPCs.** NA24 samples the tick row of an unwitnessed Idle NPC (`dispatch.rs:417`), but Patrol and Wander NPCs write every tick row, every leg's `movement.npc` head steps plus 1 in 10 (`ticks/npc_movement.rs:261-264`), and every `patrol_arrived` / `patrol_waypoint_set` and `npc_ai.path request`. That is about 11M rows a week from about 22 NPCs in Castle and DebugArea while nobody is in those zones.
2. **`no_candidates` with `witness_count = 0`** (`aggro_scan.rs:143-170`): 1.71M rows that say "nobody here, nobody aggroed". A row only explains something when a witness is present.
3. **`npc_ai.idle` `unticked`** (`detectors/sweep.rs:89`): the same per-world count every 30 s. It could be written only when the count changes, plus an hourly heartbeat.
4. **`spawn_off_mesh` for SGC_W1 spawns 65/66/67** re-WARNs on every instance creation. That is correct, because it is a seed defect, but the fix belongs in the seed, not in telemetry.

## 5. Seams (two hops)

| Hand-off | This side | Other side | Can SigNoz tell who dropped it? |
|---|---|---|---|
| Combat damage → `generate_threat` → Idle→Fighting | `npc_ai.aggro acquired` (INFO, identity, cause) | combat `abilities` damage rows | Yes for a successful entry. **No** for a non-preemptable state (§3) |
| Combat → threat → player `BSF_InCombat` | — | `threat enter_combat` / `exit_combat` (INFO, 41/41) | Yes |
| Death (`resolve_death`) → `mark_npc_dead` → respawn tick | transition DEBUG, `respawn scheduled`, `npc_respawn_recreate` | `abilities target_killed` (INFO, no `world`, no attacker identity) | Respawned: yes. **Never respawns: no** |
| Death → mission credit (`entity_death` → `fire_entity_death`) | — | `fire_entity_death: matched` (INFO) has a tag but no dead NPC id, `account_id` or `world`; `no chains matched` is DEBUG with no dead id | Partly. A DoT credit miss is silent (§3) |
| Death → loot | `loot.drop skipped npc_only_kill` (INFO) | `loot` `open_loot` (INFO) | Yes |
| Spawn → AoI introduction | spawn DEBUG rows (`npc_id`, `space_id`, world, template) | `aoi.entity_enter` (with `world` and template) / `aoi.introduce` (no `space_id`) | Joinable on `entity_id`. Instance-level summary missing (§2) |
| Respawn → AoI re-create | `npc_respawn_recreate` with `witness_count` and `recreated` | `aoi.create_emit` | Yes. `recreated < witness_count` flags an empty introduction |
| AI → navmesh (`find_path`) | `npc_ai.path`, `path_fail` WARN, `npc_off_mesh`, `spawn_off_mesh` | `movement.navmesh` residency | Mostly. `path_fail` lacks `space_id` and the template pair |
| AI → movement → wire | `movement.npc` steps | `wire.out.avatar_update` sampled 1 in 101 | Yes, but item 1 of §4 makes it unreadable |
| AI → cover reservation | `cover.hold`, `cover.selection`, `cover.coverage` | `cover.stance` | Yes, except the decision rows lack `world` |
| Content `spawn_entity` / spawn sets → AI | `Content: despawned tagged entity`, `spawn set switched` | spawn DEBUG rows | Yes |

## 6. Adversarial

1. **"The mob I killed never came back"** (players hit this on worlds with NULL `respawn_secs`, and in the Castle_CellBlock content spawns). **Today:** `target_killed` INFO and `fighting -> dead` DEBUG, then nothing. The only hint is the boot-time `spawner.npc_behaviour respawn_secs="None"`, possibly days earlier. **Should:** `npc_ai.respawn` INFO `event=respawn_not_scheduled reason=no_respawn_secs|content_spawn` at the death, with the NPC, the template, `world` and `space_id`.
2. **"The guard stood there while I shot it."** **Today:** if `handle_use_ability` refused, a WARN with an opaque reason repeats every 500 ms without a throttle. If the guard is off-mesh (SGC_W1 65-67), `spawn_off_mesh`, `npc_off_mesh`, `path_fail` and `stuck` WARNs fire. That explains the cause, but `path_fail` can't be tied to the instance, and none of the rows can be filtered by the player's account. **Should:** `event=attack_not_fired` with the refusal reason and Pattern D, the template pair and `space_id` on `path_fail`, and the target's `account_id` / `player_id` on `stuck` and `leash`.
3. **An instance comes up with no NPCs** (misspelt world, every template failing, or #838-style questions). **Today:** no per-NPC DEBUG rows (absence is not a query), and no summary. **Should:** INFO `event=instance_populated {world, space_id, record_count, spawned, failed}`.
4. **"No cover was ever used."** **Today:** `cover.coverage WARN no_usable_cover` names SGC_W1 (13 cover NPCs, 0 nodes), which explains it. In Castle and DebugArea, the `move_to_cover` / `stay_in_cover` rows can't be counted per world. **Should:** `world` and `space_id` on every cover decision row.
5. **An operator investigates any NPC fault on the colo.** **Today:** the relevant WARN sits under 11M unwitnessed patrol rows a week, and SigNoz queries on `npc_ai*` time out or sample. **Should:** unwitnessed Patrol and Wander rows sampled the way NA24 samples Idle.

## Candidate packets

| ID | Title | Severity | Files |
|---|---|---|---|
| TG-NPC-01 | Sample unwitnessed Patrol/Wander `npc_ai.tick` rows | high | 1 |
| TG-NPC-02 | Gate `movement.npc` step rows on witnesses | high | 1 |
| TG-NPC-03 | Drop `no_candidates` rows with zero witnesses | high | 1 |
| TG-NPC-04 | Death that will not respawn says so | med | 1 |
| TG-NPC-05 | NPC attack-not-fired: event, throttle, identity | med | 1 |
| TG-NPC-06 | Instance population summary and unknown-world skips | med | 1 |
| TG-NPC-07 | `player_id` holding an entity id (Rule 5) | med | 2 |
| TG-NPC-08 | Target player identity on leash, stuck and damage_ignored rows | med | 3 |
| TG-NPC-09 | Unwitnessed patrol/wander leg and path-request rows | med | 3 |
| TG-NPC-10 | `world`/`space_id`/template on cover decisions and `path_fail` | low | 2 |
| TG-NPC-11 | `world`/`space_id` on `npc_ai.tick`; aggro outcome slot | low | 3 |
| TG-NPC-12 | Respawn tick negative gaps | low | 1 |
| TG-NPC-13 | DoT kill-credit miss WARN | low | 1 |
| TG-NPC-14 | Threat on a non-preemptable NPC logs its state | low | 1 |
| TG-NPC-15 | `spawner.npc_behaviour` sentinels and Debug options | low | 1 |

**TG-NPC-01 — Sample unwitnessed Patrol/Wander tick rows (high).**
`crates/cell-combat/src/cell/service/npc_ai/dispatch.rs` `admit_ai_tick_row`: extend the NA24 sample to an NPC whose `state_before` equals its current state, is `Patrol` or `Wander`, and has no witnesses. One row per `IDLE_UNWITNESSED_TICK_SAMPLE` carrying `suppressed`. Any state change, or any witness, still writes every row. The `npc_ai.tick` DEBUG level is unchanged. Test: a service test in `crates/cell/src/cell/service/tests/npc_ai/tick_row.rs`, `unwitnessed_patrollers_sample_their_tick_row`, modelled on `idle_unwitnessed_npcs_sample_their_tick_row`. Five ticks give one row, and the next admitted row has `suppressed = 4`. A witnessed patroller writes five rows. Reverting the change gives five rows, so the test fails. No pin change.

**TG-NPC-02 — Gate `movement.npc` steps on witnesses (high).**
`crates/cell/src/cell/service/ticks/npc_movement.rs`, the step block at about `:257-284`: for an NPC with no witnesses, skip the leg-head rows and keep only a per-NPC 60 s sample, using `npc_detectors.admit_sample` with `suppressed`. `waypoint_reached` and `stop` rows are unchanged. Test: a unit test in the module, `unwitnessed_npc_steps_are_sampled`. Ten steps of an unwitnessed NPC write at most one `event=step` row, and a witnessed NPC writes its head steps. Reverting gives one row per head step, so the test fails.

**TG-NPC-03 — Drop `no_candidates` with zero witnesses (high).**
`crates/cell-world/src/cell/service/npc_ai/detectors/aggro_scan.rs` `report_scan`: return before the `no_candidates` sample when `witness_count == 0`, and keep the row for `witness_count > 0`. Test: a `LogCapture` unit test in the detector tests. `witness_count = 0` writes no row, and `witness_count = 1` with no candidate writes the DEBUG `event=no_candidates`. Reverting writes a row in the first case, so the test fails.

**TG-NPC-04 — A death that will not respawn says so (med).**
`crates/cell-combat/src/cell/combat/state.rs` `mark_npc_dead`. Add `event = "respawn_scheduled"`, `world` and `space_id` to the existing INFO row. Add an else branch: INFO, `target: "npc_ai.respawn"`, `event = "respawn_not_scheduled"`, `reason = "content_spawn"` when `spawn_id` is `None`, else `"no_respawn_secs"`, with `npc_id`, `npc_name`, `template_id`, `template_name`, `world` and `space_id`. The `npc_ai` prefix already exports it. Test: `LogCapture` unit tests in the same file's `tests` module, `death_without_respawn_secs_logs_reason` and `respawn_scheduled_row_carries_world`. Reverting removes the row, so the tests fail. Update the `npc_ai.*` row in `observability-target-catalog.md`.

**TG-NPC-05 — NPC attack-not-fired (med).**
`crates/cell-combat/src/cell/service/npc_ai/fight.rs` at about `:433-450`. Add `target: "npc_ai"`, `event = "attack_not_fired"`, the template pair, `world` and `space_id`, and a Pattern D throttle via `space_mgr.npc_detectors.admit_warn(npc_id, "attack_not_fired", now, 15 s)` with `suppressed`. The refusal reason stays `handle_use_ability_returned_false`. Mark it `needs-domain-agent` only if the returned bool is to become a reason enum. Test: a `LogCapture` service test with an NPC whose only ability is on cooldown and stubbed so the pick ignores it. Three fast retries inside the window give one WARN and then `suppressed = 2`, and a second NPC's first refusal is not swallowed (the independence guard). Reverting writes three rows, so the test fails.

**TG-NPC-06 — Instance population summary (med).**
`crates/cell-world/src/cell/space_manager/npc_population.rs`. In `spawn_instance_npcs_from_records`, add INFO `target: "spawner"`, `event = "instance_populated"`, with `world`, `space_id`, `record_count` (matching), `spawned` and `failed`. In `spawn_npcs_from_records`, count the rows whose world is neither a startup space nor known to `world_id_for_world`, and write one WARN per such `world_name`: `event = "spawn_world_unknown"`, `records`. Test: a `LogCapture` unit test in the file's tests, which asserts the summary fields and that a record for `"NoSuchWorld"` WARNs once. Reverting removes both rows, so the test fails.

**TG-NPC-07 — `player_id` holding an entity id (med).**
`detectors/aggro_scan.rs:131` (`candidate_rejected`, and the assist rows near it) and `npc_ai/leash/begin.rs:30`. Rename the key to `witness_id` + `witness_name` (aggro scan) or `entity_id` + `entity_name` (leash), and add `account_id`, `account_name`, `player_id` and `player_name` from `SpaceManager::player_identity`, passed as `Option`s. Test: a `LogCapture` test for each row asserting that `player_id` equals the sgw `player_id`, not the entity id. The current code fails it. Add the rename to the key-change table in `negative-logging-convention.md`.

**TG-NPC-08 — Target player identity on leash, stuck and damage_ignored (med).**
`detectors/leash.rs` (`on_enter`, the `loop` row, `on_damage_while_leashing`) and `detectors/sweep.rs` (`stuck`). Use `target_account_id` / `target_player_id` and their names via `player_identity(target_id)` (the subject prefix Rule 5 prescribes). `damage_ignored` takes `attacker_account_id` / `attacker_player_id`. Test: `LogCapture` in `detector_tests/leash.rs` with a player target, asserting the fields are present for the player and absent for an NPC target. Reverting fails the presence assertion.

**TG-NPC-09 — Unwitnessed leg rows (med).**
`npc_ai/patrol.rs` (`patrol_arrived`, `patrol_waypoint_set`), `npc_ai/wander.rs` (`wander_arrived`, `wander_waypoint_set`) and `npc_ai/path_request.rs` (the `request` row when `status = ok`). When the NPC has no witnesses, sample through `npc_detectors.admit_sample` (60 s) with `suppressed`. Failures are never sampled. Test: a service test in `tests/npc_ai/`. An unwitnessed patroller over three legs writes one `patrol_arrived` row, and a `path_fail` is still written. Reverting gives three rows, so the test fails. Depends on TG-NPC-01 landing first, for review context only.

**TG-NPC-10 — Cover decisions and `path_fail` attributable (low).**
`npc_ai/fight_cover.rs:129,144,256` and `npc_ai/path_failure/mod.rs`. Add `world` (via `super::world_label`), `space_id`, and the `template_id` + `template_name` pair. Test: extend the existing cover and path-fail `LogCapture` tests to assert `world`. Reverting fails them.

**TG-NPC-11 — Zone on tick rows; aggro outcome (low).**
`dispatch.rs` `log_ai_tick`: add `world` and `space_id`. `idle_aggro.rs`: `record_decision_outcome("aggro_acquired")` on engage. Add the outcome to the enum in `observability-target-catalog.md`. Test: `tick_row.rs` asserts `world` on the row, and that the engaging tick's `decision_outcome = "aggro_acquired"`. Reverting gives an empty outcome, so the test fails.

**TG-NPC-12 — Respawn tick negative gaps (low).**
`crates/cell/src/cell/service/ticks/npc_respawn/mod.rs`. Replace `let _ =` at `:277` with a WARN `reason = "loot_close_send_failed"`, and add a WARN `event = "respawn_no_spawn_position"` when `spawn_pos` is `None`. Use `target: "spawner.npc_respawn"` for both, with the NPC and template pair. Test: a `LogCapture` test in `npc_respawn/tests/` with a dropped receiver and an NPC with no spawn position. Reverting gives no row, so the test fails.

**TG-NPC-13 — DoT kill-credit miss (low).**
`crates/cell-combat/src/cell/effects/pulsing/pulse.rs:141-145`. Mirror `kill_credit.rs:171`: WARN, `target: "abilities"`, `event = "kill_credit_no_player"`, `reason = "no_credited_player"`, `source = "dot"`, with the tag and the invoker identity. Test: a `LogCapture` pulse test with an invoker pet whose owner is gone. Reverting gives no row. Coordinate with the combat review.

**TG-NPC-14 — Threat on a non-preemptable NPC (low).**
`crates/cell-combat/src/cell/combat/threat/aggro.rs` `generate_threat`. When `preemptable` is false and the state is not Fighting, write DEBUG `target: "npc_ai.aggro"`, `event = "threat_not_preempted"`, `ai_state`, attacker identity, and `cause`. Test: `LogCapture` in the file's tests with a Submit NPC hit by a player. Reverting gives no row.

**TG-NPC-15 — `spawner.npc_behaviour` field hygiene (low).**
`npc_population.rs:138-175`. Pass `world`, `space_id` and `template_id` as `Option`s (no `""` or `0`), and pass `respawn_secs` and `aggression_override` as numbers, not `?`. Test: a `LogCapture` test on a fixture NPC asserting that `respawn_secs` is numeric and that `world` is absent for an unregistered space. Reverting fails it.
