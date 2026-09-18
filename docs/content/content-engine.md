---
title: "Content engine — reference"
type: reference
audience: engineers
last_updated: 2026-07-25
---

# Content engine — reference

> **Last updated**: 2026-07-25
> **Audience**: Engineers working on the Rust server who need to understand, debug, or extend the data-driven content engine.
> **Prerequisites**: Familiar with the Cimmeria crate layout ([crates/README.md](../../crates/README.md)) and the cell/base service split ([architecture/service-architecture.md](../architecture/service-architecture.md)).
> **Diátaxis type**: Reference + explanation. For the design rationale ("why does this exist at all?") see [architecture/data-driven-content-engine.md](../architecture/data-driven-content-engine.md). For "how do I add a new variant?" see [extending-the-engine.md](extending-the-engine.md).

The content engine is the runtime that turns database rows into running game logic. Triggers fire on gameplay events, conditions gate progression, actions mutate state. It's how every mission, dialog route, region transition, and consumable in Cimmeria executes — without a line of per-mission code.

This document covers the engine end-to-end: architecture, vocabulary, execution model, schema, persistence, observability, and performance.

---

## 1. What it is

A trigger / condition / action chain runtime, split across two crates:

- **[crates/content-engine/](../../crates/content-engine/)** — pure data crate. Defines `Chain`, `Trigger`, `Condition`, `Action`, `ExecutionContext`, and `ChainEngine`. No game state, no DB, no networking. Unit-testable in isolation.
- **[crates/services/src/cell/content/](../../crates/services/src/cell/content/)** — the bridge. Loads chains from PostgreSQL at boot, fires events from real `CellEntity` state, and dispatches resolved actions back to the cell service as `CellToBaseMsg` traffic.

The boundary exists so the engine stays declarative. Chain authors write SQL rows; engineers write executor handlers. Neither has to think about the other's layer.

```
┌────────────────────────────────────────────────────────────────────┐
│                          PostgreSQL                                │
│   resources.content_chains / _triggers / _conditions / _actions    │
└──────────────────────────┬─────────────────────────────────────────┘
                           │ build_engine() at boot
                           ▼
┌────────────────────────────────────────────────────────────────────┐
│  cimmeria-content-engine (pure data crate)                         │
│    Chain { trigger, conditions, actions }                          │
│    ChainEngine { chains_by_trigger: HashMap<TriggerType, Vec> }    │
└──────────────────────────┬─────────────────────────────────────────┘
                           │ resolve_event(event, ctx) → ResolvedActions
                           ▼
┌────────────────────────────────────────────────────────────────────┐
│  cell/content/ bridge (effectful)                                  │
│    fire_<event>() builds ExecutionContext from CellEntity state    │
│    executor::execute_actions() dispatches actions →                │
│      • space_manager mutations (in-process)                        │
│      • CellToBaseMsg outbox → BaseApp persists & forwards client   │
└────────────────────────────────────────────────────────────────────┘
```

Implementation status (2026-07-25): **shipped and driving Castle_CellBlock and SGC_W1 end-to-end.** Since the original write-up the surface has grown to cover NPC AI direction (`SetNpcPoi` / `SetFollowTarget` / `SetNpcAiState`), cover-proximity triggers, and cross-world teleport. Note that a handful of authorable actions still have no executor arm — read §3's catalog before authoring a chain.

> **Unmerged work described here.** This document was revised on the
> `feat/571-black-market-phase1` branch. The `OpenBlackMarket` action, the
> `open_black_market` seed verb, the `executor/black_market.rs` handler, and the
> `chain_replay_tests/black_market.rs` guard are **part of that unmerged branch,
> not `main`**. Everything else described here is on `main`.

---

## 2. Architecture — the two-crate split

### Pure engine ([crates/content-engine/src/](../../crates/content-engine/src/))

| File | Owns |
|---|---|
| [lib.rs](../../crates/content-engine/src/lib.rs) | Module rollup, re-exports |
| [chain.rs](../../crates/content-engine/src/chain.rs) | `Chain`, `ChainEngine`, `ResolvedActions`, `resolve_event` |
| [triggers/](../../crates/content-engine/src/triggers/) | `Trigger`, `TriggerType`, `TriggerEvent` ([mod.rs](../../crates/content-engine/src/triggers/mod.rs)); `Trigger::matches` ([matching.rs](../../crates/content-engine/src/triggers/matching.rs)) |
| [conditions.rs](../../crates/content-engine/src/conditions.rs) | `Condition`, `Condition::evaluate` |
| [actions.rs](../../crates/content-engine/src/actions.rs) | `Action`, `ActionResult`, `PropertyOp`, `NpcAiStateAction` |
| [context.rs](../../crates/content-engine/src/context.rs) | `ExecutionContext` (param key/value bag) |
| [loader/](../../crates/content-engine/src/loader/) | DB-row → typed enum conversion — split into [trigger.rs](../../crates/content-engine/src/loader/trigger.rs) / [condition.rs](../../crates/content-engine/src/loader/condition.rs) / [action.rs](../../crates/content-engine/src/loader/action.rs), assembled by [mod.rs](../../crates/content-engine/src/loader/mod.rs) |

This crate does not depend on `cimmeria-services`, `cimmeria-base`, or `tokio` runtime types. Its full dep set is `cimmeria-common`, `cimmeria-entity`, `serde`, `serde_json`, `thiserror`, `tracing` ([Cargo.toml:9-15](../../crates/content-engine/Cargo.toml#L9-L15)).

### Bridge ([crates/services/src/cell/content/](../../crates/services/src/cell/content/))

| File | Owns |
|---|---|
| [mod.rs](../../crates/services/src/cell/content/mod.rs) | Public re-exports for the rest of the cell service |
| [engine_loader.rs](../../crates/services/src/cell/content/engine_loader.rs) | `build_engine` — runs the four boot SQL queries |
| [event_dispatch/](../../crates/services/src/cell/content/event_dispatch/) | `fire_<event>` factory functions, grouped by family: `cover.rs`, `dialog.rs`, `interaction.rs`, `inventory.rs`, `lifecycle.rs`, `mission.rs`, `region.rs` |
| [executor/](../../crates/services/src/cell/content/executor/) | `execute_actions` — the `match action { ... }` in [mod.rs](../../crates/services/src/cell/content/executor/mod.rs), forwarding to per-family handlers (`mission.rs`, `inventory.rs`, `dialog.rs`, `stats.rs`, `world/`, `counter.rs`, `transport.rs`, `black_market.rs`) |
| [mission_context.rs](../../crates/services/src/cell/content/mission_context.rs) | Populators: write mission/counter/stat state into `ExecutionContext` |
| [chain_replay_tests/](../../crates/services/src/cell/content/chain_replay_tests/) | Live-DB regression guards that pin chain behavior, one module per mission/feature (`mission_622.rs`, `mission_638.rs`, …, `black_market.rs`, `cover_demo.rs`) |

The bridge owns every effect (channel sends, `space_manager` mutations, log lines). The engine never produces a side effect — it only resolves which actions the bridge should run.

---

## 3. The vocabulary

### Triggers — *what fires the chain*

Defined at [triggers/mod.rs:28-146](../../crates/content-engine/src/triggers/mod.rs#L28-L146). Filterable by an optional second key (entity type, item id, region key, etc.) per variant. The DB `event_type` string each variant is authored as lives in [loader/trigger.rs](../../crates/content-engine/src/loader/trigger.rs) — only variants with a match arm there are reachable from seed data.

| Variant | Fires when |
|---|---|
| `OnEntityCreated { entity_type? }` | Entity spawns (filterable by template-string type) |
| `OnEntityDestroyed { entity_type? }` | Entity removed |
| `OnEntityDeath { entity_type?, entity_tag? }` | Entity dies; tag wins over type when both set |
| `OnAbilityUsed { ability_id? }` | Any entity uses an ability |
| `OnInteraction { interaction_type? }` | Generic right-click |
| `OnRegionEnter { region_key }` | Player enters a Kismet region (string key like `Castle_CellBlock.Region2`) |
| `OnRegionExit { region_key }` | Player exits region |
| `OnMissionStep { mission_id, step }` | Mission advances to a specific step |
| `OnItemAcquired { item_id? }` | Item enters inventory |
| `OnTimer { timer_name }` | Named timer expires (defined; see §10) |
| `OnCustomEvent { event_name }` | Generic invoke escape hatch / synthetic for triggerless chains |
| `OnPlayerLoaded { world_name? }` | Player completes mapLoaded |
| `OnDialogOpen { dialog_id }` | Server sent `onDialogDisplay` |
| `OnDialogChoice { dialog_id }` | Player clicked a dialog button. **Server-gated**: the `DialogButtonChoice` handler rejects the event unless `CellEntity::open_dialog_id == dialog_id` (the dialog was actually displayed to this player via `send_dialog_display`); a forged/replayed choice for an un-opened `dialog_id` is dropped with a `warn!` and never fires the chain (CAT-J-01 / #479). |
| `OnInteractTag { entity_tag }` | Right-click on tagged NPC/object |
| `OnInteractTemplate { template_name }` | Right-click on entity from named template |
| `OnItemUse { item_id }` | Player double-clicked inventory item |
| `OnItemEquipped { item_id? }` | Player moved a stack into the bandolier (`container_id = 3`) from any other container. `item_id` is the design / `type_id`, not the inventory instance id; `NULL` `event_key` is a wildcard that fires for any equip |
| `OnTeleportIn { region_id }` | Player arrived via ring transporter |
| `OnEffectInit / PulseBegin / PulseEnd / Removed` | Effect lifecycle hooks (unit variants) |
| `OnMissionCompleted { mission_id }` | Mission marked complete |
| `OnDialogSetOpen { dialog_set_name }` | Dialog set opened |
| `OnMissionAccepted { mission_id }` | Mission just accepted or advanced (fired from the executor's combined `Action::AcceptMission \| Action::AdvanceMission` branch after the cell-side state commit; used by chains that highlight quest objects on mission start — e.g. chain 1097 for Aftermath) |
| `OnPlayerEnteredCover { cover_set_id? }` | Player entered proximity of a cover set (`resources.cover_sets`). One event per set; a player can be in several at once. Wildcard (`NULL`) fires for any set |
| `OnPlayerLeftCover { cover_set_id? }` | Player left a cover set's proximity — the symmetric partner of `OnPlayerEnteredCover` |
| `OnPlayerInCoverDuration { cover_set_id?, seconds }` | Player has been continuously in a cover set for ≥ `seconds`. Debounced: leaving and re-entering resets the timer. Seed `event_key` convention is `"<seconds>"` or `"<seconds>:<set_id>"` ([loader/trigger.rs:87-100](../../crates/content-engine/src/loader/trigger.rs#L87-L100)) |
| `OnNpcFlanked { npc_template? }` | An NPC occupying a cover slot was flanked — its top-threat target moved outside the cover's defensive arc (orientation ± π/2) |

Within a single chain's bucket, `Trigger::matches` ([triggers/matching.rs:43](../../crates/content-engine/src/triggers/matching.rs#L43)) decides whether the event matches the chain's specific trigger variant + filter. Bucketing is by **`TriggerType` discriminant** — see §6.

**Not reachable from seed data.** `OnEntityCreated`, `OnEntityDestroyed`, `OnAbilityUsed`, `OnInteraction`, `OnMissionStep`, `OnItemAcquired`, and `OnTimer` have no match arm in [loader/trigger.rs](../../crates/content-engine/src/loader/trigger.rs), so no `content_triggers` row can bind them — a chain authored with those `event_type` strings is dropped with a `warn!`. `OnCustomEvent` has no arm either but is generated internally, as the synthetic `__direct_invoke_<id>` trigger for chains with zero trigger rows (§6). `OnEntityDeath` is reachable only through the `entity_dead_tag` (tag-filtered) form; there is no `event_type` that binds the `entity_type` form.

### Conditions — *gates that AND together*

Defined at [conditions.rs:12-95](../../crates/content-engine/src/conditions.rs#L12-L95). All conditions on a chain are AND'd ([chain.rs:161](../../crates/content-engine/src/chain.rs#L161)). For OR, author multiple chains.

| Variant | Predicate |
|---|---|
| `PropertyEquals { property, value }` | `ctx.params[property] == value` |
| `PropertyInRange { property, min, max }` | numeric in `[min, max]` |
| `HasItem { item_id, min_count? }` | reads `item_<id>_count` from ctx — **populator missing today; see §10** |
| `HasAbility { ability_id }` | reads `ability_<id>` bool |
| `InRegion { region_id }` | reads `current_region` |
| `FactionCheck { faction, relation }` | reads `faction_<name>` — **populator missing today** |
| `MissionStatus { mission_id, op, expected }` | `not_active` / `active` / `completed`; missing key defaults to `not_active` ([conditions.rs:194](../../crates/content-engine/src/conditions.rs#L194)) |
| `StepStatus { mission_id, step_id, op, expected }` | three-state per step |
| `ObjectiveStatus { mission_id, objective_id, op, expected }` | string compare on objective state |
| `Archetype { op, archetype_id }` | reads `archetype` i64 |
| `Counter { counter_name, op, value }` | reads `counter_<name>` |
| `StatBelowMax { stat_id }` | `stat_<id>_cur < stat_<id>_max`. **Fail-closed** on missing params ([conditions.rs:255-268](../../crates/content-engine/src/conditions.rs#L255-L268)) |
| `CustomExpression { expression }` | bool-key lookup, escape hatch |

**Only six are authorable.** [loader/condition.rs](../../crates/content-engine/src/loader/condition.rs) has match arms for exactly `mission_status`, `step_status`, `archetype`, `objective_status`, `counter`, and `stat_below_max`. The other seven variants (`PropertyEquals`, `PropertyInRange`, `HasItem`, `HasAbility`, `InRegion`, `FactionCheck`, `CustomExpression`) cannot be named by a `content_conditions` row at all — a seed row using them is dropped with a `warn!`. `HasItem` and `FactionCheck` are doubly dead: even reached from Rust, no populator writes the `item_<id>_count` / `faction_<name>` keys they read (§9).

### Actions — *side effects*

Defined at [actions.rs:20-323](../../crates/content-engine/src/actions.rs#L20-L323). **`Action::execute` is a stub** ([actions.rs:363-376](../../crates/content-engine/src/actions.rs#L363-L376)); only `TriggerChain` self-executes. Everything else is dispatched by [executor/mod.rs](../../crates/services/src/cell/content/executor/mod.rs).

An action has to clear **two** hurdles to do anything. It needs a match arm in [loader/action.rs](../../crates/content-engine/src/loader/action.rs) (otherwise no `content_actions` row can name it) *and* a match arm in [executor/mod.rs](../../crates/services/src/cell/content/executor/mod.rs) (otherwise it resolves and then falls through to a `debug!` no-op at [mod.rs:453-455](../../crates/services/src/cell/content/executor/mod.rs#L453-L455)). The table below is the authoritative catalog; the "Seed rows" column counts `content_actions` rows across [db/resources/Content/Seed/](../../db/resources/Content/Seed/) as of 2026-07-25.

#### Authorable and executed

| Seed verb | `Action` variant | Seed rows |
|---|---|---|
| `accept_mission` | `AcceptMission` | 49 |
| `complete_mission` | `CompleteMission` | 17 |
| `abandon_mission` | `AbandonMission` | 1 |
| `advance_step` | `AdvanceStep` | 23 |
| `complete_objective` | `CompleteObjective` | 2 |
| `display_dialog` | `DisplayDialog` | 33 |
| `add_dialog` | `AddDialog` | 10 |
| `add_dialog_set` | `AddDialogSet` | 6 |
| `remove_dialog_set` | `RemoveDialogSet` | 2 |
| `add_item` | `GrantItem` | 14 |
| `remove_item` | `RemoveItem` | 2 |
| `grant_xp` | `GrantXP` | 0 |
| `change_stat` | `ChangeStat` | 3 |
| `increment_counter` | `IncrementCounter` | 9 |
| `reset_counter` | `ResetCounter` | 3 |
| `play_sequence` | `PlaySequence` | 15 |
| `set_interaction_type` | `SetInteractionType` | 70 |
| `set_visible` | `SetVisible` | 1 |
| `destroy_entity` | `DestroyTaggedEntity` | 1 |
| `generate_threat` | `GenerateThreat` | 3 |
| `set_aggression` | `SetAggression` | 1 |
| `set_npc_poi` | `SetNpcPoi` | 0 |
| `set_follow_target` | `SetFollowTarget` | 0 |
| `set_npc_ai_state` | `SetNpcAiState` | 0 |
| `move_waypoint` | `MoveWaypoint` | 0 |
| `move_entity` | `MoveEntity` | 5 |
| `set_active_slot` | `SetActiveSlot` | 0 |
| `start_minigame` | `StartMinigame` | 4 |
| `trigger_transporter` | `TriggerTransporter` | 2 |
| `cross_world_teleport` | `CrossWorldTeleport` | 1 |

> An `open_black_market` / `OpenBlackMarket` action exists on the unmerged
> `feat/571-black-market-phase1` branch (PR #586) and is **not** on `main`. It is
> documented in [../architecture/black-market.md](../architecture/black-market.md);
> do not author against it until that branch lands.

#### Authorable but NOT executed — seeded rows that silently no-op

These have a loader arm, so the seed accepts them and the engine resolves them, but **[executor/mod.rs](../../crates/services/src/cell/content/executor/mod.rs) has no match arm** — every one falls through to the `debug!` catch-all and does nothing. This is a live correctness gap, not a roadmap item: 8 seeded rows are currently dead.

| Seed verb | `Action` variant | Seed rows | Consequence |
|---|---|---|---|
| `launch_ability` | `LaunchAbility` | 3 | Scripted ability fires never happen |
| `qr_combat_damage` | `QrCombatDamage` | 2 | Scripted damage is never applied |
| `apply_effect` | `ApplyEffect` | 1 | No chain can apply a buff/debuff |
| `remove_effect` | `RemoveEffect` | 1 | No chain can strip an effect |
| `fail_objective` | `FailObjective` | 1 | Objective-fail branches never fire |

`system_message` (`SystemMessage`, **11 seeded rows**) is a third state: it has an executor arm, but the arm only emits an `info!` log. The client wire format is still unknown — see §10.

#### Not authorable — defined in the enum, no loader arm

No `content_actions` row can name these; they are reachable only from Rust (or not at all). `SpawnEntity`, `DespawnEntity`, `PlayAnimation`, `PlaySound`, `ModifyProperty`, `RollLootTable`, `SpawnLootBag`, `StartTimer`, `CancelTimer`, `ExecuteCustom`. None has an executor arm either, so wiring any of them is a two-sided job. `GrantXP` used to head this list; it was wired on both sides in issue #611 and now appears in the executed table above with **0 seed rows** — the plumbing exists, no content uses it yet, and the seed still has `reward_xp = 0` on all 1,040 mission rows (§9).

Four variants have an executor arm but no seed verb, reached only as internal aliases or from Rust: `AdvanceMission` (aliased onto the `AcceptMission` arm), `StartDialog` (aliased onto `DisplayDialog`), `Teleport` (same-space teleport; only `cross_world_teleport` is authorable), and `TriggerChain` (resolved by the engine, re-dispatched by the caller). `SendMessage` has an arm that only logs.

See [proposed-extensions.md](proposed-extensions.md) for the wiring plan.

---

## 4. The execution model

End-to-end trace, using `OnItemUse(2893)` (Health Slappack) as the worked example.

1. **Gameplay observes the event.** Player double-clicks the Slappack. `crate::cell::content::fire_item_use(...)` is called from [base_messages/mod.rs](../../crates/services/src/cell/service/base_messages/mod.rs).
2. **The bridge builds an `ExecutionContext`.** [event_dispatch/inventory.rs:28](../../crates/services/src/cell/content/event_dispatch/inventory.rs#L28):
   - sets `item_id`, `instance_id`
   - calls `populate_mission_context` — writes every `mission_<id>_status`, `mission_<id>_step_<step>_status`, and `counter_<name>` from the source `CellEntity`
   - calls `populate_stats_context` — writes `stat_<id>_cur` / `stat_<id>_max` for every stat on the entity
3. **The bridge constructs a `TriggerEvent`.** `TriggerEvent { trigger_type: TriggerType::ItemUse, params: ctx.params.clone(), … }`.
4. **Engine resolves.** `engine.resolve_event(&event, &ctx)` ([chain.rs:247](../../crates/content-engine/src/chain.rs#L247)):
   - looks up the bucket for `TriggerType::ItemUse` (already priority-sorted at register time, [chain.rs:86](../../crates/content-engine/src/chain.rs#L86))
   - for each chain in the bucket:
     - `chain.trigger.matches(event)` — filters on `item_id`
     - `chain.conditions.iter().all(|c| c.evaluate(ctx))` — all-AND
     - on full match, pushes `(chain.id, action.clone())` for each of the chain's actions onto `ResolvedActions.actions`
   - `params` is cloned forward only when ≥1 chain matched ([chain.rs:277-279](../../crates/content-engine/src/chain.rs#L277-L279) — defer-clone optimization landed in commit `a51a10d`)
5. **Bridge executes.** `executor::execute_actions(resolved, …)` ([executor/mod.rs:59](../../crates/services/src/cell/content/executor/mod.rs#L59)). For chain 4001, the action sequence is `ChangeStat { stat_id: 7, amount: Some(500) }` then `RemoveItem { item_id: 2893, count: 1 }`.
   - `ChangeStat` ([executor/stats.rs:14](../../crates/services/src/cell/content/executor/stats.rs#L14)) mutates `entity.stats.get_mut(7).change(500)`, drains dirty stats, sends `CellToBaseMsg::EntityMethodCall { method_index: ON_STAT_UPDATE, args: payload }`.
   - `RemoveItem` ([executor/inventory.rs:149](../../crates/services/src/cell/content/executor/inventory.rs#L149)) reads the forwarded `instance_id` param; routes to `RemoveInventoryItem` (by-instance) when present, falls back to `RemoveInventoryItemByType` otherwise. The instance plumbing is what fixes the bandolier-stack-mismatch bug from PR #214.
6. **BaseApp persists and forwards.** Drains `CellToBaseMsg`s, runs the corresponding `UPSERT`s and Mercury writes.

The contract between engine and bridge is `ResolvedActions` ([chain.rs:235-238](../../crates/content-engine/src/chain.rs#L235-L238)):

```rust
pub struct ResolvedActions {
    pub actions: Vec<(i64, Action)>,
    pub params: HashMap<String, serde_json::Value>,
}
```

The forwarded `params` map is load-bearing — it carries trigger-time state (most importantly `instance_id`) into the executor so that `RemoveItem` consumes the exact stack the player clicked rather than first-by-type.

---

## 5. Schema

All content tables live in the `resources` schema. Read-only at runtime.

| Table | Rows represent | Key columns | Read by |
|---|---|---|---|
| `resources.content_chains` | Chain header | `chain_id`, `description`, `scope_type` ∈ `{mission,space,effect,global}`, `scope_id`, `enabled`, `priority` | [engine_loader.rs:50-65](../../crates/services/src/cell/content/engine_loader.rs#L50-L65) |
| `resources.content_triggers` | Event binding | `chain_id` (FK), `event_type`, `event_key`, `scope`, `once`, `sort_order` | [engine_loader.rs:67-82](../../crates/services/src/cell/content/engine_loader.rs#L67-L82) |
| `resources.content_conditions` | Gate predicate | `chain_id`, `condition_type`, `target_id`, `target_key`, `operator`, `value`, `sort_order` | [engine_loader.rs:84-100](../../crates/services/src/cell/content/engine_loader.rs#L84-L100) |
| `resources.content_actions` | Side effect | `chain_id`, `action_type`, `target_id`, `target_key`, `params jsonb`, `delay_ms`, `sort_order` | [engine_loader.rs:102-118](../../crates/services/src/cell/content/engine_loader.rs#L102-L118) |
| `resources.content_counters` | Editor metadata only | `counter_id`, `chain_id`, `counter_name`, `target_value`, `reset_on` | **Not read by engine.** Used only by the admin-api editor CRUD. |

Schema source: [db/resources/Content/Tables/](../../db/resources/Content/Tables/).

The `_type` discriminator columns (`event_type`, `condition_type`, `action_type`) are `varchar(50)` with no CHECK constraint. This is intentional: new variants ship without DDL — the loader gains a match arm, and a typo'd type silently drops the row with a `warn!`.

The `params jsonb` column is the catch-all for new action fields. Every new field rides in JSON. The trade-off: zero migration cost, but no schema-level type safety. A typo'd key ("ammount") silently no-ops.

### What's stored but NOT in these tables

- **Per-player mission state** — `sgw_mission` (player_id, mission_id, status, current_step_id, completed_step_ids[], …). Loaded by the world-entry path in [base/world_entry/methods/missions.rs](../../crates/services/src/base/world_entry/methods/missions.rs), not by `engine_loader`. The engine reads it via `CellEntity.missions` after the populator runs.
- **Counter state** — in-memory only on `CellEntity.counters: HashMap<String, i32>` ([cell_entity/mod.rs:290](../../crates/entity/src/cell_entity/mod.rs#L290)). **Not persisted; lost on logout.** Counter design assumes the completion threshold is reachable in one session. See §8.
- **Inventory, stats, abilities, effects** — all live on `CellEntity` and persist via the existing per-domain save paths. The engine consumes them via populators.

---

## 6. Boot-time loading

`build_engine(db_pool: Option<&PgPool>) -> ChainEngine` at [engine_loader.rs:19-44](../../crates/services/src/cell/content/engine_loader.rs#L19-L44):

1. If `db_pool` is `None`, log warn and return empty `ChainEngine`. Server runs without content.
2. Otherwise call `load_chains_from_db(pool)`.
3. **Four sequential SELECT queries** fire, one per table, each `ORDER BY chain_id [, sort_order]`. They are **not joined in SQL** — assembly happens in Rust.
4. All rows materialize eagerly into `Vec<DbChainRow>` etc. No streaming, no lazy hydration.
5. `build_chains_from_rows` ([loader/mod.rs:92](../../crates/content-engine/src/loader/mod.rs#L92)) groups triggers/conditions/actions by `chain_id`, sorts each group by `sort_order`, converts each row to a typed enum, and **expands multi-trigger chains** by emitting one in-memory `Chain` per trigger row.
6. A chain with **zero trigger rows** receives a synthetic `OnCustomEvent("__direct_invoke_<id>")` so it stays callable via `on_victory_chains` arrays and `Action::TriggerChain` ([loader/mod.rs:166](../../crates/content-engine/src/loader/mod.rs#L166)).
7. Each `Chain` is registered into `ChainEngine` via `register_chain`, which inserts into `chains_by_trigger: HashMap<TriggerType, Vec<Chain>>` ([chain.rs:58](../../crates/content-engine/src/chain.rs#L58)) and **sorts the bucket descending by priority** ([chain.rs:86](../../crates/content-engine/src/chain.rs#L86)).

**Indexing** is by `TriggerType` discriminant. Within a bucket, evaluation is linear (filter + condition AND). At current seed sizes (low thousands of chains, dozens per trigger type) this is a non-issue. See §11 for scale concerns.

### Hot reload

There is no hot-reload path today. Schema changes or new chains require a server restart. A `POST /content/reload-chains` admin-api endpoint is on the proposed-extensions list ([proposed-extensions.md](proposed-extensions.md)).

---

## 7. Persistence model

The engine is **read-only at runtime**. It never writes to `content_*` tables. Migration scripts in [db/scripts/](../../db/scripts/) and seed files in [db/resources/Content/Seed/](../../db/resources/Content/Seed/) are the only writers. The admin-api editor at [admin-api/src/routes/editor.rs](../../crates/admin-api/src/routes/editor.rs) writes via HTTP, but that is an out-of-band authoring path.

When a chain action mutates **player** state, persistence is **not** the engine's responsibility — actions delegate to the existing per-domain save paths:

| Action | Persistence path |
|---|---|
| `AcceptMission`, `CompleteMission`, `AdvanceStep`, `CompleteObjective`, `AbandonMission` | Routes through `crate::cell::missions::*` → emits `CellToBaseMsg::MissionUpdate` → BaseApp `UPSERT sgw_mission` at [missions.rs:103-127](../../crates/services/src/base/world_entry/methods/missions.rs#L103-L127) |
| `FailObjective` | **Nothing — no executor arm.** The `fail_objective` seed verb loads but the action no-ops. See §3 |
| `GrantItem`, `RemoveItem` | `CellToBaseMsg::GrantItem` / `RemoveInventoryItem` / `RemoveInventoryItemByType` → BaseApp inventory write |
| `ChangeStat` | Mutates `CellEntity.stats`; persistence rides existing player save |
| `IncrementCounter`, `ResetCounter` | **Not persisted.** In-memory `CellEntity.counters` only |

The chain itself never touches a persistence table. Trace example: chain 1087 fires on `entity_dead_tag` `MessHall_Guard1`, condition `mission_status 681 eq active` passes → action `complete_mission 681` runs → `complete_mission_direct` mutates `MissionInstance` on the cell entity → emits `CellToBaseMsg::MissionUpdate { mission_id: 681, status: 2, repeats: bumped, ... }` over the outbox → BaseApp dequeues, runs the `UPSERT` ([missions.rs:103](../../crates/services/src/base/world_entry/methods/missions.rs#L103)).

---

## 8. Counter state

PR #237 (commit `76f6759`) added counters as a per-entity in-memory primitive. They unlock kill-N quotas and OR-of-events completion shapes.

### Storage

```rust
// crates/entity/src/cell_entity/mod.rs:287-290
// Not persisted: counters are mission-scoped and intended to be transient.
// The chain that reaches the threshold also explicitly resets the counter
// via Action::ResetCounter.
pub counters: HashMap<String, i32>,
```

### Lifecycle

- **Mutation.** `Action::IncrementCounter` and `Action::ResetCounter` mutate `entity.counters` directly at [executor/counter.rs:19](../../crates/services/src/cell/content/executor/counter.rs#L19) and [:46](../../crates/services/src/cell/content/executor/counter.rs#L46). Increment uses `saturating_add` ([counter.rs:29](../../crates/services/src/cell/content/executor/counter.rs#L29)). Reset *removes* the entry ([counter.rs:54](../../crates/services/src/cell/content/executor/counter.rs#L54)) — not zeros it.
- **Read into ctx.** `populate_counters_context` ([mission_context.rs:37-41](../../crates/services/src/cell/content/mission_context.rs#L37-L41)) writes `counter_<name>` into `ExecutionContext.params` for every entry. Called from `populate_mission_context`, so every mission-aware dispatcher gets counters automatically.
- **Read in chains.** `Condition::Counter` reads `counter_<name>` ([conditions.rs:246-254](../../crates/content-engine/src/conditions.rs#L246-L254)). Missing key → 0. The zero-elision invariant is: **genuinely-zero counters MUST populate explicitly** so `Counter == 0` distinguishes them from "never-incremented." Pinned by `populate_counters_context_writes_counter_keys` at [mission_context.rs:228-251](../../crates/services/src/cell/content/mission_context.rs#L228-L251).

### The hidden ordering invariant

`a51a10d` fixed a bug where increment chains (1085, 1086, 1092, 1093) and completion chains (1087, 1094) lived at the same priority on the same `OnEntityDeath` trigger. Equal-priority ordering inside a `chains_by_trigger` bucket is undefined — so a completion chain's `ResetCounter` could fire before the increment chain on the same kill, leaving the next mission with a stale non-zero counter.

The fix bumped increment chains to priority 1; completion chains stay at 0; the bucket sort is descending. **Conditions evaluate before any sibling action in the same trigger pass executes** — so a "kill N" completion chain whose condition reads `counter` sees the **pre-increment** value. Hence the documented `counter >= target - 1` pattern at [executor/counter.rs:9-18](../../crates/services/src/cell/content/executor/counter.rs#L9-L18): the chain fires on the kill that brings the counter to N, not after.

### Limitations

- **Cross-session persistence:** none. Logout mid-Mess-Hall = counter resets to 0. Quotas that span sessions or zones cannot use this primitive today. Persisted counters are tracked in [proposed-extensions.md](proposed-extensions.md).
- **Counter scoping:** flat string keys. `messhall_kills` from mission 681 and a hypothetical `messhall_kills` from mission 999 collide. Mitigated by naming convention only.
- **Counter arithmetic:** `Condition::Counter` is a single-threshold check. No "kills_today + kills_yesterday >= 100".
- **Counter resets on abandon/fail:** explicit. The `content_counters` table has a `reset_on` column hinting at intent (`mission_complete`, `zone_change`, `never`), but no code reads it.

---

## 9. Mission lifecycle through the engine

Mission state lives in `MissionInstance` ([crates/entity/src/missions.rs:43-58](../../crates/entity/src/missions.rs#L43-L58)) and is mutated by a small set of executor-side action handlers. The engine does not model lifecycle as a state machine — it emerges from chain authoring.

| Stage | Trigger | Condition (typical) | Action (typical) | Persistence |
|---|---|---|---|---|
| **Accept** | `OnPlayerLoaded`, `OnRegionEnter`, `OnDialogChoice`, `OnInteractTag` | `MissionStatus eq not_active` | `AcceptMission` | `MissionUpdate` outbox → `sgw_mission` UPSERT |
| **Step advance** | `OnInteractTag`, `OnDialogChoice`, `OnRegionEnter` | `StepStatus eq active` | `AdvanceStep` | `MissionUpdate` |
| **Objective progress** | `OnEntityDeath`, `OnDialogChoice` | `ObjectiveStatus eq active` | `CompleteObjective` (or `IncrementCounter` for N-of) | `MissionUpdate` |
| **Complete** | `OnDialogOpen`, `OnEntityDeath`, `OnDialogChoice` | `MissionStatus eq active` | `CompleteMission` (+ `GrantItem` reward, often `AcceptMission` for the next step in the chain) | `MissionUpdate` (status=2, repeats++) |
| **Relog-restore (state)** | — | — | — | `sgw_mission` → `MissionManager` at world-entry; engine plays no part |
| **Relog-restore (world)** | `OnPlayerLoaded` | `StepStatus eq active` for the active step | `SetInteractionType` (re-paint quest-glow / Ring icons) | none (in-memory only — interaction flags don't persist on the entity) |

Worked example chains in [chain_replay_tests/](../../crates/services/src/cell/content/chain_replay_tests/):

- **Mission 622 "Arm Yourself"** — chains 1001 (region-accept), 1003 (dialog-complete + reward).
- **Mission 638 "Prisoner 329"** — chains 1011/1012 (archetype-routed dialog) — pinned by `assert_region_enter_resolves_dialog_set` ([chain_replay_tests/mission_638.rs](../../crates/services/src/cell/content/chain_replay_tests/mission_638.rs)).
- **Mission 681 "Mess Hall"** — chains 1085/1086 (increment counter on each guard), 1087 (complete on threshold).
- **Health Slappack consumable** — chain 4001 (`OnItemUse(2893)` + `StatBelowMax 7` → `ChangeStat amount=500` + `RemoveItem 2893`).

### What `mission_context.rs` exposes

| Key | Source | Used by |
|---|---|---|
| `mission_<id>_status` | active/completed/not_active | `Condition::MissionStatus` |
| `mission_<id>_step_<step>_status` | per-step state | `Condition::StepStatus` |
| `counter_<name>` | every entity counter | `Condition::Counter` |
| `stat_<id>_cur` / `stat_<id>_max` | populated only by `fire_item_use` (via `populate_stats_context`) | `Condition::StatBelowMax` |
| `archetype` | set directly by every `fire_*` site | `Condition::Archetype` |

**Not exposed today** (gap list — see [proposed-extensions.md](proposed-extensions.md)):

- `mission_<id>_repeats` — `MissionInstance.repeats` is incremented but no condition reads it. Blocks repeatable-mission gating and "first-time bonus" patterns.
- `item_<id>_count` — `Condition::HasItem` reads this key but no populator writes it. **`HasItem` is dead code today.**
- `faction_<name>` — `Condition::FactionCheck` reads but no populator writes. **Dead.**
- Per-player persistent flags — required for branching narrative ("you killed Frost; the brother NPC remembers").

---

## 10. Failure modes and observability

| Failure | Surfaces as (and where) |
|---|---|
| Unknown DB `event_type`/`condition_type`/`action_type` | `warn!` at [loader/mod.rs:131](../../crates/content-engine/src/loader/mod.rs#L131) and [:142](../../crates/content-engine/src/loader/mod.rs#L142); row dropped silently from the chain (chain still loads, just missing that piece) |
| All trigger rows fail to convert | `warn!` + chain skipped ([loader/mod.rs:186](../../crates/content-engine/src/loader/mod.rs#L186)) |
| `change_stat.amount` out of i32 range | `warn!` + entire action dropped ([loader/action.rs:285-299](../../crates/content-engine/src/loader/action.rs#L285-L299)) |
| Missing condition param (e.g. `mission_*_status` not populated) | Silent fall-through to evaluator default (`unwrap_or("not_active")`); chain may match or miss unexpectedly |
| Missing populator for `StatBelowMax` | **Fail-closed**: returns `false` ([conditions.rs:260-267](../../crates/content-engine/src/conditions.rs#L260-L267)). Deliberate. |
| Trigger filter mismatch | `trace!` at [chain.rs:144-150](../../crates/content-engine/src/chain.rs#L144-L150) — only visible at trace level |
| Condition fails | `trace!` at [chain.rs:163-169](../../crates/content-engine/src/chain.rs#L163-L169) |
| Action `Error` result | `warn!` at [chain.rs:201-209](../../crates/content-engine/src/chain.rs#L201-L209) |
| `RemoveItem` channel send fails | `error!` at [executor/inventory.rs:226](../../crates/services/src/cell/content/executor/inventory.rs#L226) — explicitly loud because mission progress depends on the consume |
| `ChangeStat` source entity missing | `warn!` at [executor/stats.rs:37](../../crates/services/src/cell/content/executor/stats.rs#L37) |
| Empty engine on startup | `warn!` ("No DB pool available") or `error!` ("Failed to load") at [engine_loader.rs:33-41](../../crates/services/src/cell/content/engine_loader.rs#L33-L41) — server runs without content |

The fire-time logs (`info!` on match, `debug!` on no-match) at every `fire_*` site in [event_dispatch/](../../crates/services/src/cell/content/event_dispatch/) are the production observability story. Every action execution emits an `info!` with `chain_id`, the action params, and entity. Tracing-grep for `Content:` to scope to executor activity.

### Defined-but-unhandled actions

Fifteen `Action` variants have **no match arm in [executor/mod.rs](../../crates/services/src/cell/content/executor/mod.rs)** and fall through to the `debug!` catch-all at [mod.rs:453-455](../../crates/services/src/cell/content/executor/mod.rs#L453-L455):

`ApplyEffect`, `RemoveEffect`, `SpawnEntity`, `DespawnEntity`, `PlayAnimation`, `PlaySound`, `ModifyProperty`, `RollLootTable`, `SpawnLootBag`, `StartTimer`, `CancelTimer`, `ExecuteCustom`, `QrCombatDamage`, `FailObjective`, `LaunchAbility`.

Five of those **are authorable from seed data and are used today** — `launch_ability` (3 rows), `qr_combat_damage` (2), `apply_effect` (1), `remove_effect` (1), `fail_objective` (1). Those 8 `content_actions` rows resolve, log a `debug!`, and do nothing. See the catalog in §3 for the full breakdown.

Two more arms exist but are log-only: `SystemMessage` (11 seeded rows — wire format unknown, see below) and `SendMessage` (no seed verb).

Biggest functional impacts: **no chain can apply a buff, schedule a timer, deal scripted damage, or fire a scripted ability today.** (XP grants and entity moves came off this list in issues #611 and #613.) See [proposed-extensions.md](proposed-extensions.md) for the wiring plan.

**`SystemMessage` wire format is still unresolved** (issue #268). The arm at [executor/mod.rs:275-287](../../crates/services/src/cell/content/executor/mod.rs#L275-L287) carries the reasoning: an earlier implementation routed the message id through `onPlayerCommunication` (method 28), which produced garbled `"[] says"` chat spam and client freezes, so it was reduced to an `info!`. Finding the correct client method for localized string-id display (possibly `onErrorCode` or a UI-specific method) still needs RE.

---

## 11. Performance

**Hot path?** Yes for `fire_entity_death` and `fire_enter_region` (once per kill / region cross). Cold for `fire_player_loaded` and `fire_dialog_*` (sporadic).

**Indexed by trigger type:** `chains_by_trigger: HashMap<TriggerType, Vec<Chain>>` ([chain.rs:58](../../crates/content-engine/src/chain.rs#L58)). For a given event, the engine pulls only the bucket for that `TriggerType` discriminant — no scan over unrelated trigger types.

**Within a bucket: linear scan.** Every chain in the bucket runs `Trigger::matches` (string compare on tag/region/etc.) + condition evaluation. So for `OnEntityDeath`, you pay O(chains_for_EntityDeath) per kill. The worst-case buckets are `OnInteractTag` and `OnRegionEnter` — string keys, no secondary indexing.

**Allocations.** `ResolvedActions.actions` is built fresh per event. `params` is cloned once **only when ≥1 chain matched** ([chain.rs:277-279](../../crates/content-engine/src/chain.rs#L277-L279) — defer-clone optimization from `a51a10d`; previously cloned on every event). Every action match also clones the action enum at [chain.rs:268](../../crates/content-engine/src/chain.rs#L268).

**N×M concern at scale.** Bucket-internal scan is the bottleneck. At current seed sizes (low thousands of chains) it's not measurable. If chain counts grow 10× or AoI events start firing chains per witness, the natural next step is a second-level index keyed by the trigger's filter field (entity_tag, region_key, item_id, dialog_id) — most filters are exact-match string/int equality.

**`fire_event` vs `resolve_event`.** Production uses `resolve_event` exclusively. `fire_event` ([chain.rs:127](../../crates/content-engine/src/chain.rs#L127)) is the in-engine self-executing path that only really works for `Action::TriggerChain`; everything else returns `ActionResult::Error` via `Action::execute`'s catch-all. **Treat `fire_event` as test-only.**

---

## 12. Test infrastructure

### `chain_replay_tests/` — live-DB regression guards

[chain_replay_tests/](../../crates/services/src/cell/content/chain_replay_tests/) loads a specific chain ID from the live seeded DB through the same `build_chains_from_rows` pipeline as production, registers it in a fresh `ChainEngine`, and fires synthetic `TriggerEvent`s with hand-seeded `ExecutionContext`s. Skips cleanly when `DATABASE_URL` is unset via `require_db_or_skip!`.

What it pins:
- mission-status gate semantics (chain 3026 — eq/active/completed leaves; lines 31-163)
- archetype routing correctness (chains 1011/1012 — `assert_region_enter_resolves_dialog_set` shared helper, lines 356-509)
- specific action-list shapes (chain 1034 must contain `RemoveItem { item_id: 19, count: 1 }` exactly once)
- bug-shape regressions (chain 1051 must NOT fire when mission active — the marsh-quest-loop fix from PR #214)

What it catches: SQL seed drift, condition removals, archetype/op flips, action-list shape changes.
What it misses: anything that depends on executor side effects (does `RemoveItem` actually remove? does `MissionUpdate` actually persist?). Those need executor unit tests + live-DB integration tests separately.

Two modules are the exception, and are scoped by **action verb** rather than by mission: [sgc_w1_move_entity.rs](../../crates/services/src/cell/content/chain_replay_tests/sgc_w1_move_entity.rs) and [grant_xp.rs](../../crates/services/src/cell/content/chain_replay_tests/grant_xp.rs). Both push the resolved actions on through `executor::execute_actions` and assert on the emitted `CellToBaseMsg`, because a resolve-only test cannot distinguish a wired executor arm from the `other =>` catch-all — exactly the gap that let `move_entity`’s five seeded rows no-op undetected. `grant_xp` has no seed rows, so its module inserts a sentinel chain (id `0x7000_5000`) and deletes it by exact id before asserting.

### `interact_tag_linter.rs` — boot-free seed-file lint

[interact_tag_linter.rs](../../crates/content-engine/tests/interact_tag_linter.rs) parses seed SQL files line-by-line (no DB, no engine boot) for two invariants:

1. Every `interact_tag` trigger has a matching `set_interaction_type` action **somewhere in the same file** for the same NPC tag, modulo an explicit allowlist with reason comments. Catches the bug class where the chain triggers but no `INT_*` bit is set, so the client renders the entity as scenery and never sends the click.
2. Within a single chain SQL file, every world prefix uses consistent case. The runtime resolver does case-sensitive string match, so `Castle_CellBlock.Region9` vs `Castle_Cellblock.Region9` silently never fires. Caught chain 1073's typo.

What it catches: typo soft-stucks, missing interaction-flag wiring on new chains.
What it misses: cross-file inconsistency (different worlds may legitimately use different cases); semantic-pair gaps that aren't bit-flag-shaped.

### Loader unit tests

[loader/tests/](../../crates/content-engine/src/loader/tests/) — pure-Rust unit tests for every converter, split into `trigger_conversion.rs`, `condition_conversion.rs`, `action_conversion.rs`, and `chain_loading.rs` (every trigger / condition / action variant including `change_stat`, `move_waypoint`, `set_active_slot`, multi-trigger expansion, parse_step_status three-state coverage). No DB required.

### Coverage gaps

There is **no test that exercises FK-broken rows** (chain referencing missing dialog id, action referencing missing item id), and **no test for duplicate trigger rows** producing double-fire.

`enabled = false` *is* covered, and the mechanism is worth stating precisely because it looks like a gap: the boot `SELECT` deliberately does **not** filter on `enabled`, so disabled chains are loaded and registered. They are then skipped in memory at resolve time — [chain.rs:256](../../crates/content-engine/src/chain.rs#L256) for the production `resolve_event` path and [chain.rs:138](../../crates/content-engine/src/chain.rs#L138) for `fire_event`. Pinned by `disabled_chain_is_skipped` ([chain.rs:371](../../crates/content-engine/src/chain.rs#L371)). Load-everything-then-filter is the design, not an oversight.

---

## 13. Recent additions (last 7 days)

| Commit | What landed | Why it matters |
|---|---|---|
| [`76f6759`](../../) `feat(content-engine): add counter state, multi-trigger chains, castle cellblock fixes` | `CellEntity.counters`, `IncrementCounter`/`ResetCounter` executor, `Condition::Counter`, `populate_counters_context`, multi-trigger row expansion, instance_id forwarding through `RemoveItem` | Unlocks kill-N missions, OR-of-events completion, and fixes the bandolier-stack-mismatch bug |
| [`d46b35e`](../../) `feat(content): heal-on-use via Action::ChangeStat amount delta (#220)` | `Action::ChangeStat` with `amount`/`min`/`max`/`set_to_max`/`use_ammo_stat`; `Condition::StatBelowMax` | Health-slappack-style consumables; fail-closed gating prevents heal-spam at full HP |
| [`a51a10d`](../../) `fix(content-engine): address PR review + CI failures` | Bumped increment chains to priority 1; out-of-i32-range guard on `change_stat.amount`; defer-clone of `params` on no-match | Fixes ordering invariant for counter/completion chain pairs; prevents silent wrap on bad seed data |
| [`f3cf5fa`](../../) `fix(bandolier+content): UI sync, marsh quest loop, ambernol consumption` | Marsh quest no-loop guard; ambernol consume fix; chain replay tests for both | Plugs two production-shape regressions |
| [`103225c`](../../) `fix(content): unswap Prisoner 329 archetype-gated dialog routing (#216)` | Chain 1011/1012 archetype-routing test guard | Pins archetype dialog routing against future row swaps |
| [`1a39548`](../../) `test(content-engine): cover counters + populators` | Coverage for `populate_counters_context`, `populate_mission_context`, `item_container` | Fills the test gaps PR #237 created |

The recent direction is clear: **surgical extensions tied to specific shipped content.** New variants ship with seed data, executor wiring, and replay tests in the same PR. No speculative additions.

---

## 14. Related documents

- [extending-the-engine.md](extending-the-engine.md) — How-to: add a new trigger / condition / action.
- [proposed-extensions.md](proposed-extensions.md) — Justified roadmap of future engine extensions.
- [serverEd-comparison.md](serverEd-comparison.md) — Gap analysis vs. the legacy SGW visual-graph editor.
- [architecture/data-driven-content-engine.md](../architecture/data-driven-content-engine.md) — Original design doc (Python prototype; superseded by this Rust implementation).
- [.github/instructions/content-chains.instructions.md](../../.github/instructions/content-chains.instructions.md) — Review rules for chain SQL changes.
- [content/interaction-flags.md](interaction-flags.md) — Per-bit cookbook for `INT_*` interaction flags referenced by `set_interaction_type` actions.
- [content/mission-chains.md](mission-chains.md) — Catalog of every mission chain in the seed data.
- [gameplay/mission-system.md](../gameplay/mission-system.md) — Mission lifecycle from the gameplay-system angle.
