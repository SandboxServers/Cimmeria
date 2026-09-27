---
name: na38-client-orders-reliable-stream
description: SGW client Mercury receive ordering (Ghidra, NA38 2026-09-25) -- reliable stream is ordered by queueAckForPacket with a 512 window and adopt-first inSeqAt; unreliable delivered on arrival; EntityManager caches early position/method/property for unknown entity ids
metadata:
  type: project
---

Ghidra-verified in NA38 (2026-09-25). This is standard BigWorld
`addToReceiveWindow` semantics, and CME did not change them.

- `Nub::processFilteredPacket` tail `0x01580ad4`: reliable (`0x10`) on
  a channel → `UnAckedHandler::queueAckForPacket` `0x0158cba0`, and the
  returned chain is processed in order. Unreliable packets →
  `FUN_0158bb50` dedup (`+0x128`), then processed immediately. Flag bits
  seen in the decompile: `0x40` = pop seq, `0x80` = error path, `0x04` =
  acks. The draft spec §1.2 flag table is wrong about bits 5 to 7; the
  crate constants are right.
- `inSeqAt` is `ChannelInternal+0x50`, initialised to `0x10000000` by the
  ctor `0x0158c7b0` and adopted from the first reliable packet. The
  window at `+0x30` is copied from `Channel+0x2c = 0x200` (ctor
  `0x01576bf0`). The ACK is queued before the window checks, so the
  client acks even packets it drops as out of window.
- `EntityManager` handles messages for an unknown id without losing them.
  Move `0x00dd1650` → latest position into the map at `+0x30`; create
  `0x00dd2270` consumes it. Method `0x00dd2b80` and property `0x00dd29d0`
  → per-id buffer at `+0x3c` (replay not traced). `ServerMessageHandler`
  vtable at `0x019ce978`: create = slot `0x98c`, move = slot `0x99c`.
- Client's first reliable seq on a fresh channel = 0; server's
  connect_reply = seq 1, time-sync = seq 2 (castle_cellblock_head
  fixture).

**Why:** NA37 blamed one-way player visibility on a lost CREATE_ENTITY
letting the cascade arrive first. That was a harness artefact. The real
client cannot see that order.

**How to apply:** before calling any server→client ordering bug "real",
check whether the reliable stream plus the EntityManager buffers already
absorb it. Server-logic order across tasks (seq allocated out of logical
order) is the remaining class. Rust: `Channel::receive_parsed`
(`crates/mercury/src/channel/rx_order.rs`) models this gate for the
server and the harness. Related: [[aoi-entity-introduction]].

Server client channels adopt the first sequence and are **not** pinned to
0. The coordinator rejected pinning: a client starting anywhere else would
wedge its whole reliable stream. Stuck gaps are surfaced by
`Channel::check_rx_stall` (WARN after 2 s, counter
`mercury_rx_stalls_total`) and are never skipped, matching the client.
