---
title: "Client limits: network"
type: reference
audience: engineers, the lab-chaos campaign
last_updated: 2026-10-11
---

# Client limits: network

The Mercury bounds between the server and the SGW client: datagram and fragment sizes, the reliable windows, resend timing, what the client sends while idle, and the #1341 next-request residue. Grades, column rules and the summary are in the [limits index](README.md); the evidence for the static rows is [client-mercury-receive-path.md](../../reverse-engineering/findings/client-mercury-receive-path.md).

| Bound | Value | Grade | Source | Safe level | Break level | Notes |
|---|---|---|---|---|---|---|
| Largest datagram the client receives | 1472 B | static | `FUN_0158a200`: `recvfrom` with a `0x5c0`-byte buffer | n/a | n/a | A larger datagram should fail with `WSAEMSGSIZE`; no live client has reported it yet. The live outcome (`client.mercury.socket_recv` error 10040) is CL-05 and CL-08. |
| Server datagram maximum | 1472 B | config | `crates/mercury/src/lib.rs` `PACKET_MAX_SIZE` | n/a | n/a | Matches the client's receive buffer. Size rule: [mercury-wire-format.md § Reliable datagram size budget](../../protocol/mercury-wire-format.md#reliable-datagram-size-budget). |
| Server fragment body | 1300 B | config | `crates/mercury/src/packet/build.rs` `FRAGMENT_BODY_SIZE` | n/a | n/a | |
| Server data plaintext before piggybacked ACKs | 1411 B | config | `crates/mercury/src/packet/ack_budget.rs` `MAX_DATA_PLAINTEXT_BEFORE_ACKS` | n/a | n/a | |
| Fragments per bundle, client | no cap | static | `Nub::processPacket`, `0x0157fd20` | n/a | n/a | The largest bundle seen is 15 fragments, 18,367 bytes. CL-02 checks `Bundle::iterator::data()`'s temp buffer; CL-09 finds the largest bundle the client processes. |
| Fragments per bundle, server | 64 | config | `crates/mercury/src/lib.rs` `MAX_FRAGMENTS` | n/a | n/a | |
| Open fragment groups per channel | 1 | static | `Channel+0x124`, `Nub::processPacket` `0x0157fd20` | n/a | n/a | Matched by `lastFrag` alone. A stale group is discarded after 60 s of TSC. |
| Client reorder window | 512 packets | static | `Channel+0x2c = 0x200`, read at `0x0158C801` | n/a | n/a | The client ACKs every in-window packet before deciding anything (`queueAckForPacket`, `0x0158cba0`), so "all ACKed" proves nothing about delivery. |
| Server send window | 32 packets | config | `crates/mercury/src/lib.rs` `TX_WINDOW_SIZE` | n/a | n/a | No client reason for 32 (#353). Raise candidate: CL-14. |
| Server resend timeout | 700 ms | config | `crates/mercury/src/lib.rs` `ACK_TIMEOUT_MS` | n/a | n/a | |
| Server resend tries | 20 | config | `crates/mercury/src/lib.rs` `MAX_RETRIES` | n/a | n/a | |
| Server resends per tick | 5 | config | `crates/mercury/src/lib.rs` `RETRANSMIT_BUDGET_PER_TICK` | n/a | n/a | |
| Client resend timing for its own reliable sends | not measured | n/a | n/a | n/a | n/a | CL-02: the client channel's RTO and retry cap. |
| Client socket receive buffer | not measured | n/a | n/a | n/a | n/a | CL-02: any `setsockopt(SO_RCVBUF)`, else the Windows default. |
| Packets the client processes per tick | not measured | n/a | n/a | n/a | n/a | CL-02: whether the game thread drains its incoming queue fully each tick. |
| Client idle send rate | about 6 packets/s; `perfStats` every 15.0 s | measured | SigNoz, 2026-09-19, 1 colo session (77 packets in 13 s) | n/a | n/a | Most are ignored `AUTHENTICATE`s. [Idle send cadence note](../../../.claude/agent-memory/main-session/reference_client_idle_send_cadence.md). |
| Client tolerance of server silence | 15 s | config | `crates/mercury/src/lib.rs` `UE3_INACTIVITY_TIMEOUT_MS`, the client's `NetInactivityTimeout=15` | n/a | n/a | The client's timer, not the server's: the server reaps a silent client after 60 s (`INACTIVITY_TIMEOUT`, `crates/base-session/src/base/tick_sync.rs`). |
| Next-request offset residue (#1341) | multiples of 8, 128 to 848 B, sticky per process | measured | SigNoz, 2026-09-29 to 2026-10-10, 11 readings | n/a | n/a | `Bundle::iterator`'s copy constructor (`0x01578e90`) never writes `+0x14`. When the residue equals a message cursor, the rest of the bundle is lost. CL-05 extends the readings; CL-10 checks a client without the telemetry DLL. |
| Residue exposure | first packet of a chain only | static | `0x01579cd0` | n/a | n/a | |
| Server-side added latency and jitter the session survives | not measured | n/a | n/a | not measured | not measured | Chaos profile `net`. The wrapper is [network-chaos-testing.md](../../architecture/network-chaos-testing.md). |
| Server-side loss rate the session survives | not measured | n/a | n/a | not measured | not measured | Chaos profile `net`. |
| Flush delay after the hold before the client times anything out | not measured | n/a | n/a | not measured | not measured | Chaos profile `flush`. The hold itself is in [aoi.md](aoi.md). |
