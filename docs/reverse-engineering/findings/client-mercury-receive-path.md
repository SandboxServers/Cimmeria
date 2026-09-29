# Client Mercury receive path: packet filter, reliable window, fragment reassembly and the bundle message loop

> **Date**: 2026-09-29
> **Method**: Ghidra (MCP, `SGW.exe` QA build): decompile plus disassembly of every function named below; every `ret N` and argument slot read from the prologue/epilogue bytes, not only from the decompiler. Function names in Ghidra are partly stale (`Mercury_Nub_12` is `processFilteredPacket`, `Mercury_Nub_14` is `processPacket`, `Mercury_Nub_5` is a caller named after the wrong slot); this doc uses the meaning, and gives the address.
> **Status**: statically verified. Nothing here has been observed live yet; the five telemetry seams ("Telemetry seams" below, catalog in [client-telemetry.md](../../architecture/client-telemetry.md)) exist to confirm it. Items marked **inferred** are read from code shape.
> **Feeds**: the `client.mercury.packet_in`, `client.mercury.fragment` and `client.mercury.bundle` events; the server-side audit of `crates/mercury`'s fragment encoder; the "AoI bundle partially processed" repro (15-fragment, 18,367-byte reliable bundle whose tail is never processed although every fragment was ACKed).
> **Related**: [mercury-nub-anatomy.md](mercury-nub-anatomy.md), [mercury-protocol-internals.md](mercury-protocol-internals.md), [client-entity-lifecycle.md](client-entity-lifecycle.md), [../../protocol/mercury-wire-format.md](../../protocol/mercury-wire-format.md).

## Summary

The client receives a datagram on the network thread, strips its footers from the tail, runs the reliable window, and either dispatches the packet or **assembles fragments into a chain**. A complete bundle (one packet, or the assembled chain) is then pushed onto a queue and processed on the **game thread** by a message loop that walks the packet chain with an iterator. Three facts decide whether our 15-fragment bundle works:

1. **The client ACKs before it decides anything.** `queueAckForPacket` (`0x0158cba0`) inserts the ACK for every packet with an in-range sequence number before the in-order test. A fragment that is later dropped (wrong group, missing group start, out of window, duplicate) has already been acknowledged, so the server retires it and never resends. "All ACKed, no retransmits" proves nothing about reassembly.
2. **Fragment groups are matched by `lastFrag` alone, one group per channel.** Any reliable fragment whose `lastFrag` differs from the open group's is dropped permanently.
3. **A message header must lie wholly inside one packet; only a message *body* may straddle a packet boundary.** A header that crosses the end of a fragment aborts the rest of the bundle (`"Discarding bundle due to corrupted header"`), silently, with every later message lost. This is the only condition found in this path that makes a fully assembled, fully ACKed bundle stop partway. Whether the server's encoder can produce such a split is for the server-side audit; the client-side rule is exact and stated below.

## Function map

| Address | Meaning | Thread | Signature (from prologue/epilogue) |
|---|---|---|---|
| `0x01581830` / `0x0166f680` | callers of the packet filter (recvfrom loop / a second path) | network | |
| `0x01580840` | `Nub::processFilteredPacket`: flags byte, ack and sequence footers, reliable window, chain dispatch | network | `thiscall(Nub*, NetworkAddress*, Packet*) -> int`, `ret 8` |
| `0x0157c7b0` | channel lookup by address (`Nub` -> channel, `0` if none) | network | `thiscall(Nub*, NetworkAddress*) -> Channel*` |
| `0x0158cba0` | `UnAckedHandler::queueAckForPacket`: ACK insert, in-order test, reorder buffer, chain of releasable packets | network | `thiscall(Channel*, Out*, Packet*, uint seq)`, `ret 0xc` |
| `0x0158cac0` | outbound-ACK processing (an ACK received from the server) | network | |
| `0x0158bb50` | unreliable-packet duplicate test | network | `thiscall(Channel*, uint seq) -> bool` |
| `0x0157fd20` | `Nub::processPacket`: fragment footers, group table, reassembly, push to the game-thread queue | network | `thiscall(Nub*, NetworkAddress*, Packet*, Channel*) -> int`, `ret 0xc` |
| `0x0157b120` | fragment group "stale" test (60 s of TSC) | network | `fastcall(Group*) -> bool` |
| `0x0157e2c0` | fragment group constructor | network | `thiscall(Group*, last, remaining, tsc_lo, tsc_hi, Packet*)` |
| `0x0158d7d0` / `0x0158d6a0` | `ClientIncomingMessage` constructor (the queue entry) | network | |
| `0x0150c7c2` | `concurrent_queue::push` on `Nub+0x138` | network | |
| `0x0158d3f0` | queue consumer: calls `processOrderedPacket` | game | |
| `0x0157c820` | `Nub::processOrderedPacket`: the bundle message loop | game | `thiscall(Nub*, ClientIncomingMessage*) -> int`, `ret 4` |
| `0x0157a1b0` / `0x01579710` | `Bundle::begin()`: iterator init | game | |
| `0x01578df0` | iterator `!=` end | game | |
| `0x01578f50` | iterator `msgID()` | game | |
| `0x01579830` | `Bundle::iterator::unpack(InterfaceElement*)`: header parse, header/body fit tests | game | `thiscall(Iterator*, InterfaceElement*) -> MsgInfo*`, `ret 4` |
| `0x01579a50` | `Bundle::iterator::data()`: body pointer, temp buffer for a straddling body | game | `fastcall(Iterator*) -> ptr` |
| `0x01579cd0` | `Bundle::iterator::next()`: advance, cross packet boundaries | game | `fastcall(Iterator*)` |
| `0x0158aa40` | header length of an `InterfaceElement` (1 fixed, `1+width` variable, `-1` unknown style) | game | `fastcall(Element*) -> int` |
| `0x0158b770` | `InterfaceElement::expandLength` (reads the inline length) | game | |

The log function every `[Mercury] ...` string is passed to is `0x0081c2e0`. **In this build it is a one-byte stub (`c3`, followed by `int3` padding), so every Mercury log line is discarded.** The strings are still useful as anchors; they never print.

## Packet and channel layouts used

`Packet` (refcounted, refcount at `+4`, vtable at `+0`):

| Offset | Field | Evidence |
|---|---|---|
| `+0x08` | `next` packet in a chain (getter `0x0158a850`, setter `0x015792f0`) | both read/write `[this+8]` |
| `+0x24` | data length (flags byte included; shrinks as footers are stripped) | every footer read does `len -= N` |
| `+0x28` | running count of stripped footer bytes | `+= N` beside each strip |
| `+0x30` | `firstRequestOffset` (u16) | set from the `0x01` footer; copied into the iterator |
| `+0x44` | sequence number | `0x40` footer |
| `+0x4c` / `+0x50` | `firstFrag` / `lastFrag` | `0x20` footers |
| `+0x54` | data; `data[0]` is the **flags byte**, payload starts at `data[1]` | `TEST byte [ESI+0x54]`; iterator starts its cursor at 1 (`0x01579710`) |

Channel (`ChannelInternal`, the object `queueAckForPacket` is called on and the one `processPacket` receives):

| Offset | Field |
|---|---|
| `+0x30` | reorder window size (max accepted distance ahead of `inSeqAt`) |
| `+0x40` / `+0x44` | reorder slot table / slot mask (`slot = seq & mask`) |
| `+0x48` | packets currently buffered ahead of `inSeqAt` |
| `+0x50` | `inSeqAt`, the next expected reliable sequence (`0x10000000` = unset) |
| `+0x80` | address string (used in log lines) |
| `+0x9c` | pending-ACK set; `+0x110` "has pending ACK" flag |
| `+0x124` | **the open fragment group**, one per channel (`0` if none) |

Fragment group (`0x14` bytes, ctor `0x0157e2c0`): `+0` `lastFrag`, `+4` fragments still missing, `+8`/`+0xc` TSC of the last fragment added, `+0x10` head of a **seq-sorted** packet list linked through `Packet+8`.

`Nub` counters read below: `+0xf8` bad-packet counter (incremented on almost every drop path), `+0x10c` messages dispatched, `+0x100` bundles finished, `+0x118` bundles left with unprocessed messages, `+0x11c` unreliable duplicates.

## Flags byte (`data[0]`)

The table in [mercury-protocol-internals.md](mercury-protocol-internals.md) lists `0x20` as "has sequence number" and `0x40` as "has requests"; the receive code says otherwise. **Correction**, read from `0x01580840` and `0x0157fd20`:

| Bit | Meaning here | Evidence |
|---|---|---|
| `0x01` | `firstRequestOffset` (u16) footer present | `processPacket` reads a u16 at `len-2` into `Packet+0x30` |
| `0x02` | piggyback: a whole packet is nested in this one | `processFilteredPacket` else-branch: length u16 (high bit = inverted), copies into a new 0x614-byte packet, recurses |
| `0x04` | ACK footer present | count byte then `count` u32 sequence numbers |
| `0x08` | on a channel (the channel is looked up by source address) | `0x0157c7b0` |
| `0x10` | reliable | selects `queueAckForPacket` |
| `0x20` | **fragment** | `processPacket` `TEST [ESI+0x54],0x20` |
| `0x40` | **sequence number footer** present | `processFilteredPacket`: `(flags & 0x40) == 0` is a drop |
| `0x80` | rejected: `"received packet with bad flags"`, returns `-4` | top of `0x01580840` |

## Footer layout and strip order

All values are read as raw **little-endian** words straight from the buffer (`*(uint*)(data + n)`): no byte swap anywhere in this path. The client strips from the tail; the resulting **wire order, front to back, is derived by reversing the strip order**:

```
payload | firstFrag u32 | lastFrag u32 | firstRequestOffset u16 | seq u32 | ack u32 x n | n u8
          [0x20]           [0x20]         [0x01]                    [0x40]    [0x04]        [0x04]
```

Strip order, tail first: (`processFilteredPacket`) the ACK count byte, then `n` ACKs, then the sequence number; (`processPacket`) the `firstRequestOffset`, then `lastFrag`, then `firstFrag`. Length guards fail with a `"Not enough data..."` drop when too few bytes remain (`len-1 < 4*n`, `len-1 < 4`, `len-1 < 8`).

`seq` is rejected when it equals `0x10000000` or has any bit above 28 set. A packet **without** the `0x40` footer is dropped after its ACKs were processed (an ACK-only packet returns `-4`; that is a harmless statistic, not a fault).

## The reliable window (`queueAckForPacket`, `0x0158cba0`)

For a reliable packet (`0x10`) the client requires `0x08` and a registered channel; otherwise it drops with `-4`. Then, on the channel:

1. If `inSeqAt == 0x10000000` it is set to this packet's `seq`. **The first reliable packet seen on a channel defines the window origin.** A reliable packet that arrives out of order *before* the first one is then older than `inSeqAt` and is dropped forever (but ACKed).
2. Range check: `seq` must fit in 28 bits.
3. **The ACK is inserted (`0x0157ac40` on `+0x9c`) and `+0x110` set, unconditionally, before any of the tests below.**
4. `seq == inSeqAt`: deliver. `inSeqAt++`, then drain the reorder slots: while `slot[inSeqAt & mask]` is occupied, chain that packet behind the previous one (`Packet+8`), clear the slot, `+0x48--`, `inSeqAt++`. The returned chain is the in-order run.
5. `seq` ahead of `inSeqAt` by `0 < d < 0x8000001` (mod 2^28): if `d > window` (`+0x30`) drop as out of range; else if `slot[seq & mask]` is empty, store the packet (`"Buffering packet #%d above #%d"`) and return no chain; else it is a duplicate, dropped.
6. Otherwise (behind `inSeqAt`): dropped as out-of-range / old duplicate.

Every non-delivery case returns "nothing to deliver, no error" (`processFilteredPacket` returns `0`). The window slot lookup **does not compare the buffered packet's own seq to `inSeqAt`**; correctness relies on the range check.

`processFilteredPacket` then walks the returned chain (`0x01580f25`): for each packet it saves `next`, clears `next` (`0x015792f0(pkt, 0)`), calls `processPacket(pkt, channel)`, and keeps the first non-zero result. The client's own window is 512 slots (`Channel+0x2c = 0x200`, per the ACK memory note), so 15 back-to-back sequence numbers are inside it.

## Fragment reassembly (`processPacket`, `0x0157fd20`)

Packets without `0x20` skip all of this: `0x0158a4f0` stamps the packet chain's TSC, a `ClientIncomingMessage` is built (`0x0158d7d0`), pushed on `Nub+0x138`, and `processPacket` returns `0` (`0x01580419`).

For a fragment (`0x20`), after the optional `0x01` u16:

1. **Footers**: needs `len-1 >= 8`, reads `lastFrag` (`Packet+0x50`) then `firstFrag` (`Packet+0x4c`). `count = lastFrag - firstFrag + 1`, plain signed subtraction with **no 28-bit wrap handling**. `count < 2` drops (`"Dropping fragment due to illegal bundle size"`, `-4`).
2. **Find the group.** For a packet on a channel the group is `Channel+0x124` (single slot). For a packet not on a channel it is a hash-table entry in `Nub+0xac` keyed by address and `b5 ^ b3 ^ firstFrag`.
3. **No group** (or the old one was discarded, step 4): if on a channel and `packet.seq != firstFrag`, drop (`"Bundle (#%d,#%d) is missing"`, `-4`). Otherwise **create** the group: `last = lastFrag`, `remaining = count - 1`, list = this packet, TSC = now. Store it in `Channel+0x124`. Return `0`. The first fragment is *not* counted against `remaining`; it is already in the list.
4. **Group exists, discard test**: if the list head is **unreliable**, or the group is stale (`0x0157b120`: more than `60 x ticks-per-second` TSC since the last fragment was added), log `"Discarding abandoned stale overlapping bundle"`, free the group, and fall to step 3 for this packet.
5. **Group exists, `group.last != packet.lastFrag`**: an unreliable fragment is discarded; a **reliable one is dropped with `-4` (`"Mangled fragment footers, lastFrag ..."`) and the group is left open**. The packet was already ACKed and `inSeqAt` already passed it, so it is gone for good.
6. **Same group** (`group.last == lastFrag`): refresh the TSC. Walk the sorted list: an equal `seq` is a duplicate (`"Discarding duplicate fragment #%d"`, dropped, `remaining` unchanged). Otherwise insert in `seq` order (compare `(new - node) & 0x0fffffff > 0x8000000` means "node is after"), `remaining--`.
7. **`remaining <= 0`: complete.** Free the group, clear `Channel+0x124`, and treat the head of the sorted list as the bundle: same path as a non-fragment (`0x01580420`): push one `ClientIncomingMessage` for the whole chain.

What the client does **not** check: that a fragment's own `seq` lies in `[firstFrag, lastFrag]`; that the sequence numbers of the fragments are contiguous; that the count of fragments equals the number of distinct `seq`s it saw for any reason except the `remaining` counter; and it applies no cap on `count` in this function (the 64-fragment limit in the protocol doc is not enforced here). Consequences for the encoder:

* All fragments of a bundle must carry the **same `lastFrag`**, and the first fragment must arrive first with `seq == firstFrag` (a reliable channel delivers in `seq` order, so this holds when `firstFrag` is the lowest `seq`).
* A second fragmented bundle must not start until the first is complete on that channel: its fragments have a different `lastFrag` and are dropped (and ACKed).
* Non-fragment reliable packets interleaved between the fragments are *not* dropped, but they are dispatched as soon as they arrive, **ahead of** the fragmented bundle, which only completes at its last fragment. Message order across that boundary is therefore inverted.
* Retransmitted fragments are harmless: duplicates are discarded by `seq`.

## The bundle message loop (`processOrderedPacket`, `0x0157c820`)

Runs on the game thread when it pops the `ClientIncomingMessage`. Input: the packet chain (`message+0x10 -> +4` head packet) and the source address. The iterator (`0x0157a1b0`) starts at the first packet with payload (`len > 1`; empty leading packets are skipped) at cursor `1`. Iterator fields: `+0` packet, `+4` (u16) packet data length, `+6` (u16) cursor, `+8` (u16) body offset, `+0xc` body length, `+0x10` temp buffer, `+0x14` (u16) `firstRequestOffset`, `+0x18` msg id, `+0x19` request flag (`0x20` marks an error), `+0x1c` reply id, `+0x20` decoded length.

Per message, in order:

1. **Message id**: `data[cursor]`. `table = Nub+0xc`, stride `0x24`. **Handler at `element+0xc` null: stop the loop** (`"unknown message id"`, result `0xfffffffb`); the rest of the bundle is discarded.
2. **`unpack`** (`0x01579830`):
   * header length `h` from `0x0158aa40`: `1` for a fixed-length element, `1 + width` for a variable-length one (`WORD_LENGTH` = 3), `-1` for an unknown length style;
   * **if `packetLen < cursor + h` the header does not fit in this packet: error** (`"Bundle::iterator::unpack: Error unpacking header length at %d"`), `+0x19 = 0x20`;
   * the length is read straight from the packet with `expandLength` (widths 1-4; `Overflow ... length` returns `-1`);
   * if `firstRequestOffset == cursor`, the message is a request: 6 more header bytes (4 reply id, 2 next request offset);
   * **if the body does not fit (`packetLen < bodyOffset + length`) it may straddle, but only if a next packet exists**; otherwise error.
3. `(msgInfo & 0x2000)` set (the error mark above) or `data()` (`0x01579a50`) returning `0` ends the loop with **`0xfffffffc`**, log `"Discarding bundle due to corrupted header for message"`. The remaining messages of the bundle are dropped.
4. **`data()`**: if the body lies inside one packet it is used in place. If it starts exactly at the end of a packet and fits in the next it is used in place from the next packet's `data[1]`. Otherwise it is copied into a temp buffer chunk by chunk: the first chunk from the body offset to the packet end, every following chunk from `data[1]` to that packet's end. The run-out-of-packets case logs `"Bundle::iterator::data: Run out of packets after %d of %d bytes"` and returns `0`.
5. **Dispatch**: `element->handler->vtable[1](addr, msgInfo, MemoryIStream(len), &flag)`. Counters `Nub+0x10c`, `element+0x1c`, `element+0x20` increment. After the handler, `iterator.next()` runs regardless of how much of the body the handler consumed (a leftover only logs `"MemoryIStream::~MemoryIStream: There are still %d bytes left"`). **A handler that mis-parses its own body cannot desync the loop.**
6. **`next()`** (`0x01579cd0`): `cursor' = bodyOffset + length`. If `cursor' < packetLen` stay in the packet. Otherwise step to the next packet, `cursor' -= packetLen`, then `cursor = 1 + cursor'` (a continuation begins at `data[1]`, immediately after the flags byte, with no per-fragment message header), repeating while the next packet is also shorter than the cursor. A null packet ends the iteration; that is the only normal end.

Loop exit: the `+0xc2` "break" flag is written only by the constructor and by `processOrderedPacket` itself (`0`), so it never breaks the loop (verified by a full-image instruction search). Otherwise the loop ends on end-of-chain (`Nub+0x100++`, result `0`) or on one of the two error results above (`Nub+0x118++`, result `0xfffffffb`/`0xfffffffc`, no ACK or retransmit consequence).

### The header-straddle rule (the encoder must honour it)

`unpack` demands `cursor + h <= packetLen` for the **header**. A fragment that ends after the message-id byte, or after the id and the first length byte, of a `WORD_LENGTH` message makes the client drop every message from there to the end of the bundle. No log is produced (the logger is a stub); the only visible traces are the `Nub+0x118` counter and the missing messages. BigWorld's own `Bundle::newMessage` starts a new packet when the header would not fit; an encoder that splits its byte stream at a fixed size **must** do the same, and must also apply the rule at every fragment boundary, not only the first.

## Every discard and abort in this path

| Where | Condition | Effect | Log string (address) |
|---|---|---|---|
| `0x01580840` | flags `0x80`, or fewer than 2 bytes | drop, `-4` | `0x01b17e98` bad flags |
| `0x01580840` | `0x04` and too few bytes for the ACK footer | drop | |
| `0x01580840` | no `0x40` seq footer | drop (ACKs already handled) | |
| `0x01580840` | `seq == 0x10000000` or above 28 bits | drop | |
| `0x01580840` | reliable without `0x08`, or no channel for the address | drop | |
| `0x01580840` | unreliable duplicate (`0x0158bb50`) | `-0xb`, counter `+0x11c` | |
| `0x0158cba0` | `seq` out of range, ahead by more than the window, behind `inSeqAt`, or duplicate slot | not delivered, **already ACKed** | `0x01b19e78` out-of-range |
| `0x0157fd20` | fewer than 8 footer bytes, or `count < 2` | drop `-4` | `0x01b186f0`, `0x01b18810` |
| `0x0157fd20` | no group and `seq != firstFrag` on a channel | drop `-4` | `0x01b188d0` "is missing" |
| `0x0157fd20` | reliable fragment with `lastFrag` != open group's | drop `-4`, group kept | `0x01b189a8` mangled footers |
| `0x0157fd20` | unreliable fragment, other group | discard | `0x01b18928` |
| `0x0157fd20` | duplicate `seq` in the group | discard, count unchanged | `0x01b18a04` |
| `0x0157fd20` | group head unreliable, or 60 s stale | group freed, this packet tried as a new group | `0x01b18868` |
| `0x0157c820` | handler null for the message id | loop stops, `0xfffffffb` | |
| `0x01579830` | header does not fit the packet | loop stops, `0xfffffffc` | `unpack: Error unpacking header length` |
| `0x01579830` / `0x01579a50` | body straddles but no next packet | loop stops, `0xfffffffc` | `data: Run out of packets` |

## Assessment against the observed pattern

The observed pattern is: all 15 fragments ACKed, the first 12 NPCs' messages processed in order, then a gap of about three NPCs, one stray later message, then nothing.

* **Reassembly cannot yield a partial bundle.** The bundle is queued only when `remaining` reaches 0, and the message loop starts only then. A dropped fragment (wrong `lastFrag`, missing group start, out of window, stale group) means **no message of the bundle is processed at all**, not a prefix. A prefix therefore needs a fully assembled chain and a message loop that stops midway.
* **The message loop stops midway in exactly two ways** (unknown message id; header or body that does not fit), and after either it processes **nothing more**. The "nothing more" half of the observation matches the header-straddle abort.
* **The client cannot skip messages inside the loop.** `next()` advances by the declared length, independently of the handler. A "gap of about three NPCs, then one stray message" is therefore **not** a Mercury-level skip. It matches the entity layer instead: [client-entity-lifecycle.md](client-entity-lifecycle.md) shows that a message for an entity that is not yet in the world is **queued** (`client.mercury.entity_method` with `path = queued`, logged at `info`; delivered ones are `debug`), and an NPC whose `enterAoI` count is 0 when its create arrives is parked. That would look like "skipped" in a `debug`-filtered view. **Unconfirmed**: the new `client.mercury.bundle` event states how many messages the loop dispatched, which separates the two explanations.
* **Concrete candidate, not yet proven**: a `WORD_LENGTH` message header (3 bytes) that a fragment boundary splits. It is data-dependent (NPC order changes the bytes) and probabilistic (a boundary falls inside a header in a few percent of boundaries; 14 boundaries per bundle), which fits "about 1 login in 4, different subset every time". The server-side audit must check whether the encoder starts a new fragment when fewer than `h` bytes remain.

## Telemetry seams

Five inline hooks in `cimmeria-client-telemetry` (`inline_hooks/mercury_recv.rs`, portable logic in `hooks/mercury_recv/`) turn this path into events; the event catalog is in [client-telemetry.md](../../architecture/client-telemetry.md) ("Mercury receive path"). All five prologues are fingerprinted (`push -1; push <SEH handler>; mov eax, fs:[0]` shapes, plus `push ebp; mov ebp, esp` for the filter), and each `ret N` was read from the function's epilogue bytes.

| Hook | Reads | Says |
|---|---|---|
| `0x01580840` `processFilteredPacket` | the datagram (copy), `Nub+0xf8` before and after | footers, the filter's result, whether a drop path bumped the bad-packet counter |
| `0x0158cba0` `queueAckForPacket` | channel `+0x30`, `+0x48`, `+0x50` before and after | delivered (and how many followers were released), buffered, duplicate, out of window, stale |
| `0x0157fd20` `processPacket` | the fragment's footers, the channel's group (`+0x124`, `+0`, `+4`, `+0x10` list) before and after | started, added, completed (assembled bytes and packet count), duplicate, mangled footers, bundle missing, restarted |
| `0x0157c820` `processOrderedPacket` | the bundle's packet chain, `Nub+0x10c`/`+0x118` before and after | source, total bytes, boundaries, how the loop ended |
| `0x01579830` `unpack` | the iterator (`0x24` bytes) and the message's interface element before and after | each message's offset, header, length, straddle; the exact header that did not fit |

**Static versus live.** Everything in the layout, footer and rule sections is read from decompile and disassembly and is unit-tested against synthetic memory built to those layouts. **Not yet confirmed live**: that the four-word argument list of `queueAckForPacket` is `(out, packet, seq, seq)` in that order (the hook does not depend on it: it reads the channel only and pairs with the filter's own parse); that `ClientIncomingMessage+0x10 -> +4` is the head packet on the assembled path as well as the single-packet one (read from `processOrderedPacket`'s decompile, `*(*(param_1 + 0x10) + 4)`); and that the header-straddle rule is what the server's encoder violates in the failing logins (a hypothesis until a `client.mercury.bundle` end event with `fault = header_does_not_fit_packet` shows it).

## Open questions

* The size of the interface table at `Nub+0xc` (an id at or above it goes through `_invalid_parameter_noinfo`, not a clean drop). Expected 256; not verified.
* `0x0158cac0` (ACKs received from the server) and the piggyback branch were read only for their footer handling.
* Whether any path enqueues a second `ClientIncomingMessage` for the same reliable stream out of order (the game-thread queue is FIFO; a fragmented bundle is queued at its last fragment, interleaved non-fragment packets earlier).
