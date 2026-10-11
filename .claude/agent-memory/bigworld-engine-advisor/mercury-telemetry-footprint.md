---
name: mercury-telemetry-footprint
description: Where Mercury telemetry volume and blind spots are (2026-10-10 telemetry-gaps review); old_duplicate join keys; AUTHENTICATE per bundle
metadata:
  type: reference
---

Measured on the colo SigNoz, 7 days to 2026-10-10 (review: docs/analysis/telemetry-gaps/reviews/engine-mercury.md):

- `mercury.packet` INFO (crates/mercury/src/transport.rs, one row per datagram) and `cimmeria_mercury::encryption` TRACE `encrypt`/`decrypt` make up about 126M of the 147M log rows (86%). They carry only peer and length.
- An NPC-dense zone sends one unbundled unreliable avatar-update datagram per (witness, NPC) per tick (cell_dispatch/aoi.rs `entity_moved`). One idle client received 880 datagrams/s for 10 hours on 2026-10-06.
- The client prepends BaseAppExt `authenticate` (0x01) to every bundle it sends, which is standard BigWorld behaviour. The server ignores it, so its DEBUG row fires once per packet.
- Client `old_duplicate` means the server resent a packet the client had already delivered. Client rows carry no account or peer, and the server `mercury.retransmit` row has no account or fingerprint, so the two can't be joined. `wire_fingerprint` is the intended join key: it is on the server's `mercury.reliable_send` and the client's `socket_recv` only. The `old_duplicate`s for seq 1-2 are benign: the 700 ms initial RTO resends the handshake.
- `Channel::is_timed_out` (MAX_RETRIES 20) has no production caller. The 60 s tick_sync inactivity reap ends dead channels instead.
- Panics reach stderr and Discord only, never SigNoz.

Proposed fixes are packets TG-MER-01..12 [[plugin-architecture-compat-boundary]].
