---
title: "Client limits: AoI and entities"
type: reference
audience: engineers, the lab-chaos campaign
last_updated: 2026-10-11
---

# Client limits: AoI and entities

How much of the world the server introduces to the SGW client and how fast: the AoI radii, the entity count the client holds, create bursts, the deferred buffer and the first-login hold. Grades, column rules and the summary are in the [limits index](README.md).

| Bound | Value | Grade | Source | Safe level | Break level | Notes |
|---|---|---|---|---|---|---|
| Player AoI radius | 150 m | config | `crates/entity/src/cell_entity/witness_aoi.rs` `PLAYER_AOI_RADIUS` | n/a | n/a | CL-03 confirms the client has no radius of its own. |
| NPC default AoI radius | 100 m | config | `crates/entity/src/cell_entity/construction.rs`, the `aoi_radius` field initializer (literal, no named constant) | n/a | n/a | |
| AoI leave margin | 25 m | config | `crates/entity/src/cell_entity/witness_aoi.rs` `AOI_LEAVE_MARGIN` | n/a | n/a | |
| Entities the client holds at once | no fixed limit | static | `0x00dd1d00` / `0x00c6c500` (growable `std::map`); no count check. See [CL-03](../../reverse-engineering/findings/client-entity-capacity.md) | not measured | not measured | Chaos profile `burst`. Bounded by address space (4 GB, see [process.md](process.md)); the MSVC "too long" guard is `max_size()`, far above that. CL-09 finds the live break level; a lab session ran 161 extra NPCs (Debug Area lineup) without anyone looking for the ceiling. Creates need an `enterAoI` first or the entity parks ([client-entity-lifecycle.md](../../reverse-engineering/findings/client-entity-lifecycle.md)). In stock BigWorld the effective cap is the server's per-client bandwidth budget (`bitsPerSecondToClient`, priority-queued witness updates), the server-side lever (engine knowledge, not this binary). |
| Actors the client spawns at once | no limit in `SpawnActor` | static | `0x00876970`; spawn wrapper `0x00e994b0`; no actor-count string in `.rdata` | n/a | n/a | Collision-test refusal only when a placement check applies. |
| Client entity-lifecycle cull | none | static | strings `Entity Enter AoI` `0x019cded0` and `[Entity not in client AOI]` `0x019b9d68` are logs only; `enterAoI` `0x00dd24f0` | n/a | n/a | The client creates what the server introduces and removes what it withdraws: no radius, distance test or lifecycle cull. UE3 render culling (cull distance volumes, maximum draw distance, LOD) was not examined. |
| Entity create for an id already in the world | assertion | static | `entity_manager.cpp` line `0x1e0` in `0x00dd2270` | n/a | n/a | A duplicate create is an error, not an overwrite. |
| Creates in one flush the client absorbs | not measured | n/a | n/a | not measured | not measured | Chaos profiles `burst` and `flush`. CL-09 finds the ceiling. Seen so far: up to 28 NPCs in one first-login flush (SigNoz server `create_emit` `entered`, 2026-10-11, 30 days; flush sizes 24 NPCs 148 times, 3 NPCs 15, 10 NPCs 9, 21 NPCs 3, 18 and 28 NPCs once each), all absorbed. 24 creates of 37 bytes each make the 889-byte packet #1341 hits; 28-NPC instances flush in about 11 packets (`crates/base-world-entry/src/base/world_entry/cell_dispatch/deferred_flush.rs`). |
| Deferred buffer | 512 messages, then drop with a WARN | config | `crates/base-session/src/base/deferred_aoi.rs` `MAX_DEFERRED_AOI_MSGS` | n/a | n/a | Never reached in 30 days: no buffer-full WARN; 64 `deferred_flushed` events, 2026-10-04 to 2026-10-11 (SigNoz, 2026-10-11). |
| First-login hold | 16 s | config | `crates/base-world-entry/src/base/world_entry_appearance/cinematic_aoi_hold/mod.rs` `HOLD_DURATION` | n/a | n/a | The appearance resend runs every 100 ms for 20 s (`cinematic.rs`). |
| Entities per `requestEntityUpdate` | 64 | config | `crates/cell/src/cell/service/base_messages/request_entity_update.rs` `MAX_REQUEST_ENTITIES` | n/a | n/a | A server cap. CL-09: what the client sends after a burst. |
