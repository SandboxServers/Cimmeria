---
name: leash-is-a-bare-field-write
description: npc_ai_leash sets npc.position directly, bypassing write_position — AoI spatial grid desyncs, stale velocity makes the client extrapolate the NPC off-geometry, and facing is never restored
metadata:
  type: reference
---

> **Status 2026-09-19 — still true.** Re-verified on `main`: `npc_ai/leash.rs:49` is still
> `npc.position = spawn_pos;` with no `write_position`, no velocity reset and no `target:` on the log.

**Block on sight.** `crates/services/src/cell/service/npc_ai/leash.rs:48-50`:

```rust
if let (None, Some(spawn_pos)) = (npc.follow_target_id, npc.spawn_position) {
    npc.position = spawn_pos;
}
```

A direct field write that bypasses `SpaceManager::write_position` entirely. Four consequences:

1. **AoI spatial grid desync.** `write_position` calls `space.grid.update_position(...)`
   (`cell/space_manager/entities.rs:462-466`); this does not. The grid keeps indexing the NPC in its
   old cell until something else moves it. This is both a position-write authority violation and an
   AoI correctness bug (→ aoi-witness-broadcast).
2. **`velocity` is never zeroed.** The NPC keeps its last chase velocity and the AoI tick keeps
   broadcasting it. `USGWAvatarFilter::Output` does `position = lastPosition + velocity * dt`, so the
   client **extrapolates the leashed NPC along its old chase velocity** — it drifts off the geometry.
3. **`direction` is never touched**, and per [[npcs-cannot-turn-in-place]] nothing else can re-face a
   path-less NPC, so it keeps its chase yaw (often the `pack_angle` north-snap) permanently.
4. **It is a teleport, not a walk** — the doc comment at `:10-11` admits it ("In a full
   implementation this would pathfind the NPC back to spawn"). What players perceive as "walking
   home" is the client's AvatarFilter lerping across the snap gap while extrapolating on stale
   velocity — which is exactly why the walk home ignores geometry and faces wrong.

**Minimum fix:** route through `update_position_preserving_facing` (or a variant that also sets the
spawn heading) with `velocity = [0.0; 3]`. Fixes grid desync, drift and facing without building the
full walk-back.

**It is also effectively unlogged.** `leash.rs:67` is a bare `tracing::info!` with **no `target:`**,
so it lands on the module path rather than `npc_ai` / `movement.npc`, and carries no position, no
spawn position, no distance and no reason. The 2026-09-18 session produced zero leash rows and we
could not tell whether leash never fired or fired unqueryably.

Note the follower exception at `:48`: an NPC with `follow_target_id` set is deliberately **not**
snapped (GC1b-0 hardening — a yanked escort would be stranded). So escorts and mobs take different
leash paths; check which one a report is about.

See [[authorized-teleport-paths]] — every path that mutates position must go through the writer.
