# Content engine — reference

> **Last updated**: 2026-05-07
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

Implementation status (2026-05-07): **shipped and driving Castle_CellBlock and SGC_W1 end-to-end.** The Health Slappack consumable and the Mess Hall kill-counter mission are the latest content shapes wired up; both rely on counter state and `ChangeStat` added in the last week.

---

## 2. Architecture — the two-crate split

### Pure engine ([crates/content-engine/src/](../../crates/content-engine/src/))

| File | Owns |
|---|---|
| [lib.rs](../../crates/content-engine/src/lib.rs) | Module rollup, re-exports |
| [chain.rs](../../crates/content-engine/src/chain.rs) | `Chain`, `ChainEngine`, `ResolvedActions`, `resolve_event` |
| [triggers.rs](../../crates/content-engine/src/triggers.rs) | `Trigger`, `TriggerType`, `TriggerEvent`, `Trigger::matches` |
| [conditions.rs](../../crates/content-engine/src/conditions.rs) | `Condition`, `Condition::evaluate` |
| [actions.rs](../../crates/content-engine/src/actions.rs) | `Action`, `ActionResult`, `PropertyOp` |
| [context.rs](../../crates/content-engine/src/context.rs) | `ExecutionContext` (param key/value bag) |
| [loader.rs](../../crates/content-engine/src/loader.rs) | DB-row → typed enum conversion |

This crate does not depend on `cimmeria-services`, `cimmeria-base`, or `tokio` runtime types. Its full dep set is `cimmeria-common`, `cimmeria-entity`, `serde`, `serde_json`, `thiserror`, `tracing` ([Cargo.toml:9-15](../../crates/content-engine/Cargo.toml#L9-L15)).

### Bridge ([crates/services/src/cell/content/](../../crates/services/src/cell/content/))

| File | Owns |
|---|---|
| [mod.rs](../../crates/services/src/cell/content/mod.rs) | Public re-exports for the rest of the cell service |
| [engine_loader.rs](../../crates/services/src/cell/content/engine_loader.rs) | `build_engine` — runs the four boot SQL queries |
| [event_dispatch.rs](../../crates/services/src/cell/content/event_dispatch.rs) | `fire_<event>` factory functions, one per `TriggerType` |
| [executor.rs](../../crates/services/src/cell/content/executor.rs) | `execute_actions` — the giant `match action { ... }` |
| [mission_context.rs](../../crates/services/src/cell/content/mission_context.rs) | Populators: write mission/counter/stat state into `ExecutionContext` |
| [chain_replay_tests.rs](../../crates/services/src/cell/content/chain_replay_tests.rs) | Live-DB regression guards that pin chain behavior |

The bridge owns every effect (channel sends, `space_manager` mutations, log lines). The engine never produces a side effect — it only resolves which actions the bridge should run.

---

## 3. The vocabulary

### Triggers — *what fires the chain*

Defined at [triggers.rs:20-98](../../crates/content-engine/src/triggers.rs#L20-L98). Filterable by an optional second key (entity type, item id, region key, etc.) per variant.

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
| `OnDialogChoice { dialog_id }` | Player clicked a dialog button |
| `OnInteractTag { entity_tag }` | Right-click on tagged NPC/object |
| `OnInteractTemplate { template_name }` | Right-click on entity from named template |
| `OnItemUse { item_id }` | Player double-clicked inventory item |
| `OnTeleportIn { region_id }` | Player arrived via ring transporter |
| `OnEffectInit / PulseBegin / PulseEnd / Removed` | Effect lifecycle hooks (unit variants) |
| `OnMissionCompleted { mission_id }` | Mission marked complete |
| `OnDialogSetOpen { dialog_set_name }` | Dialog set opened |
| `OnMissionAccepted { mission_id }` | Mission just accepted or advanced (fired from the executor's combined `Action::AcceptMission \| Action::AdvanceMission` branch after the cell-side state commit; used by chains that highlight quest objects on mission start — e.g. chain 1097 for Aftermath) |

Within a single chain's bucket, `Trigger::matches` ([triggers.rs:178](../../crates/content-engine/src/triggers.rs#L178)) decides whether the event matches the chain's specific trigger variant + filter. Bucketing is by **`TriggerType` discriminant** — see §6.

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

### Actions — *side effects*

Defined at [actions.rs:20-237](../../crates/content-engine/src/actions.rs#L20-L237). The full surface is large — listed in three groups by lineage. **`Action::execute` is a stub** ([actions.rs:264-277](../../crates/content-engine/src/actions.rs#L264-L277)); only `TriggerChain` self-executes. Everything else is dispatched by [executor.rs](../../crates/services/src/cell/content/executor.rs).

**Generic actions** (designed up front):
`GrantXP`, `GrantItem`, `RemoveItem`, `ApplyEffect`, `RemoveEffect`, `Teleport`, `SpawnEntity`, `DespawnEntity`, `StartDialog`, `AdvanceMission`, `CompleteMission`, `PlayAnimation`, `PlaySound`, `SendMessage`, `ModifyProperty`, `RollLootTable`, `SpawnLootBag`, `StartTimer`, `CancelTimer`, `TriggerChain`, `ExecuteCustom`.

**DB-driven actions** (added as content shipped):
`AcceptMission`, `DisplayDialog`, `AddDialogSet`, `RemoveDialogSet`, `PlaySequence`, `AdvanceStep`, `SetInteractionType`, `StartMinigame`, `SetAggression`, `DestroyTaggedEntity`, `TriggerTransporter`, `SystemMessage`, `QrCombatDamage`, `ChangeStat`, `AbandonMission`, `FailObjective`, `CompleteObjective`, `IncrementCounter`, `ResetCounter`, `SetVisible`, `MoveEntity`.

**Space-script actions** (auto-converted from level scripts):
`MoveWaypoint`, `SetActiveSlot`, `LaunchAbility`, `AddDialog`, `GenerateThreat`.

> Several variants are **defined but not executed** today: `ApplyEffect`, `RemoveEffect`, `StartTimer`, `CancelTimer`, `RollLootTable`, `SpawnEntity`, `GrantXP`. The loader accepts them and the engine resolves them, but `executor.rs` has no match arm — they fall through to a `debug!` no-op. See [proposed-extensions.md](proposed-extensions.md) for the wiring plan.

---

## 4. The execution model

End-to-end trace, using `OnItemUse(2893)` (Health Slappack) as the worked example.

1. **Gameplay observes the event.** Player double-clicks the Slappack. `crate::cell::content::fire_item_use(...)` is called from [base_messages/mod.rs](../../crates/services/src/cell/service/base_messages/mod.rs).
2. **The bridge builds an `ExecutionContext`.** [event_dispatch.rs:393-415](../../crates/services/src/cell/content/event_dispatch.rs#L393-L415):
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
5. **Bridge executes.** `executor::execute_actions(resolved, …)` ([executor.rs:27](../../crates/services/src/cell/content/executor.rs#L27)). For chain 4001, the action sequence is `ChangeStat { stat_id: 7, amount: Some(500) }` then `RemoveItem { item_id: 2893, count: 1 }`.
   - `ChangeStat` ([executor.rs:474-575](../../crates/services/src/cell/content/executor.rs#L474-L575)) mutates `entity.stats.get_mut(7).change(500)`, drains dirty stats, sends `CellToBaseMsg::EntityMethodCall { method_index: ON_STAT_UPDATE, args: payload }`.
   - `RemoveItem` ([executor.rs:404-473](../../crates/services/src/cell/content/executor.rs#L404-L473)) reads the forwarded `instance_id` param; routes to `RemoveInventoryItem` (by-instance) when present, falls back to `RemoveInventoryItemByType` otherwise. The instance plumbing is what fixes the bandolier-stack-mismatch bug from PR #214.
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
5. `build_chains_from_rows` ([loader.rs:73-190](../../crates/content-engine/src/loader.rs#L73-L190)) groups triggers/conditions/actions by `chain_id`, sorts each group by `sort_order`, converts each row to a typed enum, and **expands multi-trigger chains** by emitting one in-memory `Chain` per trigger row ([loader.rs:147-166](../../crates/content-engine/src/loader.rs#L147-L166)).
6. A chain with **zero trigger rows** receives a synthetic `OnCustomEvent("__direct_invoke_<id>")` so it stays callable via `on_victory_chains` arrays and `Action::TriggerChain` ([loader.rs:147-149](../../crates/content-engine/src/loader.rs#L147-L149)).
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
| `AcceptMission`, `CompleteMission`, `AdvanceStep`, `CompleteObjective`, `FailObjective`, `AbandonMission` | Routes through `crate::cell::missions::*` → emits `CellToBaseMsg::MissionUpdate` → BaseApp `UPSERT sgw_mission` at [missions.rs:103-127](../../crates/services/src/base/world_entry/methods/missions.rs#L103-L127) |
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

- **Mutation.** `Action::IncrementCounter` and `Action::ResetCounter` mutate `entity.counters` directly at [executor.rs:727-767](../../crates/services/src/cell/content/executor.rs#L727-L767). Increment uses `saturating_add`. Reset *removes* the entry (line 761) — not zeros it.
- **Read into ctx.** `populate_counters_context` ([mission_context.rs:37-41](../../crates/services/src/cell/content/mission_context.rs#L37-L41)) writes `counter_<name>` into `ExecutionContext.params` for every entry. Called from `populate_mission_context`, so every mission-aware dispatcher gets counters automatically.
- **Read in chains.** `Condition::Counter` reads `counter_<name>` ([conditions.rs:246-254](../../crates/content-engine/src/conditions.rs#L246-L254)). Missing key → 0. The zero-elision invariant is: **genuinely-zero counters MUST populate explicitly** so `Counter == 0` distinguishes them from "never-incremented." Pinned by `populate_counters_context_writes_counter_keys` at [mission_context.rs:228-251](../../crates/services/src/cell/content/mission_context.rs#L228-L251).

### The hidden ordering invariant

`a51a10d` fixed a bug where increment chains (1085, 1086, 1092, 1093) and completion chains (1087, 1094) lived at the same priority on the same `OnEntityDeath` trigger. Equal-priority ordering inside a `chains_by_trigger` bucket is undefined — so a completion chain's `ResetCounter` could fire before the increment chain on the same kill, leaving the next mission with a stale non-zero counter.

The fix bumped increment chains to priority 1; completion chains stay at 0; the bucket sort is descending. **Conditions evaluate before any sibling action in the same trigger pass executes** — so a "kill N" completion chain whose condition reads `counter` sees the **pre-increment** value. Hence the documented `counter >= target - 1` pattern at [executor.rs:735-743](../../crates/services/src/cell/content/executor.rs#L735-L743): the chain fires on the kill that brings the counter to N, not after.

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

Worked example chains in [chain_replay_tests.rs](../../crates/services/src/cell/content/chain_replay_tests.rs):

- **Mission 622 "Arm Yourself"** — chains 1001 (region-accept), 1003 (dialog-complete + reward).
- **Mission 638 "Prisoner 329"** — chains 1011/1012 (archetype-routed dialog) — pinned by `assert_region_enter_resolves_dialog_set` ([chain_replay_tests.rs:356-509](../../crates/services/src/cell/content/chain_replay_tests.rs#L356-L509)).
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

| Failure | Surfaces as | Site |
|---|---|---|
| Unknown DB `event_type`/`condition_type`/`action_type` | `warn!` at [loader.rs:114-127](../../crates/content-engine/src/loader.rs#L114-L127); row dropped silently from the chain (chain still loads, just missing that piece) |
| All trigger rows fail to convert | `warn!` + chain skipped ([loader.rs:168-174](../../crates/content-engine/src/loader.rs#L168-L174)) |
| `change_stat.amount` out of i32 range | `warn!` + entire action dropped ([loader.rs:467-474](../../crates/content-engine/src/loader.rs#L467-L474)) |
| Missing condition param (e.g. `mission_*_status` not populated) | Silent fall-through to evaluator default (`unwrap_or("not_active")`); chain may match or miss unexpectedly |
| Missing populator for `StatBelowMax` | **Fail-closed**: returns `false` ([conditions.rs:260-267](../../crates/content-engine/src/conditions.rs#L260-L267)). Deliberate. |
| Trigger filter mismatch | `trace!` at [chain.rs:144-150](../../crates/content-engine/src/chain.rs#L144-L150) — only visible at trace level |
| Condition fails | `trace!` at [chain.rs:163-169](../../crates/content-engine/src/chain.rs#L163-L169) |
| Action `Error` result | `warn!` at [chain.rs:201-209](../../crates/content-engine/src/chain.rs#L201-L209) |
| `RemoveItem` channel send fails | `error!` at [executor.rs:466-471](../../crates/services/src/cell/content/executor.rs#L466-L471) — explicitly loud because mission progress depends on the consume |
| `ChangeStat` source entity missing | `warn!` at [executor.rs:550-556](../../crates/services/src/cell/content/executor.rs#L550-L556) |
| Empty engine on startup | `warn!` ("No DB pool available") or `error!` ("Failed to load") at [engine_loader.rs:33-41](../../crates/services/src/cell/content/engine_loader.rs#L33-L41) — server runs without content |

The fire-time logs (`info!` on match, `debug!` on no-match) at every `fire_*` site in [event_dispatch.rs](../../crates/services/src/cell/content/event_dispatch.rs) are the production observability story. Every action execution emits an `info!` with `chain_id`, the action params, and entity. Tracing-grep for `Content:` to scope to executor activity.

### Defined-but-unhandled actions

`ApplyEffect`, `RemoveEffect`, `StartTimer`, `CancelTimer`, `RollLootTable`, `SpawnEntity`, `GrantXP` — the loader accepts them ([loader.rs:488-493](../../crates/content-engine/src/loader.rs#L488-L493) for the effect pair) and the engine resolves them, but `executor.rs` has no match arm. They fall through to a `debug!("Unhandled action: ...")` and silently no-op. The biggest functional impact: **no chain can grant XP, apply a buff, or schedule a timer today.** See [proposed-extensions.md](proposed-extensions.md) for the wiring plan.

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

### `chain_replay_tests.rs` — live-DB regression guards

[chain_replay_tests.rs](../../crates/services/src/cell/content/chain_replay_tests.rs) loads a specific chain ID from the live seeded DB through the same `build_chains_from_rows` pipeline as production, registers it in a fresh `ChainEngine`, and fires synthetic `TriggerEvent`s with hand-seeded `ExecutionContext`s. Skips cleanly when `DATABASE_URL` is unset via `require_db_or_skip!`.

What it pins:
- mission-status gate semantics (chain 3026 — eq/active/completed leaves; lines 31-163)
- archetype routing correctness (chains 1011/1012 — `assert_region_enter_resolves_dialog_set` shared helper, lines 356-509)
- specific action-list shapes (chain 1034 must contain `RemoveItem { item_id: 19, count: 1 }` exactly once)
- bug-shape regressions (chain 1051 must NOT fire when mission active — the marsh-quest-loop fix from PR #214)

What it catches: SQL seed drift, condition removals, archetype/op flips, action-list shape changes.
What it misses: anything that depends on executor side effects (does `RemoveItem` actually remove? does `MissionUpdate` actually persist?). Those need executor unit tests + live-DB integration tests separately.

### `interact_tag_linter.rs` — boot-free seed-file lint

[interact_tag_linter.rs](../../crates/content-engine/tests/interact_tag_linter.rs) parses seed SQL files line-by-line (no DB, no engine boot) for two invariants:

1. Every `interact_tag` trigger has a matching `set_interaction_type` action **somewhere in the same file** for the same NPC tag, modulo an explicit allowlist with reason comments. Catches the bug class where the chain triggers but no `INT_*` bit is set, so the client renders the entity as scenery and never sends the click.
2. Within a single chain SQL file, every world prefix uses consistent case. The runtime resolver does case-sensitive string match, so `Castle_CellBlock.Region9` vs `Castle_Cellblock.Region9` silently never fires. Caught chain 1073's typo.

What it catches: typo soft-stucks, missing interaction-flag wiring on new chains.
What it misses: cross-file inconsistency (different worlds may legitimately use different cases); semantic-pair gaps that aren't bit-flag-shaped.

### Loader unit tests

[loader.rs:660-1050](../../crates/content-engine/src/loader.rs#L660-L1050) — pure-Rust unit tests for every converter (every trigger / condition / action variant including `change_stat`, `move_waypoint`, `set_active_slot`, multi-trigger expansion, parse_step_status three-state coverage). No DB required.

### Coverage gaps

There is **no test that exercises FK-broken rows** (chain referencing missing dialog id, action referencing missing item id), no test for **duplicate trigger rows** producing double-fire, no test for **`enabled = false`** filtering at the loader (the loader currently doesn't filter — `enabled` is honored only if the resolver checks it).

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
