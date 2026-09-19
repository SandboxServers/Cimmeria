---
name: reference-periodic-aoi-tick-covers-midsession-spawn
description: The unconditional 100ms AoI tick already introduces a mid-session-spawned NPC to already-connected witnesses — no explicit push needed, confirmed via GmSpawnNpcReady's own comment and message_loop.rs cadence
metadata:
  type: reference
---

Question: when an NPC is inserted into `space.entities` mid-session (a space
that already has connected players), does it need an explicit AoI
introduction, or does the periodic tick pick it up?

**Answer: the periodic tick picks it up, unconditionally, every ~100ms.** No
explicit push needed for the "spawn into an already-occupied space" case.

Evidence:

- `crates/services/src/cell/service/message_loop.rs:54-66` — `run_aoi_tick`
  fires on **every** 100ms `tick_interval` tick, unconditionally (unlike NPC
  AI at every-20th and respawn at every-10th, which gate on
  `aoi_tick_counter.is_multiple_of(N)`).
- `compute_player_aoi` (`space_manager/aoi.rs`) recomputes `current_aoi` fresh
  from the spatial grid every call, for every player already in
  `space.players`. A newly-inserted NPC that's in the grid (`spawn_npc` /
  `spawn_npc_from_record_in_space` both call `space.space.add_entity(...)`
  before returning) is picked up on the very next tick — `EnteredAoI` fires
  because it's absent from that player's persisted `witnesses` set.
- `service/base_messages/gm_spawn.rs::handle_gm_spawn_npc_ready` — the
  existing "spawn NPC while players are already connected" path (GM
  `.spawn`/`.spawnrandom`/native `gmSpawnByCmd`) — has this comment verbatim:
  *"AoI fanout handles client visibility on the next tick, so there's no
  extra send here (same as DB-seeded NPC spawns)."* It does nothing beyond
  `spawn_npc_from_record_in_space` + GM feedback. This is the established,
  working pattern.

**Do not confuse this with #582 / the connect-time race** (see
npc-ai-spawn-advisor's `spawn-timing-instanced-spaces.md`). That bug was
specifically about a player **not yet in `space.players` at all** —
`compute_aoi_changes`'s `space.players.is_empty()` guard skips the whole space
until the player connects, and the fix was the one-shot
`compute_aoi_changes_for_player(connecting_player_id)` call on
`ConnectEntity`. That mechanism introduces entities *to the connecting
player*; it is orthogonal to introducing a *new* entity to players who are
*already* connected — the latter is the unconditional periodic tick's job and
already works.

So: a content `spawn_entity` action that inserts via `spawn_npc` /
`spawn_npc_from_record_in_space` (both grid + entities map insert) needs no
extra AoI call. Worst case latency to appear for already-connected witnesses
is one tick (~100ms), not "until relog."
