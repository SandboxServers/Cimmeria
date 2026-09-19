---
name: reference-despawn-npc-vs-destroy-entity
description: SpaceManager::despawn_npc vs destroy_entity semantics, death-state bookkeeping ownership, and the looting_entity cleanup gap
metadata:
  type: reference
---

File: `crates/services/src/cell/space_manager/entities.rs`

## `destroy_entity(entity_id)` (sync, no tx)

Removes from `space.entities` + spatial grid, drops GM-session buffers
(`authoring_changes`, `autosave_spawns`, `pending_content_actions`), forgets the
movement-validator clock. **Does not touch other entities' `witnesses` sets** —
they self-heal on the next periodic AoI tick's `LeftAoI` diff (up to ~100ms
lag), or never heal if the observer has already left the space.

## `despawn_npc(entity_id, tx) -> DespawnOutcome` (async)

The immediate-fanout version: sends `LeftAoI` to every current witness first,
scrubs the dead id out of **every** entity's witness set in the same pass (so
the next tick doesn't double-emit), then calls `destroy_entity`. Refuses
players structurally (`DespawnOutcome::RefusedPlayer`) — checks both
`entity.is_player` and `space.players.contains`. GM `.despawn`
(`console/spawn/mod.rs::despawn_entity`) is the only current caller; it awaits
this and reports the exact outcome to the GM.

`DespawnOutcome` is `#[must_use]` — every caller must report/log the variant,
not discard it.

## Death/respawn state is entity-local — no external bookkeeping table

`mark_npc_dead` (`combat/state.rs`) and `npc_respawn_tick`
(`service/ticks/npc_respawn/mod.rs`) mutate fields directly on the `CellEntity`
struct: `ai_state`, `state_field` (BSF_DEAD/BSF_MOVEMENT_LOCK), `respawn_at`,
`respawn_secs`, `interaction_type_flags` / `original_interaction_type_flags`,
`loot` list + `next_loot_index`, `spawn_position`/`spawn_direction` (snap-back
target). There is **no separate loot-bag entity and no spawn-region occupancy
table** keyed by entity id — grepped, none found. So `destroy_entity` /
`despawn_npc` removing the `CellEntity` wholesale cleans up all of this
atomically; nothing external leaks.

## Confirmed gap: `looting_entity` is NOT cleared by despawn

`npc_respawn_tick` step 5 explicitly closes any player's open loot window
(`player.looting_entity == this NPC` → empty `onLootDisplay` + clear the
field) because a corpse respawning out from under an open loot window would
leave stale UI + a dangling reference. **Neither `destroy_entity` nor
`despawn_npc` does this.** If a player has a despawned NPC's loot window open
(`looting_entity == Some(despawned_id)`), the field goes stale and no close
packet fires — the client loot window doesn't auto-close, and a subsequent
take-item call references a now-nonexistent entity id. Relevant for corpse
despawn (H27 "hide the body", H45 "corpse stays for instance life" — Harset
work packets) since those are exactly loot-window-eligible corpses. This is a
real pre-existing gap in `despawn_npc`, not something the content-executor
caller can paper over — fix belongs in `despawn_npc` itself (mirror
`npc_respawn_tick`'s step 5) if corpse despawn ships.

## Content-tag despawn can never hit `RefusedPlayer`

Player entities carry no `tag` (tags only come from `spawnlist.tag` at NPC
spawn — see `spawner/npcs.rs` and the doc comment on
`executor/world/mod.rs::set_follow_target`). `find_entity_by_tag` structurally
cannot resolve a player, so a tag-based content despawn action can never reach
`despawn_npc`'s `RefusedPlayer` guard. Still worth logging if hit (defense in
depth against a future invariant break), just not a path that needs product
design.
