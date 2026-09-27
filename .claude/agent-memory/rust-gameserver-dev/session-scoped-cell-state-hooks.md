---
name: session-scoped-cell-state-hooks
description: Where to hook cell state that must survive gate travel but die with the session (squads, ORG-03) - DisconnectEntity vs DestroyEntity, InitPlayerState as the per-world-entry replay point, key by player_id
metadata:
  type: project
---

Cell state that belongs to the *session* (not the entity) must be keyed by `player_id` and torn down only in the `DisconnectEntity` arm (`crates/cell/src/cell/service/base_messages/lifecycle.rs`), never in `DestroyEntity` or `SpaceManager::destroy_entity`: `DestroyEntity` is also the gate-travel teardown, and gate travel re-creates the cell entity (same id, but `character_name` and any stamped fields are gone until `InitPlayerState`). The base sends `DisconnectEntity` from `destroy_client_entities` (logoff, crash, timeout, duplicate login), `logOff`, and an abandoned gate transfer.

`InitPlayerState` reaches the cell on every world entry, gate arrivals included, so it is the replay point for anything the client lost to RESET_ENTITIES (ORG-03 re-sends [35]/[38]/[37]/[51] there, after `handle_init_player_state` stamps `player_id`).

Resolve a member's live entity per event with `SpaceManager::player_entity_by_player_id` (scans `space.players`, so an in-transit player is `None`); queue anything owed to an in-transit player for their next `InitPlayerState` rather than dropping it.

**`DisconnectEntity` can arrive for an entity that no longer exists** (member in gate transit: aborted transfer, or crash/timeout mid-transfer), so `get_entity(id).player_id` is `None` there and a `player_id`-keyed teardown silently skips them. The base cannot help on the abort path (`abandon_unspaced_session` fires only when it has no `active_player_id`). Keep the last live entity id per session in your registry and fall back to it (ORG-03 `SquadRegistry::note_entity` / `member_by_entity`). A full-exit `logOff` also sends a second `DisconnectEntity` for an already-gone entity, so the plain miss is DEBUG, not WARN.

**Why:** ORG-03 (2026-09-27) needed squads to survive world changes; hooking `DestroyEntity` would dissolve every squad on the first gate trip (PR #584's per-space manager had the same bug, audit A-30). `destroy_entity_keeps_the_squad` in `crates/cell/src/cell/service/base_messages/tests/org.rs` pins it.

**How to apply:** any new per-session cell registry (ORG-04 squad chat state, pending creations, trade-like handshakes) follows the same three hooks. Related: [[cross-world-transfer-flow]], [[ring-transport-fsm]].
