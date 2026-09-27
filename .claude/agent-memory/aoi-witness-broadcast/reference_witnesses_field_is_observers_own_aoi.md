---
name: reference-witnesses-field-is-observers-own-aoi
description: CellEntity.witnesses stores what THAT entity sees (its own AoI, players+NPCs), not who sees it — a recurring source-vs-target confusion bug
metadata:
  type: reference
---

`CellEntity::witnesses: HashSet<EntityId>` (only ever populated for players by
`SpaceManager::compute_aoi_changes`) holds **what that player currently
sees** — every entity, player or NPC, inside that player's own AoI radius.
It is NOT "who witnesses/observes this entity." The naming is a standing
trap; the doc comment on `SpaceManager::get_witnesses_of`
(`crates/services/src/cell/space_manager/queries.rs:440-450`) spells out the
correct reverse-mapping algorithm: scan `space.players`, keep only those
whose `.witnesses` set contains the target.

Every correct production call site does one of:
1. Call `space_mgr.get_witnesses_of(target_id)` (scans `space.players`,
   filters by inbound membership) — the vetted way to get "who observes
   target_id".
2. Directly scan `space.players` and filter by `.witnesses.contains(&target)`
   inline (disconnect_entity / despawn_npc in
   `cell/space_manager/entities.rs`).
3. Use `entity.witnesses` as a pure membership check ("is X in the
   requester's own AoI"), e.g. `RequestEntityUpdate`
   (`cell/service/base_messages/request_entity_update.rs:75`) — reading the
   set, not iterating it as send targets.

**The bug shape**: iterating `entity.witnesses` directly and treating each
entry as a recipient/witness_id. Since the set includes NPCs the entity
can see, every NPC in range becomes a bogus send target. NPCs have no
`entity_to_addr` entry (that map is player-only), so `send_to_witness*`
logs `AoI reliable: no client addr for witness -- packet dropped`
(`reason = entity_to_addr_miss`) once per NPC per event — with
`witness_id` in the log being the **NPC's** id, not a player's. This was
the root cause of a 2026-09-26 colo WARN storm (14 hits, all `witness_id`s
NPC ids in Castle space 65537, `entity_count_in_map` = 1-2 = the real
connected-player count).

**Found and fixed** in `crates/services/src/cell/chat.rs`'s
`broadcast_to_witnesses` (spatial chat fan-out for say/emote/yell): it did
`entity.witnesses.iter().map(...)` directly with no `is_player` filter, then
sent one `CellToBaseMsg::EntityMethodCall { entity_id: witness_id, .. }` per
entry. Fix: filter to `space_mgr.get_entity(wid).is_some_and(|e| e.is_player)`
before treating an id as a send target. Regression test:
`broadcast_say_skips_npc_witnesses` in the same file (an NPC placed in the
sender's witness set alongside a real player witness; asserts the NPC id
never appears as an `entity_id` in the emitted messages).

**Audit note**: as of 2026-09-26, every other production site was already
correct — `get_witnesses_of`, the inline `space.players` scans, and the
membership-check-only site in `request_entity_update.rs` don't have this
bug. `chat.rs` was the only offender. If a new "broadcast to observers"
site is added, grep for `.witnesses.iter()` reachable from an entity (not
`space.players`) and check for the missing `is_player` filter before it
lands.
