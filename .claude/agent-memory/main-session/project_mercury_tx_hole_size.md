---
name: project-mercury-tx-hole-size
description: "Diagnostic method for a stuck reliable stream (mercury.tx_hole): oversized datagrams over 1472 B wedge the client; how to confirm and where the rules live."
metadata:
  type: reference
---

Method for a `mercury.tx_hole` that stays open while later ACKs arrive (retransmits climb, the same sequences stay outstanding):

1. Read the stalled sequences' `wire_len` and `send_kind` on `mercury.reliable_send` and `tx_hole_stall`. Anything over `PACKET_MAX_SIZE = 1472` bytes is the suspect: the client's Mercury socket reader (`FUN_0158a200` in `SGW.exe`) calls `recvfrom` with a 0x5c0 (1472-byte) buffer, so a larger UDP payload should fail with Winsock `WSAEMSGSIZE` (10040) and never reach `processFilteredPacket`. Finding: `docs/reverse-engineering/findings/client-mercury-receive-path.md`. The live socket outcome was never proven; look for it in `client.mercury.socket_recv`.
2. Absence of a `client.mercury.packet_in` record for the sequence is not proof of a socket drop: ordinary packet events are throttled and the old hook ran after the socket boundary.
3. The datagram fingerprint is FNV-1a over the exact bytes; it is diagnostic, not cryptographic.

Two causes seen so far, both about size: uncapped piggybacked ACKs (budget rule: [mercury-wire-format.md § Piggybacked ACK budget](../../../docs/protocol/mercury-wire-format.md#piggybacked-ack-budget)), and single-packet sends with data-sized bodies such as the NPC `createOnClient` cascade (a Debug Area lineup spawn reached 1504 bytes with zero ACKs). Rule and the audited send sites: [mercury-wire-format.md § Reliable datagram size budget](../../../docs/protocol/mercury-wire-format.md#reliable-datagram-size-budget). Single-packet sends measure the packet before reserving a sequence number and fragment an oversize body through `build_fragmented_bundle` (`crates/mercury/src/packet/build.rs`, `crates/base-session/src/base/helpers/reliable_fit.rs`). A dense-NPC arrival should log `event = "oversize_fragmented"` and no `tx_hole_stall`. Event fields: `docs/architecture/client-telemetry.md`.