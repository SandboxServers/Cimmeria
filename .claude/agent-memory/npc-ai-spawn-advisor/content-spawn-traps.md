---
name: content-spawn-traps
description: Traps for content-driven NPC spawn/despawn (H03 spawn_entity) — faction-0 auto-aggro dead zone, set_visible NPC routing drop, respawn-tick key, entity_templates missing is_stationary/aggression.
metadata:
  type: project
---

# Content-driven spawn traps (verified against main @ 3c1fed6c)

## 1. faction 0 is an auto-aggro dead zone

`npc_ai_idle_auto_aggro` (`cell/service/npc_ai/fight.rs:44`) filters
candidates with `if !p.is_player || p.faction == npc_faction { return None }`.

Players are **never assigned a faction** — `CellEntity::new` sets
`faction: 0` (`crates/entity/src/cell_entity/construction.rs:54`) and no
world-entry / connect path writes it. So an NPC with
`entity_templates.faction` NULL or 0 will **never** auto-aggro, no matter
how high its `aggression` is.

`HOSTILE_FACTION = 10` (`cell/combat/mod.rs:21`). Hostile templates must
carry a nonzero faction; several runtime paths force `faction = 10`
post-hoc as a workaround (`cell_methods/player/interaction/mod.rs:65,113`,
`ticks/auto_cycle.rs:226,437`).

**Why:** faction is the only discriminator the idle-aggro scan has.
**How to apply:** when authoring or porting a hostile template, assert
`faction != 0`. When a content spawn "doesn't aggro", check faction
before touching aggression.

## 2. `set_visible` on an NPC is dropped on the floor

`content/executor/world/mod.rs:335` sends
`CellToBaseMsg::EntityMethodCall { entity_id: target_id }`. Base's
`entity_method_call` (`base/world_entry/cell_dispatch/aoi.rs`) routes via
`entity_to_addr`, **which only holds player entries**, and hardcodes
`IDBASE_SGW_PLAYER`. An NPC target resolves to no address → nothing sends.

Correct primitive is `abilities::send_entity_method_to_witnesses`
(`cell/abilities/messaging.rs:98`) which emits `WitnessEntityMethod` per
witness and carries `entity_is_player` for idbase selection. Note
`abilities::send_entity_method` (line 39) already branches on
`is_player` and does the right thing — `set_visible` simply doesn't use it.

## 3. What the respawn tick actually keys on

`ticks/npc_respawn/mod.rs:105` — `ai_state == Dead && respawn_at <= now`.
Nothing else. `respawn_at` is stamped by `combat::state.rs:110` whenever
`entity.respawn_secs.is_some()`.

`spawn_id` is **not** consulted. `spawn_npc_from_record_into`
(`space_manager/spawn.rs`) maps `record.spawn_id > 0` → `Some`, else
`None` — the `-1` GM sentinel is only about not targeting a phantom
`spawnlist` row from authoring commands. It does not protect against
respawn. A non-DB NPC with `respawn_secs = Some(n)` **will** be revived.

## 4. `entity_templates` has no `is_stationary` and no `aggression` column

`is_stationary` lives only on `resources.spawnlist`; `aggression` lives
only on the runtime `CellEntity` (written by `set_aggression` /
`.aggression`). So "None = inherit the template" is meaningless for both —
a prototype `SpawnRecord` built from `entity_templates` alone must default
`is_stationary: false`, and aggression must be applied post-spawn.

DB CHECK: `entity_templates.respawn_secs >= 3`.
