---
name: spawn-timing-instanced-spaces
description: Spawn timing for instanced spaces (Castle_CellBlock etc.) — NPCs created at CreateEntity before ConnectEntity; spawnlist is the only spawn source.
metadata:
  type: project
---

# Instanced-space NPC spawn timing (confirmed)

`crates/services/src/cell/service/base_messages/mod.rs`:

- `BaseToCellMsg::CreateEntity` (instanced world) → `space_mgr.create_entity` →
  **immediately** `spawner::spawn_instance_npcs_from_records(...)` (mod.rs:73).
  So ALL spawnlist NPCs for that world are inserted into `space.entities`
  during instance creation, BEFORE any player ConnectEntity.
- `BaseToCellMsg::ConnectEntity` (mod.rs:124) → `connect_entity` + an inline
  `compute_aoi_changes_for_player(entity_id)` introduction loop (mod.rs:134).
  The comment explicitly names "the Castle_CellBlock stasis-room corpses" as
  the NPCs this inline introduction exists to surface — this is PR #525's
  fix (player-enters-occupied-space path).

Implication: every spawnlist NPC is **static / always-on** (path (a)). The
only "spawn after connect" path is the GM spawn handler
(`BaseToCellMsg::GmSpawnNpcReady`) — that's the entity-spawns-into-occupied-
space path #525 does NOT cover.

## Content chains NEVER create entities

Castle_CellBlock chains (`castle_cellblock_chains.sql`,
`space_castle_cellblock_chains.sql`) only: bind dialogs (add_dialog_set),
set_interaction_type, generate_threat, set_aggression, add/remove_item,
advance_step, accept/complete_mission, play_sequence, destroy_entity. There
is NO `spawn_entity`/`create_entity` content action. Mission "appearance" of
an NPC is done by binding a dialog set to an already-spawned corpse's
template_id, not by spawning it. Python parity: scripts use
`Atrea.findEntities` / `findEntityOnSpace` to locate pre-existing tagged
entities, never to create them.

## Castle_CellBlock = world_id 12, instanced (flags=1), space scope_id 8
