---
name: stored-target-lifetime-and-gm-view-check
description: #844 clears CellEntity::current_target_id on AoI leave/destroy/respawn/kill; GM target resolution requires the target in the caller's witness set, so console tests must insert it; killer clear must follow the auto-cycle sweep
metadata:
  type: project
---

Since #844 (2026-09-28) `current_target_id` has a lifetime, all in
`crates/cell-world/src/cell/space_manager/target_lifetime.rs`:

- AoI leave: `compute_player_aoi` drops it on the previous-in/current-out
  transition only (a self-target is never in the witness set and must
  survive).
- Destroy: `destroy_entity` calls `clear_targets_on` (every teardown,
  including `despawn_npc` and disconnect, runs through it).
- Respawn: `npc_respawn` clears and sends `onTargetUpdate(0)`.
- Kill: death burst clears the killer only. **It must run after
  `clear_auto_cycle_for_target`**, which finds the killer's loop by matching
  `current_target_id`; clearing first leaves `BSF_AUTO_CYCLING` armed.

**GM target resolution** (`console/dispatch.rs::resolve_target`,
`gm/query.rs::subject_or_self`) now needs `SpaceManager::target_in_view`
(self, or in the caller's `witnesses`). A console test that sets
`current_target_id = Some(x)` must also `witnesses.insert(EntityId(x))`, or
typed commands refuse with "not in view". Tests that count witnesses of the
target (despawn notifications, aggression fan-out) then see the GM as one
more witness.

**Why:** a 19-minute-old stale target sent a `.summon` 216 m away (colo).

**How to apply:** any new reader of `current_target_id` that acts on it
should also check `target_in_view`; any new sweep keyed on it must run before
a clear in the same burst. Related: [[npc-class-filter-and-dead-target-traps]].
