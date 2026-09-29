---
name: project-838-fragment-header-straddle
description: "#838 root cause candidate (PR #1087): fragmenter cut bodies raw every 1300 B so a message header could straddle fragments; client drops the rest of the bundle. Where the guard and tests live."
metadata:
  type: project
---

The SGW client requires each message header (id + u16 length = 3 bytes for WORD_LENGTH) inside ONE packet; a straddling header aborts the rest of the bundle silently after all fragments were ACKed (`Bundle::iterator::unpack` @ 0x01579830, `processOrderedPacket` @ 0x0157c820). Bodies may straddle. Our fragmenter split raw at 1300 B. Fix: `crates/mercury/src/packet/fragmenting.rs` `plan_fragments` (used by `build_fragmented_bundle`, `ChannelBundle::estimated_packet_count`, `fragment_count`).

**Why:** any AoI/witness fanout that sends a bundle over one packet is exposed; the 24-NPC world-entry phase-2 cascade is 15 fragments so it hit ~1 login in 4. Not confirmed live as of 2026-09-29 (waits for client telemetry).

**How to apply:** when reviewing a new large-bundle send, confirm it goes through `build_fragmented_bundle` (never hand-chunk), and test it with `cimmeria_mercury::client_model` (`unpack_reliable_stream`). Client keeps one fragment group per channel keyed by lastFrag, so a group's seqs must stay contiguous (the atomic `fetch_add` reservation in `send_bundle_to_witness_reliable` guarantees it). TX window is 32: two ~16-fragment groups back to back overflow it (queued, not lost).
