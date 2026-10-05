---
name: reliable-datagram-size-budget
description: No reliable datagram may exceed 1472 B (client recvfrom buffer). Single sends are measured before a seq is reserved; oversize bodies are fragmented (PR #1274, 2026-10-05).
metadata:
  type: project
---

The client reads Mercury datagrams into a 1472-byte buffer, so a larger reliable datagram never arrives and every retransmit resends the same bytes. That wedges the stream behind it (`mercury.tx_hole`).

There were two causes:
- Uncapped piggybacked ACKs, fixed in #1141.
- Single-packet sends whose body is data-sized, fixed in #1274. The case found was the NPC `createOnClient` cascade: template 221 has a 1427-byte body, which with 10 ACKs came to 1504 bytes on the wire.

**Rule.** Every single reliable send goes through `send_reliable_to_addr` (`crates/base-session/src/base/helpers/reliable_fit.rs`). It builds the packet once with no ACKs to measure it, before any sequence number is reserved. Then it sizes the ACKs to that packet, or sends the body as a fragmented bundle. Fragments need contiguous sequence numbers, so measuring after reserving cannot work: another task may already hold the next number.

Senders that reserve a range up front (enter-world, the reanchor appearance replay) size it with `fragment_count` and build with `build_fragmented_bundle`.

Verified live 2026-10-05: Petbe #221's 1473-byte cascade went out as 2 fragments, the client's fragment group completed (`assembled_bytes=1473`), and there was no `tx_hole`.

**Why:** fragmenting keeps the "one bundle == one client frame" semantics. A cascade split across separate packets would not.

**How to apply:** for a new reliable sender with a data-sized body, use the fitted send. Never hand-allocate a sequence number and then build. The protocol doc is the authority: `docs/protocol/mercury-wire-format.md` § Reliable datagram size budget. Related: [[na38-client-orders-reliable-stream]], [[cell-gm-class-id-and-raw-bundle-blobs]].
