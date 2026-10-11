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
| Entities the client holds at once | not measured | n/a | n/a | not measured | not measured | Chaos profile `burst`. CL-03: any fixed-size entity table; CL-09: the count at which creates fail. A lab session ran 161 extra NPCs (Debug Area lineup) without anyone looking for the ceiling. |
| Creates in one flush the client absorbs | not measured | n/a | n/a | not measured | not measured | Chaos profiles `burst` and `flush`. CL-09 finds the ceiling. Seen so far: 24 creates of 37 bytes each in one 889-byte packet, the packet #1341 hits; 28-NPC instances flush in about 11 packets (`crates/base-world-entry/src/base/world_entry/cell_dispatch/deferred_flush.rs`). |
| Deferred buffer | 512 messages, then drop with a WARN | config | `crates/base-session/src/base/deferred_aoi.rs` `MAX_DEFERRED_AOI_MSGS` | n/a | n/a | CL-05 checks whether a dense map reaches it. |
| First-login hold | 16 s | config | `crates/base-world-entry/src/base/world_entry_appearance/cinematic_aoi_hold/mod.rs` `HOLD_DURATION` | n/a | n/a | The appearance resend runs every 100 ms for 20 s (`cinematic.rs`). |
| Entities per `requestEntityUpdate` | 64 | config | `crates/cell/src/cell/service/base_messages/request_entity_update.rs` `MAX_REQUEST_ENTITIES` | n/a | n/a | A server cap. CL-09: what the client sends after a burst. |
