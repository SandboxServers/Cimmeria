---
name: npc-follow-state-gaps
description: AiState::Follow is wired end-to-end but has three structural gaps - no player-as-target resolution, no post-combat resume, and a hardcoded move_speed slower than player run speed
metadata:
  type: project
---

`AiState::Follow` is fully implemented and dispatched
(`crates/services/src/cell/service/npc_ai/follow.rs`, dispatched from
`npc_ai/dispatch.rs:89`), and `set_follow_target` has a real executor arm
(`cell/content/executor/world/mod.rs:135`). Three gaps block using it for a
player escort:

1. **The player cannot be named as a follow target.** `set_follow_target`
   resolves `target_tag` through `SpaceManager::find_entity_by_tag`
   (`space_manager/queries.rs:325`), which scans `entity.tag`. `tag` is only
   populated from `spawnlist.tag` at NPC spawn — player entities always have
   `tag = None`. The existing convention to copy is `move_entity`'s
   `use_player: Option<bool>` param (`content-engine/src/loader/action.rs:372`).

2. **Follow never resumes after combat.** Threat preemption pushes
   Follow → Fighting; only Patrol and Wander auto-resume from per-state scratch.
   Worse, `npc_ai_leash` (`npc_ai/leash.rs:39-48`) **snaps the NPC back to
   `spawn_position`** and sets Idle, leaving `follow_target_id` set but inert.
   For an escorted NPC that means teleporting home mid-escort, permanently.
   Mitigating factor: `npc_ai_idle_auto_aggro` (`npc_ai/fight.rs:40-45`) only
   targets `is_player` entities, so hostile NPCs never aggro a friendly escort
   on their own — the path into Fighting is player AoE or an explicit
   `generate_threat`.

3. **`move_speed` is a hardcoded 0.6 units/tick = 6.0 u/s**
   (`crates/entity/src/cell_entity/construction.rs:86`). It is NOT a column on
   `entity_templates` and is never loaded from the DB. Player run speed in
   world 12 (Castle_CellBlock) is 8.125 u/s, so a follower is ~26% slower than
   a running player and can never re-enter the default `[2.0, 5.0]` follow band
   while the player runs — it just trails further and further behind.

`follow_min_distance` / `follow_max_distance` ARE real `entity_templates`
columns (default `[2.0, 5.0]` when NULL).

Seed usage as of 2026-09-17: `set_follow_target` 0 rows, `move_waypoint` 0 rows,
`move_entity` 5 rows in `sgc_w1_chains.sql` that are **dead** — `Action::MoveEntity`
has no executor arm. `move_waypoint` is the only working teleport-an-NPC
primitive (`executor/world/mod.rs:330`, calls `update_entity_position` +
`note_authorized_teleport`).

Related: [[castle-cellblock-navmesh-components]]
