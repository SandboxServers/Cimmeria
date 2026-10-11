# Combat telemetry review

Combat is the most heavily instrumented system on the server: every launch gate, hit, effect plan and wire send has a row, and most rows carry Rule 5 and Rule 6 fields. The gaps are at the edges. The respawn refusals never reach SigNoz, and a guard bug hides that. Nothing flags a player stuck in combat or an effect that does nothing. The bandolier, death and revive rows break Rule 5 and the numeric-field rule.
The 7-day colo data has no combat ERROR at all. The WARNs are mostly normal states: a startup WARN that fires on every boot, and no-witness rows for corpses nobody is watching.

Reviewer: combat-systems-advisor, 2026-10-10. Data: colo `distributed_logs_v2`, last 7 days unless stated.

## 1. Inventory

Targets the combat crates emit: `abilities` (115 sites), `abilities.wire` (16), `abilities.sequence`, `abilities.effect`, `abilities.pulse`, `abilities.qr`, `abilities.ledger`, `threat`, `ammo`, `bandolier`, `vitals`, `player.death`, `loot.drop`, `duel`, plus `player.respawn` (cell-methods, cell-interactions) and the module paths under `cimmeria_cell_combat` / `cimmeria_cell_effect_scripts`.

`OTEL_FILTER` (`crates/server/src/logging/filters.rs:290`):

| Target | Filter | Consequence |
|---|---|---|
| `abilities*`, `ammo`, `vitals`, `duel`, module paths | `debug` | Everything reaches SigNoz |
| `threat` | `info` | Any future DEBUG `threat` row is dropped. Today all `threat` rows are INFO |
| `bandolier`, `player.death`, `loot.drop` | not named (default `info`) | Fine today: every emitter is INFO |
| `player.respawn` | not named (default `info`) | **Both refusal rows are DEBUG and never leave the host** (`cell-methods/.../player/combat/mod.rs:325`, `:353`) |

What fired in the last 7 days: `abilities.wire` DEBUG 74.5k, `abilities` DEBUG 28.4k / INFO 10.4k / WARN 15, `abilities.sequence` DEBUG 9.7k / WARN 34, `abilities.qr` 9.3k, `abilities.effect` 9.0k, `vitals` 7.4k (mostly TRACE), `loot.drop` 746, `threat` 82, `bandolier` 81, `player.death` 7, `player.respawn` INFO 6. **Zero combat ERROR rows.** Never fired: `effect_script_unknown`, `effect_def_missing`, `unknown_ability_fallback_damage`, `kill_credit_no_player`, `use_ability_caster_missing`, and every NPC-side refusal (all 548 `ability_refused` rows were players: on_cooldown 348, no_ammo 89, reload_in_flight 73).

Spans: `combat.use_ability` (`use_ability/handle.rs:53`), `combat.death` (`death/mod.rs:53`), `respawn` (`cell-interactions/.../respawn/mod.rs:~95`), `threat.*` at TRACE by design (#408).

## 2. Positive gaps

- **Rule 5 missing** on `bandolier` `active_slot_change` (`bandolier/active_slot.rs:412`), `weapon_ability_swap` (`bandolier/weapon_abilities.rs:117`, `cell/.../player_init/mod.rs:43`); `abilities` `target_killed` (`death/mod.rs:451`); the `callForAid` and `player revived` rows (`player/combat/mod.rs:47`, `:75`).
- **Numbers typed as strings** (field-naming rule): `item_id = ?item_id` logs `"Some(55)"` / `"None"` (confirmed in ClickHouse: 0 numeric `item_id` on 81 `bandolier` rows), `resolved_ranged_ability = ?` logs `"Some(559)"`; `player.death` `world = ?world_name` logs `Some("Castle_CellBlock")`.
- **Wrong keys:** `target_killed` uses `attacker` / `target` rather than `attacker_id` / `target_id`. `player.death` uses `killer`, its `killer_name` is `unwrap_or_default()` (`""`, a Rule 6 violation, `death/side_effects.rs:512`), and `ability_id = -1` stands in for none.
- **No world** on `target_killed`, `player revived`, `wire_npc_no_witnesses`.
- **`enter_combat` overstates.** `threat.rs` `enter_combat` (`threat/player_combat.rs:68`) says "BSF_InCombat set; weapon drawn", but the proximity-aggro path deliberately sends nothing to the client (`npc_ai/idle_aggro.rs:150-171`, the "ghost combat HUD" carve-out). There were 41 `enter_combat` rows and only 10 `onStateFieldUpdate reason=entered_combat` sends. A reader cannot tell which transitions the client saw.

## 3. Negative gaps

- **Respawn refusals are invisible on the colo**: `respawn_not_dead` and `respawner_not_offered` (`player/combat/mod.rs:313-372`) are DEBUG on an unexported target. A refused `callForAid` sends the client nothing by design, so the player sits in the Defeat Window and SigNoz shows only `aid_wait_sent`.
- **The guard that should have caught it is blind.** `strip_test_module` (`server/src/logging/target_scan_tests.rs:154`) truncates a file at the first `#[cfg(test)] mod`. That includes a `mod tests;` *declaration*, which `player/combat/mod.rs:22` has at the top. I found 25 in-process files with log sites below such a declaration. `player.respawn@debug` is the only one I found that escapes today, but the guard is off for all of them.
- **Effects that do nothing log at DEBUG only.** `PlannedEffect::landing` returns `path=skipped reason=no_script` (`effect_plan.rs:214`). Live: `Staff Melee AA` (710), effect 736 `Strike Damage`, did nothing on all 56 Praxis Jaffa Guard swings. It is a seed fact with no WARN.
- **Stale combat state has no detector.** Nothing checks that `threatened_mobs` names living NPCs, or that `BSF_InCombat` agrees with the set. The vitals tick (`cell/.../ticks/vitals.rs:20`) skips a player whose bit is set with an empty set, and samples a stale set forever as `threat_count=1`. Audits S7 and S12, and the submit leak, were all this shape.
- `let _ = tx.send(ON_END_AID_WAIT)` (`cell-interactions/.../respawn/mod.rs:155`): if it fails, the Defeat Window stays open after a revive, and nothing is logged.
- `update_bandolier_ammo` rows_affected 0 (`base-methods/.../inventory/ammo.rs:46`) is DEBUG with no `rows_affected` / `expected` / `account_id` (Pattern B). Handed to the items review.
- `death/side_effects.rs:38-42`: a missing Entity_Death sequence (event set 1025) returns silently, so no death animation plays and nothing says why. Low: it is a seed fact.

## 4. Noise

- `abilities` `ability_launched` INFO (`use_ability/handle.rs:539`): 8,985 rows, 8,736 of them NPC casts (97%), almost all Debug Area NPC-vs-NPC. Rule 2 puts per-cast transitions at DEBUG.
- Per NPC-only death, three INFO rows: `target_killed` (776), `NPC death: respawn scheduled` (747), `loot.drop npc_only_kill` (746, INFO by decision #1009). The first two add nothing at INFO when no player is involved.
- `effect_script_unregistered` WARN (`cell/src/cell/service/startup.rs:350`) fires on every boot (13 boots, 13 rows) for a state the shipped seed pins as expected (`cell-effect-scripts/.../registry.rs:197`: `""` effect 2907, `Reload` effect 658, which the reload pipeline handles). A WARN that is always present trains readers to ignore the one that matters.
- `abilities.wire` `timer_update_not_sent reason=not_player` DEBUG: 8,711 rows restating a fixed rule (the client binds method 12 on SGWPlayer only).
- `ability_refused` (metrics row) duplicates every gate row (348 + 348). Deliberate (one row per metric sample); left alone.
- 65 `wire_npc_no_witnesses` WARNs come in triples (onStatUpdate, onStateFieldUpdate, InteractionType): an NPC death burst. `player_present` was true only because a player is on the dead NPC's threat list, out of AoI. That is benign, and it is indistinguishable from a real AoI miss (see §5).

## 5. Seams (two hops)

| Hand-off | This side | Other side | Can SigNoz say who dropped it? |
|---|---|---|---|
| NPC AI fight → `handle_use_ability` (`npc_ai/fight.rs`) | gate rows carry `caster_kind=npc` | `npc_ai.decision_outcome` | Yes |
| Hit → `generate_threat` → `enter_player_combat` → `onStateFieldUpdate` | `enter_combat` INFO, `wire_sent reason=entered_combat` | client | **No** for proximity aggro: the row claims a set and nothing is sent (packet 08) |
| NPC death → `clear_dead_npc_from_all_player_threat` → player's state | `exit_combat`, `death_in_combat_cleared` | — | Yes for the clear; **no** when the clear never happens (packet 02) |
| Pet engage `generate_threat` (`pet/engage.rs:167`) | `target_refused_threat` Err, logged by `pets.ai` | — | Yes |
| Combat wire → AoI witnesses → base `send_to_witness` | `wire_sent`, `wire_no_witnesses`, `wire_npc_no_witnesses` | base Pattern C rows | Partly: the NPC no-witness WARN does not say *why* a player counted as present (AoI = real bug, threat list = benign) (packet 07) |
| Ammo spend → `BandolierAmmoUpdate` → base UPDATE | send-failure WARN only; no success row | rows_affected 0 at DEBUG, unpaired | **No**: an unsent flush and a no-op UPDATE look alike (items review) |
| Weapon swap → per-weapon grants → `onKnownAbilitiesUpdate` | `weapon_ability_swap` INFO | `abilities.wire wire_sent` | Yes, but the rows lack `player_id` (packet 05) |
| Content `GrantAbility` → cell mirror | `content ability grant: learned` (base) | `ContentAbilitiesGranted: cell mirrored` (116 = 116) | Yes |
| Kill → `entity_death` content trigger (`kill_credit.rs:188`) | `kill_credit_no_player` WARN | content executor rows | Yes. A tagless NPC is silent by design |
| Death → Defeat Window → `callForAid` → respawn fork → reanchor / GateTravel | `aid_wait_sent`, `player revived`, `respawn` span | base reanchor rows | **No** for refusals (packet 01) or a lost `onEndAidWait` (packet 10) |
| Effect script registry → dispatch | startup WARN, `effect_script_unknown` | — | Yes, but buried in a WARN that fires on every boot (packet 04) |

## 6. Adversarial

1. **A player can't respawn: "Release does nothing"** (players have reported Defeat Window trouble). Today SigNoz shows `player death`, then `Sent onBeginAidWait`, then nothing. It should show `player.respawn reason=respawner_not_offered` (or `respawn_not_dead`) with the respawner and world. The fix is packet 01.
2. **Stuck in combat: no regen, weapon never holsters.** An NPC leaves `threatened_mobs` without an exit, through a new surrender-like path, a despawn without the sweep, or a GM `.kill` that skips the death tail. Today `enter_combat` appears and never pairs with an `exit_combat`, and `vitals combat_sample threat_count=1` repeats every 2 s with no mob named. It should show one WARN `threat event=combat_state_stale reason=mob_gone`, naming the mob and how long the player has been stuck, with `suppressed=N` on the repeats. The fix is packet 02.
3. **"The guard hits me and nothing happens" / "my ability does nothing"** (live for ability 710). Today SigNoz has the `ability_launched` INFO, a `qr_rolled` hit and `effect_planned path=skipped reason=no_script` at DEBUG. A reader has to know to look for that last row. It should show one WARN per (ability, effect) per 60 s: `effect_inert`, naming the ability and effect. The fix is packet 03.
4. **A new effect script is misspelled in the seed.** Today it shows as one more name inside the boot WARN that is always present. After packet 04 the boot is quiet, so any `effect_script_unregistered` row is new.
5. **A swap leaves the right-click attack unbound.** The `bandolier` rows exist, but `player_id = 72` doesn't find them and `item_id = 55` doesn't match (`"Some(55)"`). After packet 05 both filters work.

## Candidate packets

| ID | Title | Severity | Files | Domain agent |
|---|---|---|---|---|
| TG-CMB-01 | Fix the target-scan blind spot; export `player.respawn` DEBUG | high | 2 | no |
| TG-CMB-02 | Stale combat-state detector on the vitals tick | high | 3 | no |
| TG-CMB-03 | `effect_inert` WARN for effects that do nothing | med | 1-2 | no |
| TG-CMB-04 | Quiet the boot `effect_script_unregistered` WARN | med | 3 | no |
| TG-CMB-05 | `bandolier` rows: Rule 5 and numeric ids | med | 3 | no |
| TG-CMB-06 | Death and revive rows: Rule 5/6 keys and world | med | 3 | no |
| TG-CMB-07 | No-witness WARNs say why a player counted as present | med | 3 | no |
| TG-CMB-08 | Proximity aggro: say the combat flag was not announced | low | 1 | no |
| TG-CMB-09 | Demote NPC-only launch and kill INFO rows | low | 3 | no |
| TG-CMB-10 | Log a failed `onEndAidWait` send | low | 1 | no |

**TG-CMB-01: Fix the target-scan blind spot; export `player.respawn` DEBUG** (high)
- Files: `crates/server/src/logging/target_scan_tests.rs` (`strip_test_module`), `crates/server/src/logging/filters.rs` (`OTEL_FILTER`).
- Change: strip only an *inline* test module (`#[cfg(test)]` followed by `mod name {`). A `mod name;` declaration must not end the scan. Add `player.respawn=debug` to `OTEL_FILTER` next to `player.journal`. If the widened scan flags other targets, add their rows in the same packet. My sweep found only this one.
- Test: unit test `strip_test_module_keeps_code_after_a_test_mod_declaration`, on source with `#[cfg(test)]\nmod tests;` followed by `tracing::debug!(target: "x", ...)`, asserting `scan` finds `x`. The existing `every_source_target_reaches_signoz_at_its_level` then fails if the filter row is reverted.

**TG-CMB-02: Stale combat-state detector** (high)
- Files: `crates/cell/src/cell/service/ticks/vitals.rs` (`vitals_sample_tick`), `crates/cell-combat/src/cell/combat/vitals.rs` (new `log_combat_state_stale`), and the `SpaceManager` struct file plus `destroy_entity` in `crates/cell-world` (a `LogThrottle` field, released on destroy).
- Row: WARN, target `threat`, `event = "combat_state_stale"`. `reason` is one of `mob_gone` (id not in the space), `mob_dead` (`BSF_Dead`), `bit_without_threat` (`BSF_InCombat` set, set empty), `threat_without_bit`. Fields: `account_id`, `player_id`, `entity_id` with their names, `mob_id` + `mob_name` (if any), `threat_count`, `state_field` + `state_field_names`, `world`, `suppressed`. Pattern D: `LogThrottle`, keyed by entity, 60 s window, counter on every occurrence. Check every living player, not only those with a non-empty set.
- Test: LogCapture, one per reason. Plus the burst guard (3 ticks give 1 row; the next row carries `suppressed=2`) and the independence guard (a second player's first row is not swallowed). Each fails if the check or the throttle is removed.

**TG-CMB-03: `effect_inert` WARN** (med)
- Files: `crates/cell-combat/src/cell/abilities/effect_plan.rs` (`PlannedEffect::log`); add a throttle module beside it if the file passes 500 lines.
- Row: when `path == skipped && reason == no_script`, also write WARN `abilities.effect`, `event = "effect_inert"`, with `ability_id` + `ability_name`, `effect_id` + `effect_name`, `entity_id` + `entity_name` (caster), `target_id` + `target_name`, `caster_kind`, `suppressed`. Throttle keyed by `(ability_id, effect_id)`, 60 s. This is a seed fact like `abilities.sequence no_event_set`, and its state is bounded by the effect table, so a static `Mutex<HashMap>` is fine (no entity release).
- Test: LogCapture. One plan with a no-script, no-NVP effect gives one WARN; three give one row, then `suppressed=2`; a different effect id is not swallowed.

**TG-CMB-04: Quiet the boot `effect_script_unregistered` WARN** (med)
- Files: `crates/cell-world/src/cell/effects/registry.rs` (`unregistered`: skip an empty `script_name` and the names in a new `NATIVE_EFFECT_NAMES = ["Reload"]`), `crates/cell/src/cell/service/startup.rs:346` (log the native or blank ones once at INFO, `event = "effect_script_native"`, and keep the WARN for the rest), `crates/cell-effect-scripts/src/cell/effects/registry.rs` (`KNOWN_UNSCRIPTED` becomes empty, with the doc comment updated).
- Test: unit test in cell-world `registry.rs`: defs naming `""`, `Reload` and `Bogus` yield only `Bogus`. The live-DB guard asserts an empty list on the shipped seed. Reverting the filter fails both.

**TG-CMB-05: `bandolier` rows, Rule 5 and numeric ids** (med)
- Files: `crates/cell-combat/src/cell/cell_methods/inventory/bandolier/active_slot.rs:412`, `.../bandolier/weapon_abilities.rs:117`, `crates/cell/src/cell/service/base_messages/player_init/mod.rs:43`.
- Change: add `account_id`, `account_name`, `player_id`, `player_name` from `space_mgr.player_identity`. Pass `item_id` as the `Option<i32>` value, not `?`. Rename `resolved_ranged_ability` to `resolved_ability_id` (an `Option<i32>` value) and add `resolved_ability_name`. Add the rename to the key-change table in `negative-logging-convention.md`.
- Test: LogCapture on `weapon_ability_swap` and `active_slot_change`, asserting `player_id == "72"` and `item_id == "55"`. Fails on `"Some(55)"`.

**TG-CMB-06: Death and revive rows, Rule 5/6** (med)
- Files: `crates/cell-combat/src/cell/abilities/death/mod.rs:451` (`target_killed`), `.../death/side_effects.rs:513` (`player.death`), `crates/cell-methods/src/cell/cell_methods/player/combat/mod.rs:47,75` (`callForAid`, `player revived`).
- Change: rename `attacker` to `attacker_id` and `target` to `target_id`, and `killer` to `killer_id`. Add the attacker's and target's `account_id` / `player_id` with names when they are players. Make `killer_name` an `Option<&str>` (no `""`). Log `world` as an `Option<&str>` value, not `?`. Make `ability_id` an `Option<i32>`. On `callForAid` and `player revived`, add the identity quartet and `world`. Add the renames to the convention's key-change table.
- Test: LogCapture for the `player.death` row: `world == "Castle"` (not `Some("Castle")`), no `killer_name` field for an environment kill. For `target_killed`: a player attacker's `player_id` is present.

**TG-CMB-07: No-witness WARNs say why a player counted as present** (med)
- Files: `crates/cell-world/src/cell/space_manager/player_presence.rs` (new `player_present_via(entity_id, counterpart) -> Option<&'static str>`, returning `player_side`, `aoi` or `threat_list`; `player_present` becomes `.is_some()`), `crates/cell-combat/src/cell/abilities/messaging.rs:245`, `.../use_ability/sequence.rs` (the `no_witnesses` outcome).
- Change: add `present_via` and `world` to both WARNs. `present_via = threat_list` logs at DEBUG under the same event. A player watching nothing is not a fault. That covers the 65-row death-burst triples.
- Test: LogCapture. An NPC with a threat-listed player outside AoI gives DEBUG `present_via=threat_list`; a player in AoI with an empty witness list gives WARN `present_via=aoi`.

**TG-CMB-08: Proximity aggro, combat flag not announced** (low)
- File: `crates/cell-combat/src/cell/service/npc_ai/idle_aggro.rs:164`.
- Row: when `generate_threat` returned `Some` on the proximity path, write INFO `threat`, `event = "enter_combat_unannounced"`, `reason = "proximity_carveout"`, with the identity quartet, `mob_id` + `mob_name`. It is INFO because `threat=info` drops DEBUG. About 23 a week.
- Test: LogCapture through `idle_aggro` with a hostile player in range gives one row. Removing the row fails it.

**TG-CMB-09: Demote NPC-only launch and kill INFO rows** (low)
- Files: `crates/cell-combat/src/cell/abilities/use_ability/handle.rs:539`, `.../abilities/death/mod.rs:451`, `crates/cell-combat/src/cell/combat/state.rs` (`NPC death: respawn scheduled`).
- Change: INFO when a player or a player's pet is the caster, attacker or target; DEBUG otherwise. Same event, two callsites. Optionally move `timer_update_not_sent reason=not_player` (`abilities/timer_update.rs`) to TRACE. That removes about 1.6k INFO rows a day on the colo.
- Test: LogCapture. An NPC-on-NPC launch logs at DEBUG; a player launch at INFO.

**TG-CMB-10: Log a failed `onEndAidWait` send** (low)
- File: `crates/cell-interactions/src/cell/respawn/mod.rs:155`.
- Change: replace the `let _` with WARN `wire_send_failed`, `method = "onEndAidWait"`, `reason = "cell_to_base_closed"`, plus the identity quartet.
- Test: LogCapture with a closed channel, reason pinned.
