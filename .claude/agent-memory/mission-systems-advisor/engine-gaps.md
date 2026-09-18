---
name: engine-gaps
description: Content-engine capability gaps that block mission porting — dead triggers, loader-parseable actions with no executor arm, and the dead-code objective enum.
metadata:
  type: project
---

# Content-engine gaps (verified against main, 2026-09-17)

## Triggers that parse but are NEVER dispatched

`dialog_set_open`, `effect_init`, `effect_pulse_begin`, `effect_pulse_end`,
`effect_removed`. No `fire_*` site raises them.

**`dialog_set_open` is the painful one.** It is the trigger the original Atrea
`Event_DialogSetMap` node compiles to (`owner.subscribe("dialog_set.open::"+id)`),
and it carries a `target` entity the python reads a **tag** off of. Cimmeria's
`OnDialogSetOpen { dialog_set_name }` carries only a name and never fires. Any
python mission using "open a dialog set on a prop, branch on the prop's tag"
(Harset 742's bug-planting step, for one) has **no direct port**. Workaround is
`interact_tag` per prop — one chain per tag instead of one chain with a compare.

`mission_accepted` / `mission_completed` fire **only** from inside the executor's
own accept/complete arms. A mission accepted by any non-chain path raises nothing.

## Actions that parse but have NO executor arm (silent no-op, `debug!` only)

`qr_combat_damage`, `apply_effect`, `remove_effect`, `fail_objective`,
`move_entity`, `launch_ability`. `system_message` has an arm that only logs
(wire format unknown, issue #268).

Partial: `change_stat` no-ops on `use_ammo_stat`/negative stat id;
`move_waypoint` discards `speed` (instant snap, not pathing);
**`set_visible` is effectively broken for NPCs** — it sends an `EntityMethodCall`
routed through a player-only address map, so nothing reaches any client.

## Actions with no DB key at all (unauthorable)

`GrantXP`, `Teleport` (same-space), `SpawnEntity`, `DespawnEntity`,
`PlayAnimation`, `PlaySound`, `ModifyProperty`, `RollLootTable`, `SpawnLootBag`,
`StartTimer`, `CancelTimer`, `TriggerChain`, `ExecuteCustom`, `SendMessage`.

**No spawn action of any kind, and no currency/cash action at all.**

## Conditions

Only six authorable: `mission_status`, `step_status`, `archetype`,
`objective_status`, `counter`, `stat_below_max`. No inventory-contents
condition (`HasItem` has no DB key *and* no populator) and **no position /
proximity condition** (`InRegion` has no DB key).

## No per-player entity state — anywhere

`set_visible`, `set_aggression`, `set_interaction_type` all mutate a shared
entity field and broadcast to every witness. The only per-player state the
engine has is `entity.counters` on the player's own entity. So "NPC alive for
player A, corpse for player B" and "spawn this NPC for one party" are not
approximable — they need new engine work, not clever chain authoring.

## No timers

Three independent misses: `StartTimer`/`CancelTimer` unauthorable + no arm;
`Trigger::OnTimer` has no `event_type` string; `content_actions.delay_ms` is
read into `DbActionRow` and then discarded. Only
`content/castle-cellblock-rebuild` fixes the third (deferred-action queue on the
100 ms cell tick).

## `mission_steps.step_enabled = false` means NOTHING — do not treat it as a signal

Two independent proofs:

1. **No loader reads it.** `crates/services/src/cell/spawner/missions.rs:54-59` and
   `:86-90` select from `mission_steps` / `mission_objectives` with **no `WHERE`
   on `step_enabled` or `is_enabled`**. Same for `mission_objectives.is_enabled`.
2. **Every shipped, working mission has it false.** Castle Cellblock 622, 638,
   639, 641, 682, 687, 688 — all chain-replay-tested, all rendering steps in the
   live client — are `0 enabled / N steps`. Mission 622's step 2113 is the
   worked example in `docs/engine/cooked-data-pak-format.md:462` showing the
   client PAK carrying it fine.

Seed-wide it is 1,497 true / 1,982 false, so `false` is close to the file
default. `mission_tasks.is_enabled` is `true` on all 4,358 rows.

A spec that infers "all steps disabled ⇒ this content was switched off / must be
re-enabled" is reading a flag the server never consults. Push back on it.

## `crates/game/src/missions/objectives.rs` is DEAD CODE

The `MissionObjective` enum (KillCount / CollectItem / VisitRegion / TalkToNpc /
UseObject) is referenced **nowhere outside its own module** — grep for
`MissionObjective::` across `crates/` returns hits only in that file. There is
no objective-primitive runtime. Live mission progress is 100% hand-authored
chains using `complete_objective` / `increment_counter` + `Condition::Counter`.
Do not cite those primitives as if they were implemented.

Corollary: `resources.mission_tasks.task_type` is not read by any Rust code.

## Branch status (2026-09-17, none merged)

- `content/castle-cellblock-rebuild` — adds `delay_ms` honoring only
  (`Chain.action_delays` + `deferred_content_actions.rs`). Refactors the
  executor match into `execute_one_action`, so it textually conflicts with the
  two feat branches.
- `feat/content-move-entity-grant-xp` — adds `grant_xp` loader key + `GrantXP`
  and `MoveEntity` executor arms.
- `feat/content-effect-apply-entry-point` — **strict superset of the above**,
  plus `effect_apply.rs`, `LaunchAbility` and `ApplyEffect` arms. Supersedes the
  move-entity branch entirely.

## Mission-progression semantics (verified 2026-09-18, castle-m706 worktree)

### `resolve_event` is a single-snapshot resolve, then sequential execute

`content-engine/src/chain/mod.rs:288-325`: **every** matching chain's conditions
are evaluated against the *same* pre-action `ExecutionContext`, then all their
actions are concatenated and run in order. So for one event, chain B gated on
`step_status X eq active` still fires even if chain A already advanced past X.
This is what makes "kill → advance step" + "kill → set the next NPC's `!`"
co-fire correctly, and it is the same property the Cellblock counter chains
document as the "target - 1" rule (`castle_cellblock_chains.sql:1445-1477`).
Re-entrancy (an action that fires a nested dispatcher) *does* see post-mutation
state — see the accept/complete note below.

### `complete_objective` can end a mission early — check `is_optional` first

`cell/missions/progression.rs:176-213`: after marking one objective complete it
tests `active_objectives.iter().filter(|o| !o.optional).all(completed)` and, if
true, calls `mission.complete()`. **A step whose only non-optional objectives are
already done — or a step with *zero* non-optional objectives — completes the whole
mission on the first `complete_objective`.** Always grep `mission_objectives.sql`
for `is_optional` on the target step before authoring `complete_objective`.
(Castle step 2415 is safe only because hidden objective 2796 is `is_optional=false`.)

`advance_step` (`progression.rs:57-66`) force-completes every not-yet-complete
objective of the outgoing step and never runs the all-required check, so
`complete_objective <the one the player did>` then `advance_step <next>` is the
correct "leave a multi-objective step" idiom. Two warts: it emits no
`onObjectiveUpdate` for the force-completed ones, and it force-completes
*optional* ones too (both "(Option #1)" and "(Option #2)" tick).

Prefer `complete_mission` over N× `complete_objective` for a final step:
`complete_mission_direct` (`progression.rs:226-309`) sends
`onMissionUpdate(status = STATUS_COMPLETED)`, whereas `complete_objective`'s
auto-complete path sends `MISSION_ACTIVE` as the status byte
(`progression.rs:202`) — almost certainly a bug, and a reason not to rely on it.

### `active_objective_ids` is persisted as the STEP id — objective state does not survive relog

`cell/content/executor/mission.rs:85` and `:242` send
`active_objective_ids: vec![step_id]` and `completed_objective_ids: vec![]`.
`player_init/mod.rs:169-189` rebuilds `active_objectives` from that list with
`hidden: false, optional: false`. So after a relog a mission's
`active_objectives` is `[<step_id masquerading as an objective>]`:
`complete_objective` no-ops, `advance_step` still works, objective ticks in the
client log are lost, and `complete_mission_direct` emits
`onObjectiveUpdate(<step_id>, COMPLETED)`. Multi-step ports must not depend on
objective state across a relog.

### `set_interaction_type` is GLOBAL; `add_dialog_set` is PER-PLAYER

`executor/world/mod.rs:19-64` mutates the shared `CellEntity.interaction_type_flags`
and broadcasts to every witness. `space_manager/aoi.rs:117-180` sends that value
as the AoI-create base and merges the *per-player* `available_interactions`
(from `add_dialog_set`) on top as a separate `InteractionType` update. Quest
glyphs painted with `set_interaction_type` are therefore visible to, and
clobberable by, every other player in the zone. Entities start from their
template's `interaction_type` column (`space_manager/spawn.rs:127`), e.g.
template 162 `DHD_Frost` is already `INT_DHD = 16` at spawn.

### `content_triggers.scope` and `content_chains.scope_type/scope_id` are DEAD

`loader/mod.rs:88-215` reads them into the row structs and never uses them,
exactly like the `once` column. Only conditions gate a chain.

### Multi-trigger chains are real OR-semantics

`loader/mod.rs:170-205`: N trigger rows → N in-memory `Chain`s sharing one id,
conditions and action list. Safe as long as no two trigger rows can match the
same event — if they can, the action list runs twice.
