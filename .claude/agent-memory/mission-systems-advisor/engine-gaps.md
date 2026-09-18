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
