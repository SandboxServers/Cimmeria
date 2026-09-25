---
name: hostility-and-stationary-gates
description: faction==10 (HOSTILE_FACTION) is the ONLY gate that makes an NPC damageable/attackable; aggression has no DB column (always 0 at spawn); is_stationary is read only in fight.rs
metadata:
  type: project
---

> **Stale since NA13 (2026-09-25):** the `aggression` field and "faction 10 alone never aggroes" claims below are superseded; see [[faction-derived-aggro-na13]].

# Two gates that silently make NPC config inert (verified 2026-09-17)

## `HOSTILE_FACTION = 10` is the whole hostility model

`crates/services/src/cell/combat/mod.rs:21`. Three consumers:

- `abilities/use_ability/handle.rs:224` — a harmful ability is **refused**
  when `target.is_player || target.faction != HOSTILE_FACTION`.
- `cell_methods/player/interaction/interact.rs:45` — right-click only
  reroutes to combat for `faction == 10 && alive`.
- `abilities/cone_aoe/geometry.rs:87`, `abilities/dispatch.rs:124` — AoE
  target collection skips non-10.

**Consequence:** an NPC whose template `faction != 10` cannot be damaged,
cannot die, and therefore never uses `respawn_secs` / `loot_table_id` /
death chains. Most seeded templates ship `faction = 1` (Harset 43/159/160/164
all do; 163 Petbe is NULL → `unwrap_or(0)`), so they are decorative today.
Ordering rule: a packet that flips a template to faction 10 must land
**with or after** the packet that sets `respawn_secs`, or it re-creates the
one-shot-NPC bug (H-B7).

## `aggression` is not a DB column

Defaults to `0` (`cell_entity/construction.rs:88`); the only writers are the
`set_aggression` content action and `.aggression` console command. The
Idle→Fighting auto-aggro branch (`npc_ai/fight.rs:13-61`, gated in
`dispatch.rs:59`) therefore never fires for a seeded NPC. Seeded mobs only
enter Fighting via player-generated threat. So respawn timers cannot cause
"re-aggro on the way back" — a respawned mob is passive.

## `is_stationary` is a fight-only flag

Read in exactly three places, all `npc_ai/fight.rs`: cover reservation gate
(:256), chase-pathfinding skip + `stationary_holds` log (:343), min-range
backup skip (:436). It does **not** affect facing, aggro, threat,
interaction/dialog, loot, leash, respawn — and notably **does not suppress
patrol/wander/follow/investigate**, which never read it. Setting it is cheap
insurance in a zone with a broken/absent navmesh (converts the silent
`no_path` freeze into an explicit hold), but it is not a general "do not
move" flag; `COALESCE(s.patrol_path_id, t.patrol_path_id)` can still make a
"stationary" NPC walk.

Related: [[harset-zone-evidence]], [[npc-follow-state-gaps]]
