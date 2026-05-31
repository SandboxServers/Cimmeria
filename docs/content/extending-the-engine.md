---
title: "Extending the content engine"
type: reference
audience: engineers
last_updated: 2026-05-27
---

# Extending the content engine

> **Last updated**: 2026-05-07
> **Audience**: Engineers adding a new trigger, condition, or action variant.
> **Prerequisites**: Read [content-engine.md](content-engine.md) first — this how-to assumes you already know what `Trigger`, `Condition`, `Action`, `ExecutionContext`, and the bridge are.
> **Diátaxis type**: How-to guide.

This guide walks you through the three extension shapes the engine supports. Each follows a fixed pattern: declare the variant, teach the loader to parse the SQL row, do the work in the executor (actions only), populate any new context keys (conditions only), and write tests.

---

## Pick the extension shape

| You want to… | Add a… |
|---|---|
| React to a new gameplay event (player did X for the first time, NPC arrived, timer fired) | **Trigger** |
| Gate an existing chain on new state (player has buff X, faction standing, mission repeats) | **Condition** |
| Cause a new effect when a chain fires (set a flag, give currency, trigger a UI) | **Action** |

If your need spans multiple shapes — for instance "gate a chain on item count" requires a `Condition::HasItem` (already exists) **and** a populator that writes `item_<id>_count` (missing) — list each as a separate task.

---

## Add a new action

Use this section as the canonical walkthrough. The `Action::ChangeStat` addition (PR #220, commit `d46b35e`) is the worked example.

### Files to touch

1. **Variant declaration** — [crates/content-engine/src/actions.rs](../../crates/content-engine/src/actions.rs).
2. **Loader arm** — [crates/content-engine/src/loader.rs](../../crates/content-engine/src/loader.rs) `convert_action`.
3. **Executor arm** — [crates/services/src/cell/content/executor.rs](../../crates/services/src/cell/content/executor.rs).
4. **Seed SQL** — [db/resources/Content/Seed/](../../db/resources/Content/Seed/) (or [db/scripts/](../../db/scripts/) for a migration script).
5. **Tests** — unit tests in `executor.rs` + a chain-replay test in [chain_replay_tests.rs](../../crates/services/src/cell/content/chain_replay_tests.rs).

### Step-by-step

1. **Declare the variant.** Add to the `Action` enum:

   ```rust
   /// Doc comment explaining what this does, when it fires, and the order
   /// of any internal sub-operations (see ChangeStat at actions.rs:163-179).
   ChangeStat {
       stat_id: i32,
       amount: Option<i32>,
       // ... other fields, all Option for forward-compat
   },
   ```

   Use `Option<T>` for fields that future seeds might omit. Use `Vec<T>` for list-shaped params (see `StartMinigame.on_victory_chains`).

2. **Add the loader arm.** In `convert_action` ([loader.rs:305-613](../../crates/content-engine/src/loader.rs#L305-L613)):

   ```rust
   "change_stat" => {
       let stat_id = row.target_id?;
       let params = row.params.as_object()?;
       let amount = params.get("amount").and_then(|v| v.as_i64());
       // Validate at the boundary — out-of-range values get warn!+drop, not silent wrap
       let amount = match amount {
           Some(a) if a >= i32::MIN as i64 && a <= i32::MAX as i64 => Some(a as i32),
           Some(a) => {
               warn!("change_stat.amount {a} out of i32 range; dropping action");
               return None;
           }
           None => None,
       };
       Some(Action::ChangeStat { stat_id, amount, /* ... */ })
   }
   ```

   Boundary validation (range checks, enum parsing) goes here. Do not propagate raw DB values into the variant.

3. **Add the executor arm.** In the `match action` block in `executor.rs`:

   ```rust
   Action::ChangeStat { stat_id, amount, .. } => {
       let entity = match space_mgr.get_entity_mut(source_id) {
           Some(e) => e,
           None => {
               warn!("ChangeStat: source entity {source_id} missing");
               continue;
           }
       };
       if let Some(stat) = entity.stats.get_mut(*stat_id) {
           if let Some(delta) = amount {
               stat.change(*delta);
           }
       }
       // Drain dirty stats, send EntityMethodCall { method_index: ON_STAT_UPDATE }
       let payload = entity.stats.serialize_dirty();
       if !payload.is_empty() {
           tx.send(CellToBaseMsg::EntityMethodCall {
               entity_id: source_id,
               method_index: ON_STAT_UPDATE,
               args: payload,
           }).await.ok();
       }
   }
   ```

   The executor is where every channel send and `space_manager` mutation lives. **Do not put any of this in the engine crate.**

4. **Seed it.** A migration script in `db/scripts/` is the canonical pattern:

   ```sql
   BEGIN;
   \set ON_ERROR_STOP on

   INSERT INTO resources.content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
   VALUES (4001, 'Health Slappack TC1 — heal on use', 'global', NULL, true, 0)
   ON CONFLICT (chain_id) DO UPDATE SET description = EXCLUDED.description;

   DELETE FROM resources.content_triggers   WHERE chain_id = 4001;
   DELETE FROM resources.content_conditions WHERE chain_id = 4001;
   DELETE FROM resources.content_actions    WHERE chain_id = 4001;

   INSERT INTO resources.content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
   VALUES (4001, 'item_use', '2893', 'player', false, 0);

   INSERT INTO resources.content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
   VALUES (4001, 'stat_below_max', 7, NULL, 'eq', 'true', 0);

   INSERT INTO resources.content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
   VALUES
     (4001, 'change_stat', 7,    NULL, '{"amount": 500}', 0, 0),
     (4001, 'remove_item', 2893, NULL, '{"qty": 1}',      0, 1);

   COMMIT;
   ```

   The delete-then-insert for child rows + `ON CONFLICT` upsert for the parent makes the script idempotent.

5. **Test it.** Three layers:

   - **Unit tests in `executor.rs`** — five tests for `ChangeStat`: advance, clamp at max, clamp at min (negative damage), `set_to_max`, `use_ammo_stat=true` early-return ([executor.rs:1069-1310](../../crates/services/src/cell/content/executor.rs#L1069-L1310)).
   - **Loader unit tests in `loader.rs`** — round-trip a representative SQL row through `convert_action` and assert the variant shape ([loader.rs:660-1050](../../crates/content-engine/src/loader.rs#L660-L1050)).
   - **Chain-replay test in `chain_replay_tests.rs`** — load the seeded chain through `load_single_chain_for_test`, fire the trigger event, assert the `ResolvedActions` shape. Pin **behavior, not chain_id** — a future renumber should not break the guard.

   ```rust
   #[tokio::test]
   async fn item_use_2893_resolves_to_health_slappack_heal_and_consume() {
       require_db_or_skip!();
       // ... see engine_loader.rs:277-357 for the full pattern
   }
   ```

6. **Update docs.** Add the new variant to the action table in [content-engine.md](content-engine.md) §3 and (if it's a non-trivial pattern) document its execution-order semantics.

### Action checklist

- [ ] Variant declared in `actions.rs` with rustdoc covering what it does and any ordering invariants
- [ ] Loader arm in `loader.rs` with boundary validation
- [ ] Executor arm in `executor.rs`
- [ ] At least one seed row in `db/resources/Content/Seed/` or `db/scripts/`
- [ ] Unit tests for the executor branch
- [ ] Unit test for the loader converter
- [ ] Chain-replay test pinning behavior shape
- [ ] Reference table in [content-engine.md](content-engine.md) §3 updated

---

## Add a new condition

The pattern is similar to actions, with one extra step: the condition reads from `ExecutionContext.params`, which means you may need a **populator** to write the param.

### Files to touch

1. **Variant declaration** — [crates/content-engine/src/conditions.rs](../../crates/content-engine/src/conditions.rs).
2. **Loader arm** — [loader.rs](../../crates/content-engine/src/loader.rs) `convert_condition`.
3. **Populator** — [crates/services/src/cell/content/mission_context.rs](../../crates/services/src/cell/content/mission_context.rs), if the condition reads context keys nothing else writes.
4. **Tests.**

### Step-by-step

1. **Declare the variant** in `conditions.rs` and implement `evaluate`:

   ```rust
   Condition::MissionRepeats { mission_id, op, value } => {
       let key = format!("mission_{mission_id}_repeats");
       let actual = ctx.params.get(&key).and_then(|v| v.as_i64()).unwrap_or(0);
       compare_i64(actual, op, *value as i64)
   }
   ```

2. **Decide on the populator strategy.**

   - If the condition reads a **mission-related key**, add to [mission_context.rs](../../crates/services/src/cell/content/mission_context.rs):

     ```rust
     pub fn populate_mission_context(entity: &CellEntity, ctx: &mut ExecutionContext) {
         // ... existing code ...
         for mission in entity.missions.all_missions() {
             ctx.params.insert(
                 format!("mission_{}_repeats", mission.mission_id),
                 json!(mission.repeats),
             );
         }
     }
     ```

   - If the condition reads a **non-mission** key (faction, inventory count, ability state), write a new populator and call it from every `fire_*` site that uses the condition.

3. **Fail-closed when the populator is missing**, if the safe default is "skip the chain":

   ```rust
   Condition::StatBelowMax { stat_id } => {
       let cur = match ctx.params.get(&format!("stat_{stat_id}_cur")) {
           Some(v) => v.as_i64().unwrap_or(0),
           None => return false,  // populator missing -> fail closed
       };
       // ... see conditions.rs:255-268 for the full pattern
   }
   ```

   Fail-closed is the right default for *gating* conditions (don't heal at full HP). For *informational* conditions (like `Counter`), the missing-key-→-zero default is correct because zero counters are conceptually equivalent to never-incremented.

4. **Loader arm.** In `convert_condition`:

   ```rust
   "mission_repeats" => {
       let mission_id = row.target_id?;
       let op = parse_op(&row.operator)?;
       let value = row.value.parse::<i32>().ok()?;
       Some(Condition::MissionRepeats { mission_id, op, value })
   }
   ```

5. **Tests.**

   - Unit test for the evaluator (positive match, negative match, missing-key fallback)
   - Unit test for the loader converter
   - Unit test for the populator (verify the key is written for both populated and "should be zero" cases — see `populate_counters_context_writes_counter_keys` at [mission_context.rs:228-251](../../crates/services/src/cell/content/mission_context.rs#L228-L251))
   - If a real mission shape uses the new condition, add a chain-replay test

### Condition checklist

- [ ] Variant declared with `evaluate` impl
- [ ] Fail-mode chosen explicitly (open vs closed) and documented in the rustdoc
- [ ] Loader arm in `loader.rs`
- [ ] Populator added (if reading a new context key)
- [ ] Populator called from every `fire_*` site that uses this condition
- [ ] Unit tests for evaluator, loader, populator
- [ ] Chain-replay test if the condition gates production content

---

## Add a new trigger

Triggers are the most invasive extension shape because they need a new event-dispatch site and (usually) a wire from gameplay code that observes the event.

### Files to touch

1. **Variant declaration** — [crates/content-engine/src/triggers.rs](../../crates/content-engine/src/triggers.rs).
2. **Discriminant + matcher** — same file, extend `TriggerType` and `Trigger::trigger_type` and `Trigger::matches`.
3. **Loader arm** — `convert_trigger` in `loader.rs`.
4. **Dispatcher** — write a `fire_<event>` function in [event_dispatch.rs](../../crates/services/src/cell/content/event_dispatch.rs).
5. **Public re-export** — add to [mod.rs](../../crates/services/src/cell/content/mod.rs).
6. **Wire** — call the new `fire_<event>` from the gameplay code that observes the event.
7. **Tests.**

### Step-by-step

1. **Declare the variant.** In `triggers.rs`:

   ```rust
   pub enum Trigger {
       // ...
       OnNpcArrived { entity_tag: String, region_key: String },
   }
   ```

2. **Extend `TriggerType`** with the matching discriminant (`NpcArrived`).

3. **Extend `Trigger::trigger_type`** ([triggers.rs:148](../../crates/content-engine/src/triggers.rs#L148)) and **`Trigger::matches`** ([triggers.rs:178](../../crates/content-engine/src/triggers.rs#L178)). The matcher should filter on whatever secondary keys the variant carries (entity_tag, region_key, etc.).

4. **Add the loader arm:**

   ```rust
   "npc_arrived" => Some(Trigger::OnNpcArrived {
       entity_tag: row.event_key.clone(),
       region_key: row.params.get("region_key")?.as_str()?.to_string(),
   }),
   ```

5. **Write the dispatcher.** In `event_dispatch.rs`:

   ```rust
   pub async fn fire_npc_arrived(
       entity_id: i64,
       entity_tag: &str,
       region_key: &str,
       engine: &ChainEngine,
       tx: &mpsc::Sender<CellToBaseMsg>,
       space_mgr: &mut SpaceManager,
   ) {
       let mut ctx = ExecutionContext::new();
       ctx.source_entity = Some(entity_id);
       ctx.params.insert("entity_tag".into(), json!(entity_tag));
       ctx.params.insert("region_key".into(), json!(region_key));

       // Populate any conditions you expect this trigger's chains to use
       if let Some(entity) = space_mgr.get_entity(entity_id) {
           populate_mission_context(entity, &mut ctx);
       }

       let event = TriggerEvent {
           trigger_type: TriggerType::NpcArrived,
           params: ctx.params.clone(),
           // ...
       };

       let resolved = engine.resolve_event(&event, &ctx);
       executor::execute_actions(resolved, entity_id, tx, space_mgr).await;
   }
   ```

   **Always populate every context key your conditions might read.** A condition that reads `mission_<id>_status` against a `fire_npc_arrived` site that forgot to call `populate_mission_context` will silently fail-open or fail-closed depending on the variant — see [content-engine.md](content-engine.md) §10.

6. **Re-export from `mod.rs`:**

   ```rust
   pub use event_dispatch::{fire_npc_arrived, /* ... */};
   ```

7. **Wire it.** Find the gameplay code that observes the event (e.g. the path-completion callback in `space_manager`) and call `fire_npc_arrived` from there.

8. **Tests.**

   - Unit test for `Trigger::matches` covering the variant's filter logic
   - Unit test for the loader converter
   - Async unit test for the `fire_*` site that calls it against an empty `ChainEngine` (no panic + correct context shape — see the existing patterns in `mod.rs#tests`)
   - Chain-replay test if the trigger gates production content

### Trigger checklist

- [ ] `Trigger` variant + `TriggerType` discriminant declared
- [ ] `trigger_type()` and `matches()` extended
- [ ] Loader arm in `loader.rs`
- [ ] `fire_<event>` written, with all needed populators called
- [ ] Re-exported from `mod.rs`
- [ ] Wired from the gameplay-code site that observes the event
- [ ] Unit + replay tests
- [ ] Reference table in [content-engine.md](content-engine.md) §3 updated

---

## Common gotchas

| Gotcha | Symptom | Fix |
|---|---|---|
| Forgot to call a populator from a new `fire_*` site | Chain matches in tests, never matches in production | Add the populator call; verify with a replay test |
| Used the same key for "zero" and "missing" | `Counter == 0` matches both never-incremented and explicitly-zeroed | Populate explicitly when the value is genuinely zero (see `populate_counters_context` invariant) |
| Two chains at the same priority on the same trigger with order-dependent actions | Intermittent failures depending on iteration order | Bump one chain's priority (see PR #237 / `a51a10d` increment-vs-completion fix) |
| Loader silently dropped the row | Chain partially loaded; `warn!` at boot but no production observability | Read `loader.rs` warn lines at startup; add CHECK constraint on the discriminator if the type is part of a stable contract |
| Action defined but no executor arm | Chain fires, action falls through to `debug!("Unhandled")` no-op | Audit the `executor.rs` match arms before assuming a variant works (current dead variants: `ApplyEffect`, `RemoveEffect`, `StartTimer`, `CancelTimer`, `RollLootTable`, `SpawnEntity`, `GrantXP` — see [proposed-extensions.md](proposed-extensions.md)) |
| `interact_tag` chain author forgot the `INT_*` flag | Right-click does nothing in-game (entity rendered as scenery) | The linter at [interact_tag_linter.rs](../../crates/content-engine/tests/interact_tag_linter.rs) should catch this; if you legitimately need to skip it, add to the allowlist with a reason comment |
| Chain references a missing dialog/item/mission ID | Loads fine; fails at action execute time with a `warn!` | Run the validation CLI (proposed in [proposed-extensions.md](proposed-extensions.md)); for now, manually grep the seeds |

---

## Related

- [content-engine.md](content-engine.md) — Reference for what already exists.
- [proposed-extensions.md](proposed-extensions.md) — The justified roadmap of extensions still to come.
- [.github/instructions/content-chains.instructions.md](../../.github/instructions/content-chains.instructions.md) — Review rules for chain SQL.
- [TESTING.md](../../TESTING.md) — Test type picker.
