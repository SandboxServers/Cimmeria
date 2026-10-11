---
name: client-recv-thread-order
description: SGW client runs recvfrom, cipher filter, processFilteredPacket, window and reassembly on the network thread; the tbb queue is only before processOrderedPacket
metadata:
  type: reference
---

Verified 2026-10-11 from the decompile of `Nub::processPendingEvents` `0x01581ab0`
(`docs/reverse-engineering/decompiled/02_bigworld_network.c`): each loop iteration does
`FUN_0158a200` (recvfrom) then `0x01581830` (channel filter `recv` slot 2, or
`processFilteredPacket` directly) on the same packet before the next read. The
`Nub+0x138` tbb queue sits between `processPacket` and `processOrderedPacket` (game thread).

- A per-thread stash from the recvfrom detour reaches the filter detour safely.
- Game-thread hooks (`unpack`) need a cross-thread key: packet object + `Packet+0x44` seq.
- The draft `docs/drafts/spec/mercury-wire-format.md` (prose near "second thread drains",
  Figure 33) puts the queue before the filter: wrong, flagged in
  `findings/client-mercury-receive-path.md` §"Recvfrom to the packet filter".

Related: [[na38-client-orders-reliable-stream]]
