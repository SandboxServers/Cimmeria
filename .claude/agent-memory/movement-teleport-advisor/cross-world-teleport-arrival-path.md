---
name: cross-world-teleport-arrival-path
description: What Action::CrossWorldTeleport actually does end to end — destroy + GateTravel + full world entry, zero destination-side position validation, and the two ways it can strand a player
metadata:
  type: project
---

Traced 2026-09-17. `cross_world_teleport` is **not** a forced-position path and must not be reviewed as one — the destination position rides the world-entry sequence, so `build_forced_position` never enters the picture. Forced position stays the intra-space primitive (`Action::Teleport`, `executor/transport.rs:34-92`, which does `update_position_preserving_facing` → `note_authorized_teleport` → `CellToBaseMsg::TeleportPlayer`).

**The sequence** (`crates/services/src/cell/content/executor/transport.rs:110-154`):

1. flush dirty bandolier ammo
2. `space_mgr.destroy_entity(entity_id)` — which also calls `movement_validator.forget(entity_id)` (`space_manager/entities.rs:152`), so there is no stale validator clock at the far end and no `note_authorized_teleport` is needed
3. `CellToBaseMsg::GateTravel { destination_ring_id: None, destination_space_id: None }`
4. base `handle_gate_travel` (`base/world_entry/gate_travel/mod.rs:105+`) → RESET_ENTITIES → `BaseToCellMsg::CreateEntity` → full world-entry + mapLoaded
5. cell `handle_create_entity` (`cell/service/base_messages/lifecycle.rs:72-75`) → `create_entity(world_name, position, rotation)`

**There is no navmesh, bounds or walkability check anywhere on that path.** `insert_entity_into_space` (`space_manager/entities.rs:73-81`) writes the position straight in. Validation only begins with the first inbound client position packet, and that one reads bounds from the destination's navmesh AABB or `SpaceBounds::FALLBACK` when there is none (`space_manager/client_move.rs:170-178`), with `is_position_valid` returning `true` unconditionally on a navmesh-less space (`space_manager/spatial.rs:53-67`). So on a world with no `.nav` file the arrival is unvalidated **and** unrejectable — no identity fallback, no `CorrectionSuppressed`, no respawner redirect. The off-navmesh strand in [[arrival-coordinate-offnavmesh]] can only fire on a navmesh-backed destination.

**Two real ways this strands a player, both before arrival:**

- **No startup space.** `find_or_create_space` (`space_manager/lifecycle.rs:74-99`) returns the cached `world_spaces` entry for non-instanced worlds and **errors** if there isn't one: *"Non-instanced world X has no startup space — it should be listed in cell_spaces.xml"*. The origin entity is already destroyed by then. Before blessing any cross-world destination, check the world has a `<Space WorldName="…"/>` row in `entities/cell_spaces.xml` (that file, not `entities/spaces.xml` — the latter only carries `Instanced` and MinX/MaxX, and those bounds are **not** what the movement validator uses).
- **Exact-match world names.** `target_key` is compared verbatim all the way down; see the capital-H trap in [[snap-back-termination]].

Also note `worlds.flags` is **not** the instanced bit — `is_world_instanced` reads `WorldDef.instanced`, sourced from `entities/spaces.xml`. Harset_CmdCenter is `flags = 1` in `worlds.sql` and `Instanced="false"` in spaces.xml; the flags column misleads on sight.
