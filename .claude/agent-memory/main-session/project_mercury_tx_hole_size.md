---
name: project-mercury-tx-hole-size
description: "A September 2026 transmit hole involved 1488-byte encrypted UDP payloads; the exact client receive or network drop point remains unproven. Read before changing packet size or retransmit policy."
metadata:
  type: project
---

**Observation (SigNoz, 2026-09-29).** A `mercury.tx_hole` opened for reliable sequence 1949 after the peer acked 1950. It first warned at 22:59:37.747 UTC and remained open through 23:02:38.312 UTC, with 363 retransmits. At the end, sequences 1949, 1953, 1955 and 1958 were outstanding, each with 1488 encrypted UDP bytes. Later ACKs reached at least 2082. Client telemetry had `REASON_GENERAL_NETWORK` around the onset and an earlier old-duplicate receive, but no `client.mercury.packet_in` record for 1949; that absence did not prove a socket drop because ordinary packet events were throttled and the old hook ran after the socket boundary. The 1488 bytes exceed `PACKET_MAX_SIZE = 1472`, so datagram size is a concrete hypothesis, not an established cause.

**Static client finding (Ghidra `SGW.exe`, 2026-09-29).** The game Mercury socket reader `FUN_0158a200` calls `recvfrom` with a **0x5c0 (1472-byte) buffer** at `Packet+0x54`, and sets `Packet+0x24` length only on nonnegative return. A 1488-byte UDP payload should therefore return Winsock `WSAEMSGSIZE` (10040) and never reach `processFilteredPacket`. The observed loss boundary is still unproven until a live client reports that error. This source finding is documented in `docs/reverse-engineering/findings/client-mercury-receive-path.md`.

**Follow-up (2026-09-29).** Merged PR #1118 added vitals, vendor, Lua and sequence telemetry but no Mercury send/receive correlation. This work adds `mercury.reliable_send` and `tx_hole_stall` wire size, fingerprint and dispatch site, a client Winsock `recvfrom` event at the IAT slot identified through Ghidra's `recvfrom` thunk (`0x012f3d8c` jumps through `0x017eff60`), and client receive-gap lifecycle from `queueAckForPacket`. The fingerprint is FNV-1a over exact datagram bytes; it is diagnostic, not cryptographic. A repeat session is needed to prove the live socket outcome. See `docs/architecture/client-telemetry.md` and `docs/protocol/mercury-wire-format.md` for the event fields.

**Fix (2026-10-03).** The oversize came from piggybacked ACKs, the one unbounded part of a packet: every send drained the whole pending list (a 1300-byte bundle fragment plus 40 ACKs encrypts to 1488 or more). Sends now take ACKs through `cimmeria_mercury::packet::take_piggyback_acks`: 10 under v1, 2 under v2, 255 on tickSync, with the rest left pending. If `mercury.reliable_send` still warns about an oversize datagram, look for a body that is too large, not for ACKs.
