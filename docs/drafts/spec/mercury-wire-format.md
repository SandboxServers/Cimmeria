---
title: Mercury Wire Format
chapter_id: spec.protocol.mercury-wire-format
status: draft
last_verified: 2026-05-14
verified_by: automated-agent
confidence:
  re: high
  client: n/a
  deprecated: n/a
  rust_expected: n/a
  rust_actual: n/a
evidence_refs:
  re:
    - docs/reverse-engineering/findings/mercury-protocol-internals.md
    - docs/reverse-engineering/findings/world-entry-pipeline.md
    - docs/reverse-engineering/findings/entity-creation-wire-formats.md
    - docs/reverse-engineering/findings/system-protocol-wire-formats.md
    - docs/reverse-engineering/findings/position-movement-wire-formats.md
    - docs/reverse-engineering/findings/space-viewport-wire-formats.md
    - docs/reverse-engineering/findings/entity-property-sync.md
    - ghidra://SGW.exe@0x015841d0
    - ghidra://SGW.exe@0x01582160
    - ghidra://SGW.exe@0x01581ab0
    - ghidra://SGW.exe@0x01580840
    - ghidra://SGW.exe@0x0157fd20
    - ghidra://SGW.exe@0x0157c820
    - ghidra://SGW.exe@0x01576f90
    - ghidra://SGW.exe@0x0157a7a0
    - ghidra://SGW.exe@0x0157ac90
    - ghidra://SGW.exe@0x0157a990
    - ghidra://SGW.exe@0x0158acc0
    - ghidra://SGW.exe@0x0158b770
    - ghidra://SGW.exe@0x0158b120
    - ghidra://SGW.exe@0x01588530
    - ghidra://SGW.exe@0x01588ec0
    - ghidra://SGW.exe@0x01603a70
    - ghidra://SGW.exe@0x01603b80
    - ghidra://SGW.exe@0x01603fa0
    - ghidra://SGW.exe@0x01604d00
    - ghidra://SGW.exe@0x017bade0
    - ghidra://SGW.exe@0x017bb200
    - ghidra://SGW.exe@0x00dd9280
    - ghidra://SGW.exe@0x00dda0e0
    - ghidra://SGW.exe@0x00dd8510
    - ghidra://SGW.exe@0x00dd6a60
    - ghidra://SGW.exe@0x00dd6980
    - ghidra://SGW.exe@0x00dd9ee0
    - ghidra://SGW.exe@0x00dda2e0
    - ghidra://SGW.exe@0x00dda6c0
    - ghidra://SGW.exe@0x00dddca0
    - ghidra://SGW.exe@0x00dddd80
    - ghidra://SGW.exe@0x01590df0
  client: []
  deprecated: []
  rust: []
related_chapters:
  - spec.protocol.entity-property-sync
  - spec.protocol.message-catalog
  - spec.engine.universal-rpc-dispatcher
  - spec.world.world-entry
disputed_by: []
supersedes: []
---

# Mercury Wire Format

A Mercury packet is a UDP datagram with one flags byte at the front, a payload of one or more interface-element messages in the middle, and a footer of acks, sequence numbers, and fragment boundaries reversed at the back. Above that envelope, the channel layer adds AES-256-CBC + HMAC-MD5 on every reliable packet, with a literal-zero IV and no KDF. This chapter pins each of those bytes to the binary that emits them.

The Mercury layer in SGW is BigWorld's reliable-UDP transport with a small set of deliberate SGW customizations — a 1-byte flags field instead of stock BigWorld's `uint16`, little-endian footers instead of network order, AES instead of Blowfish, an 8-byte `ENABLE_ENTITIES` payload instead of 1, and a `forcedPosition` message extended by 13 bytes. Each divergence is called out inline next to the stock baseline.

---

## Section 1 — RE findings

This section distills the seven V5 finding docs that together canonize the Mercury wire format: `mercury-protocol-internals.md` (packet flags, footer parse order, cipher chain, sequence-number constants, Nub construction); `world-entry-pipeline.md` (phase-by-phase sequence + `ENABLE_ENTITIES` payload reconciliation); `entity-creation-wire-formats.md` and `space-viewport-wire-formats.md` (byte-level layouts for every server-to-client message in the `0x00–0x37` range, including the full `RESOURCE_FRAGMENT` 0x36 dissection); `system-protocol-wire-formats.md` (Ghidra decompilation evidence for the system-message handlers, plus the `startEntityMessage`/`startProxyMessage` cell-vs-base wire-shape distinction); `position-movement-wire-formats.md` (the 32 `UPDATE_AVATAR` variants + `detailedPosition` + `forcedPosition` byte tables); and `entity-property-sync.md` §13 (the sub-slot client-method encoding threshold). Every claim below resolves to a Ghidra anchor in `SGW.exe` (image base `0x00400000`); the address-map provides the persistent symbol table.

### 1.1 Packet anatomy

A Mercury packet is the contents of a single UDP datagram. The on-wire layout is three concatenated regions:

```text
+---------+--------------------------------------------+---------+
|  flags  |          message body (interface           | footer  |
| (1 B)   |  elements packed end-to-end)               | (var)   |
+---------+--------------------------------------------+---------+
   ^                                                       ^
   |                                                       |
  read forward from offset 0                  read backward from end of datagram
```

The packet flags byte at offset 0 is the *only* field at a fixed position. Every other field — sequence number, ack count, ack list, fragment IDs, first-request-offset — lives in a variable-width footer parsed *backward* from the end of the datagram, gated on bits in the flags byte. The message body fills the space between flags and the consumed footer tail.

**Maximum packet size**: `0x5AD` bytes (1453). Stamped at `ghidra://SGW.exe@0x0157ac90` as the per-packet space check in `Mercury::Bundle::newMessage` — a message that does not fit triggers `Bundle::reserve` (`ghidra://SGW.exe@0x0157a5d0`) to allocate a new packet, and the bundle fragments across packets when the bundle's total exceeds 64 packets (`Packet::MaxFragmentsPerBundle`). The constant is V5-anchored via `mercury-protocol-internals.md` §"Protocol Constants" — a derivation of 1453 from Ethernet/IP/UDP/cipher-tag overhead is not in the V5 record; treat the value as the binary's stamped cap, not as a network-layer calculation.

(The flags byte is also stored at in-memory offset `+0x54` of the `Mercury::Packet` struct per `mercury-protocol-internals.md` §"Packet Flags Byte". That offset is an artifact of the in-memory struct layout; on the wire the flags byte unconditionally occupies byte offset 0 of the datagram. Do not treat `+0x54` as a wire-format claim.)

**Worked example.** A reliable, sequenced, fragmented bundle's first packet — flags byte `0xB8` (`0x80 | 0x20 | 0x10 | 0x08` = `FLAG_IS_FRAGMENT | FLAG_HAS_SEQUENCE_NUMBER | FLAG_IS_RELIABLE | FLAG_ON_CHANNEL`) — has the following on-wire shape. Bytes are shown in **wire order** (low offset → high offset). Pop order is the reverse — see the explicit pop-order callout below:

```text
WIRE ORDER (low offset → high offset, as transmitted):

byte 0:         0xB8                           ← flags (FLAG_IS_FRAGMENT|...)
byte 1..N:      [interface element calls]      ← message body, packed end-to-end
byte N+1..N+4:  sequenceId       (u32 LE)      ← written first by Bundle::finalise
byte N+5..N+8:  lastFragmentId   (u32 LE)
byte N+9..N+12: firstFragmentId  (u32 LE)      ← written last; sits at end of datagram

POP ORDER on receive (from end of datagram, moving toward byte 0):

  1. pop firstFragmentId  (last on wire = first popped)
  2. pop lastFragmentId
  3. pop sequenceId       (first on wire = last popped)
```

Pop order is the **inverse** of the wire order because every footer field is read backward from the end of the datagram — the last field appended is the first field consumed. See §1.3 for the full peel-order rules.

A non-fragmented unreliable position-update packet — flags byte `0x28` (`0x20 | 0x08` = `FLAG_HAS_SEQUENCE_NUMBER | FLAG_ON_CHANNEL`) — has only the 4-byte sequence ID in its footer. A purely unreliable broadcast — flags byte `0x00` — has no footer at all; the entire packet is `[0x00][body]`. The flags byte's bits are the contract: setting bit N obligates the sender to append a specific footer field at the end and the receiver to pop that field on parse.

**Divergence from stock BigWorld 2.0.1.** Stock BW uses a `uint16` flags field at the front of the packet (2 bytes, network-byte-order). SGW collapses this to a `uint8` (1 byte). The low byte of stock BW's `uint16` carries flags 0x01–0x80; the high byte carries 0x0100 (`FLAG_HAS_CHECKSUM`), 0x0200 (`FLAG_CREATE_CHANNEL`), and 0x0400 (`FLAG_HAS_CUMULATIVE_ACK`). SGW omits all three: no CRC32 checksum, no create-channel marker, no cumulative-ack mechanism. The full divergence inventory is in §1.13.

### 1.2 Header (the packet flags byte)

The flags byte is the gate for the entire packet shape. Eight bits, mapped exactly to stock BigWorld's low byte:

| Bit | Mask | Flag | Triggers (on send) | Triggers (on receive) |
|----:|------|---|---|---|
| 0 | `0x01` | `FLAG_HAS_FIRST_REQUEST_OFFSET` | Bundle contains at least one request message | Reader pops a `uint16 firstRequestOffset` from the footer |
| 1 | `0x02` | `FLAG_HAS_PIGGYBACKS` | A previously sent reliable packet is being retransmitted inline | Reader processes piggybacked sub-packets before main body |
| 2 | `0x04` | `FLAG_HAS_ACKS` | One or more reliable packets are being acknowledged | Reader pops `uint8 ack_count` then `ack_count * uint32` from the footer |
| 3 | `0x08` | `FLAG_ON_CHANNEL` | Packet is bound to a registered Mercury channel | Reader routes to the channel's `processFilteredPacket` |
| 4 | `0x10` | `FLAG_IS_RELIABLE` | Packet requires acknowledgement; goes into the send window | Receiver schedules an ack for this packet |
| 5 | `0x20` | `FLAG_HAS_SEQUENCE_NUMBER` | Packet carries a sequence number for ordering / dedup | Reader pops `uint32 sequenceId` from the footer |
| 6 | `0x40` | `FLAG_HAS_REQUESTS` | Bundle contains request/reply messages | Reader uses the popped firstRequestOffset to walk request chain |
| 7 | `0x80` | `FLAG_IS_FRAGMENT` | Packet is one fragment of a larger bundle | Reader pops two `uint32` fragment IDs (begin, end) from the footer |

Source of truth: the flag-mask table at `ghidra://SGW.exe@0x01580840` (`Mercury::Nub::processFilteredPacket_inner`), which decodes each bit in order to peel the matching footer field off the back of the datagram. Bit 7 (`FLAG_IS_FRAGMENT`) is the largest peel because it consumes 8 bytes (two `uint32` fragment IDs). The decode order in `processFilteredPacket_inner` is the reverse of the bit order, because each peel shortens the buffer that the next peel reads from.

**Two flags are present in stock BW but absent in SGW.** `FLAG_HAS_CHECKSUM` (`0x0100` in stock BW's `uint16`) would compute a CRC32 over the packet contents — SGW drops the field because the HMAC-MD5 tag in the cipher envelope provides packet integrity at a higher layer. `FLAG_HAS_CUMULATIVE_ACK` (`0x0400`) would advertise a "all packets up to sequence N are acknowledged" optimization — SGW omits it because external (client-facing) Mercury channels never need it; cumulative acks in stock BW are an internal-channel feature.

**Indexed-channel routing is not available.** Stock BigWorld's `uint16` flags field carries `FLAG_INDEXED_CHANNEL` in its high byte (`0x0800`, per `external/BigWorld-2.0.1/src/lib/network/packet.hpp`) — used to route a packet to one of many addressable channels on a single endpoint. SGW's 1-byte flags only retains the low-byte flags (`0x01`–`0x80`) and therefore has nowhere to put `FLAG_INDEXED_CHANNEL` — the indexed-channel-routing mechanism is simply absent from SGW's wire format. The SGW baseapp connection topology does not need it: one client owns one Mercury channel; routing happens via `ChannelInternal` lookup, not via a flag bit. Bit 7 in both stock BW and SGW unambiguously means `FLAG_IS_FRAGMENT`; the indexed-channel divergence is the *absent* high-byte flag, not a low-byte bit-7 collision.

**`FLAG_IS_RELIABLE` (bit 4) is the load-bearing flag for the entire reliability layer.** When set, the sender's `ChannelInternal` (the ~0x180-byte inner channel object at `ghidra://SGW.exe@0x0158c7b0`) places the packet into a fixed-size circular send window of in-flight reliable packets and starts a 700ms resend timer; the receiver schedules an ack via `UnAckedHandler::queueAckForPacket` at `ghidra://SGW.exe@0x0158cba0`. When clear, the packet is fire-and-forget — used for position-update spam and unreliable bundle flushes. The exact send-window slot count is not enumerated in the current V5 evidence (the `UnAckedHandler` hash-table region at `ChannelInternal+0x40`/`+0x44` per `mercury-protocol-internals.md` §"Channel Internal Layout" is the structure that backs the window, but the doc does not pin a literal slot count) — see Q5 in §1.16.

### 1.3 Footer

The footer is the variable-width trailing region that carries reliability state, sequence ordering, and fragment boundaries. It is *parsed backward* from the end of the datagram — `processFilteredPacket_inner` calls a sequence of `buf.pop()` operations starting from the tail, each one shrinking the buffer that the next field is popped from.

**Wire order** (top = closer to message body; bottom = end of datagram). The sender writes these fields in this order — bit 0 first, bit 7 last — so the highest-bit footer field sits at the very end of the datagram:

```text
WIRE ORDER (top = just past body, bottom = end of datagram):

┌─────────────────────────────────────────────────────────────────┐
│ if FLAG_HAS_FIRST_REQUEST_OFFSET:  firstRequestOffset (u16 LE)  │ ← written first
│ if FLAG_HAS_PIGGYBACKS:            piggyback chain (see §1.3.2) │
│ if FLAG_HAS_ACKS:                  ack list (see §1.3.1)        │
│ if FLAG_HAS_SEQUENCE_NUMBER:       sequenceId (u32 LE)          │
│ if FLAG_IS_FRAGMENT:               lastFragmentId (u32 LE)      │
│ if FLAG_IS_FRAGMENT:               firstFragmentId (u32 LE)     │ ← written last
└─────────────────────────────────────────────────────────────────┘
                                                ^
                                                |
                                          end of datagram
```

**Pop order on receive** is the *inverse* — the receiver reads each field from the *end* of the datagram backward, so the field that was written last is popped first:

```text
POP ORDER (from end of datagram, peeling toward body):

  1.  if FLAG_IS_FRAGMENT:                pop firstFragmentId  (u32 LE)
  2.  if FLAG_IS_FRAGMENT:                pop lastFragmentId   (u32 LE)
  3.  if FLAG_HAS_SEQUENCE_NUMBER:        pop sequenceId       (u32 LE)
  4.  if FLAG_HAS_ACKS:                   pop ack list         (see §1.3.1)
  5.  if FLAG_HAS_PIGGYBACKS:             pop piggyback chain  (see §1.3.2)
  6.  if FLAG_HAS_FIRST_REQUEST_OFFSET:   pop firstRequestOffset (u16 LE)
```

A sender writes flags byte first, then writes message body, then appends each footer field in flag-bit order. A receiver reads flags byte first, then pops each footer field in reverse flag-bit order — so the field that was *appended last* is popped *first*.

**Request-chain walking** (`FLAG_HAS_FIRST_REQUEST_OFFSET` bit 0 + `FLAG_HAS_REQUESTS` bit 6). The two request-related flags work together as a header + index pair, not as orthogonal signals. Bit 6 (`FLAG_HAS_REQUESTS`) is the sender's promise "this packet contains at least one request message"; bit 0 (`FLAG_HAS_FIRST_REQUEST_OFFSET`) is the receiver's index "the first request's body starts at byte N of the message body." In practice the two bits are always set together: a packet with requests must carry the offset to find them, and a packet without requests has no offset to advertise. The Cimmeria reimplementation treats the two as an inseparable pair.

The reason both flags exist (rather than collapsing to one) is the receiver's walk pattern. Request messages form a *linked list inside the packet body*: each request's payload begins with an offset field that points (as a byte offset relative to the message body start) to the next request in the packet, with a sentinel value as the terminator. The receiver pops `firstRequestOffset` from the footer (gated on bit 0), seeks to that offset, parses the request, reads its inline next-pointer, and repeats until it hits the sentinel.

This lets the receiver process requests in priority order *without* walking the entire message body sequentially — useful when a bundle contains many entity-method calls interleaved with a handful of requests, and the request-handling code path runs separately from the entity-method dispatch path. The linked-list mechanism is stock BigWorld; SGW is presumed to inherit it because `Bundle::startMessage_request` at `ghidra://SGW.exe@0x0157adc0` exists and `FLAG_HAS_FIRST_REQUEST_OFFSET` is consumed at parse time. The exact next-pointer field width and terminator sentinel are not enumerated in `mercury-protocol-internals.md` — confidence on the per-byte chain layout is medium pending direct decompilation of the request-walk code at the receiver side.

#### 1.3.1 Ack list encoding

When `FLAG_HAS_ACKS` is set, the ack list at the tail is:

```text
[ ack[N-1]: u32 LE ]   ← popped second
[ ack[N-2]: u32 LE ]
...
[ ack[0]:   u32 LE ]
[ ackCount: u8     ]   ← popped first
```

Each `ack[i]` is the sequence ID of a previously received reliable packet. The receiver pops `ackCount` first (1 byte), then pops `ackCount × 4` bytes as the ack array. The sender side mirrors this in `UnAckedHandler::buildAndSendAckBundle` at `ghidra://SGW.exe@0x0158b2d0`: walks a 32-bit ack mask and writes each sequence ID into the bundle.

**Ack coalescing.** `ackCount` is a `u8`, so a single packet can carry at most 255 acks — a hard ceiling, not a practical one. The send path prefers to *piggyback* acks onto the next outgoing reliable bundle (whatever that bundle's primary purpose is — a game-level entity-method call, a position update, a control message) rather than emit a standalone ack-only packet, which keeps wire overhead minimal. When the send-alive timer at `ChannelInternal+0x16c` expires with pending acks but no game-level traffic to piggyback on, `UnAckedHandler::sendAckBundle2` at `ghidra://SGW.exe@0x0158bbc0` (also referred to as `UnAckedHandler::sendAckBundle` in some V5 sources — `mercury-protocol-internals.md`'s "All Mercury Functions" table omits the `2` suffix while the same doc's Session 5b additions includes it; this chapter uses the suffixed name for disambiguation against any hypothetical sibling) builds an empty bundle with the `FLAG_IS_RELIABLE` flag set — see §1.7 for the keepalive role. Per `mercury-protocol-internals.md` §"Complete Function Inventory (Session 5b Additions)" the function "creates empty bundle, sets reliable flag, sends." The keepalive bundle does not itself need `FLAG_HAS_ACKS` set — its purpose is to force a reliable round-trip so the receiver acks the empty packet — but in practice any queued acks `Bundle::finalise` finds in `UnAckedHandler`'s 32-bit ack mask will piggyback onto the keepalive bundle's footer, which is why a wire capture of a keepalive packet usually shows both `FLAG_IS_RELIABLE` and `FLAG_HAS_ACKS` set together. The 32-bit ack mask in `UnAckedHandler` lets the implementation track up to 32 unsent acks before they must be flushed; in practice, latency keeps the typical queued-ack count well below that and well below the 255-byte wire ceiling.

#### 1.3.2 Piggyback chain encoding

Piggybacks are *whole previously-sent packets* embedded in the footer area of a new outgoing packet. Format inherited from stock BigWorld 2.0.1; both ends of the protocol parse the same wire bytes. Confidence: medium — `mercury-protocol-internals.md` confirms `FLAG_HAS_PIGGYBACKS` bit 1 exists and is honored at parse time, but does not enumerate the chain wire bytes; the layout below is the stock-BW reference structure and is presumed inherited because no SGW divergence is named:

```text
[ packet[N] data    : packet[N]_len bytes ]
[ packet[N] length  : int16 LE, NEGATIVE  ]  ← terminator (~length to recover)
[ packet[N-1] data  : ...                 ]
[ packet[N-1] length: int16 LE, POSITIVE  ]
...
[ packet[0] data    : ...                 ]
[ packet[0] length  : int16 LE, POSITIVE  ]
```

The negative-length marker (encoded as `~length` — bitwise NOT of the positive length) indicates the final piggyback in the chain. Each embedded packet has its own flags byte and is processed as if it had been received independently, *except* that its own footer cannot include acks/sequence/fragments — only the body matters. SGW's client emits piggybacks under the `FLAG_HAS_PIGGYBACKS` bit; SGW's server side accepts them.

Confidence: medium for the SGW server side. The Cimmeria Rust implementation explicitly rejects piggybacks (`WARN_BAD_PACKET("Piggybacked packets are not supported")` per the existing `docs/protocol/mercury-wire-format.md`), and the SGW client does not appear to send them in observed pcaps. The format is well-documented in stock BW; whether SGW's deprecated C++ server ever generated piggybacks is a separate question for Section 3.

#### 1.3.3 Byte order

**Every multi-byte field in the SGW Mercury footer is little-endian.** Sequence IDs, ack sequence IDs, fragment IDs, first-request-offset — all little-endian. This is a direct SGW divergence from stock BigWorld, which writes the footer in network byte order via the `BW_HTONS` / `BW_HTONL` macros at `external/BigWorld-2.0.1/src/lib/network/packet.cpp`.

**Divergence:**

| Field | Stock BigWorld 2.0.1 | SGW |
|---|---|---|
| Flags | `uint16` (2 bytes), network order | `uint8` (1 byte) |
| Sequence ID | `uint32`, network order | `uint32`, little-endian |
| Ack sequence ID | `uint32`, network order | `uint32`, little-endian |
| Fragment begin/end | `uint32`, network order | `uint32`, little-endian |
| First-request-offset | `uint16`, network order | `uint16`, little-endian |
| Channel ID/version (indexed) | Present | Omitted (no indexed channels) |
| CRC32 checksum | `FLAG_HAS_CHECKSUM` available | Omitted (HMAC-MD5 supersedes) |
| Cumulative ack | `FLAG_HAS_CUMULATIVE_ACK` available | Omitted |

This is one of the most consequential SGW divergences for any reimplementation: getting the footer byte order wrong produces silent packet drops because the HMAC validation will succeed on the cipher envelope but the parsed sequence/ack values will be nonsense, and packets will appear to land out-of-window. The cross-check is straightforward — emit a packet with a known sequence ID like `0x12345678` and verify it appears on the wire as `78 56 34 12`, not `12 34 56 78`.

### 1.4 Cipher envelope

Every Mercury packet on the external (client-facing) channel is wrapped in AES-256-CBC then HMAC-MD5. The wrapping is a `MessageFilter` layered above the wire format described above: the sender builds the plaintext packet (flags + body + footer) and the cipher envelope is applied as the very last step before `sendto()`.

**Wire order: encrypt-then-MAC.**

```text
[ AES-256-CBC ciphertext (PKCS#7-padded plaintext) ]  ← variable length
[ HMAC-MD5 tag (always 16 bytes, no truncation)    ]
```

The ciphertext is produced by `CryptoPP::StreamTransformationFilter` (`ghidra://SGW.exe@0x004089b0`) over the Mercury plaintext, then passed to `CryptoPP::HashFilter` (`ghidra://SGW.exe@0x00414720`) which appends the HMAC-MD5 tag. The HMAC covers the ciphertext, not the plaintext.

**Worked example of cipher framing.** A 21-byte Mercury plaintext (e.g. a small `enableEntities` bundle: 1 flags byte + 1 msg_id byte + 8 dummy + 4 sequence ID + 4-byte ack = 18 bytes; padded toward 21 for example purposes) becomes:

```text
plaintext:        21 bytes
+PKCS#7 pad:      32 bytes  (pad to AES block boundary; pad value = 32 - 21 = 11)
AES-256-CBC →     32 bytes ciphertext  (same length as padded plaintext)
HMAC-MD5(ct) →    16 bytes tag        (appended after ciphertext)
on-wire frame:    48 bytes total: [32 B ct][16 B tag]
```

A 16-byte plaintext expands to **48 bytes too** (pads to 32 — PKCS#7 always pads, never zero-pads, so a 16-byte exact-block input gets a full 16-byte pad block appended). The cipher overhead is therefore a minimum of 17 bytes (1 byte of pad + 16-byte HMAC) and a maximum of 32 bytes (16-byte pad block + 16-byte HMAC) per packet. This is why the on-the-wire effective MTU is ~1456 bytes rather than the bare 1472 left by IP+UDP headers — the cipher envelope reserves room for itself.

**Key material — no KDF.** The 32-byte AES key and the 32-byte HMAC key are the *same buffer*. Both `PacketEncrypter::send` at `ghidra://SGW.exe@0x01603b80` and `PacketEncrypter::recv` at `ghidra://SGW.exe@0x01603fa0` read `GetCheckedArrayElement(this+0x08, 0, len)` for both the AES Rijndael key and the HMAC-MD5 key.

The key itself comes from the SOAP auth response (`SessionKey` attribute, 64-char hex string) and is decoded by the gSOAP `xsd:hexBinary` dispatcher at `ghidra://SGW.exe@0x015eb940` (case `0x26` of the type dispatcher at `ghidra://SGW.exe@0x015ed300`). The decoded 32 bytes are passed *verbatim* to the `PacketEncrypter` constructor at `ghidra://SGW.exe@0x01603a70` — no PBKDF, no salting, no SHA-style key stretching, no truncation.

**IV — literal zero, every packet.** The constructor stores 16 zero bytes at `PacketEncrypter+0x18` via `FUN_00a587f0(this+0x18, 0x10, null)` (null source → zero-filled). The IV buffer is read on every encrypt/decrypt call but is *never mutated*: the same zero IV is reused for every packet on the channel. This is a deliberate 2009 design choice; combined with PKCS#7 padding it produces a deterministic ciphertext for identical plaintexts but matches the wire-format invariant the SGW client expects.

**Library: CryptoPP, not OpenSSL.** RTTI strings at `0x01e93b70`–`0x01ea3c5c` stamp `HMAC_Base@CryptoPP`, `HMAC@VMD5@Weak1@CryptoPP`, and friends. The Cimmeria `crates/mercury/src/encryption.rs` doc-comment that mentions "OpenSSL" is incorrect (the runtime uses RustCrypto, not OpenSSL either, but the binary it's emulating uses CryptoPP). The HMAC algorithm is the MD5 variant tagged as `Weak1` in CryptoPP's namespace — a 2009 design choice; modern code would not pair MD5 with HMAC.

**Cipher object layout.**

| Offset | Size | Field | Notes |
|---|---|---|---|
| `+0x00` | 4 | vtable | `0x01b27374` |
| `+0x04` | 4 | ref_count | `SafeReferenceCount` base |
| `+0x08` | var | `key_buf` | `std::vector`-like ptr/end/capacity; holds the 32-byte key (AES + HMAC) |
| `+0x18` | var | `iv_buf` | `std::vector`-like ptr/end/capacity; holds 16 zero bytes (re-read every packet) |

The vtable is stamped at `0x01b27374` with four slots:

| Slot | Address | Role |
|---|---|---|
| 0 | `0x01604ac0` | Destructor |
| 1 | `0x01603b80` | `send` — encrypt outgoing packet |
| 2 | `0x01603fa0` | `recv` — decrypt incoming packet |
| 3 | `0x016039a0` | Returns `0x1f` (31) — likely `OptimalBlockSize` |

**Divergence from stock BigWorld 2.0.1.** Stock BW uses Blowfish ECB with XOR chaining, 8-byte blocks, a `0xdeadbeef` magic prefix, and a wastage byte. None of that applies to SGW. The SGW cipher chain is a wholesale replacement, not a parameter tweak. The stock BW encryption code in `external/BigWorld-2.0.1/src/lib/network/encryption_filter.cpp` is irrelevant for SGW emulation.

### 1.5 InterfaceElement length encoding

A Mercury bundle is a sequence of *interface element calls*. Each call is one entry of the form `[msg_id: u8][length-prefix][payload]`, where the length-prefix encoding is determined by the `InterfaceElement` registered for that `msg_id`. Three length formats exist:

| Format name | Length field width | When |
|---|---|---|
| `CONSTANT_LENGTH` | 0 bytes (implicit) | Fixed-size payload known from the message table |
| `WORD_LENGTH` | 2 bytes (`u16` LE) | Variable-size payload, typical entity-method call (and `REPLY_MESSAGE 0xFF` per `space-viewport-wire-formats.md`) |
| `DWORD_LENGTH` | 4 bytes (`u32` LE) | Variable-size payload; the only V5-confirmed user is `AUTHENTICATE` (msg_id `0x00`, see §1.10.7) |

The `InterfaceElement` table is a static array of fixed-size descriptor entries. At runtime, the Nub builds a parallel array of smaller runtime entries indexed directly by `msg_id`, populated from the static array. The runtime entries are read by `Mercury::Nub::processOrderedPacket` at `ghidra://SGW.exe@0x0157c820` on every incoming message. **Confidence: medium** on the exact byte sizes of the static descriptor and the runtime form — the chapter previously cited `0x90` (144 bytes) and `0x24` (36 bytes) but neither size is enumerated in the current V5 evidence; the sizes are inherited from stock BigWorld 2.0.1 (`external/BigWorld-2.0.1/src/lib/network/interfaces.hpp`) where they are documented. A direct Ghidra read of the static descriptor allocation site would pin SGW's sizes; until then, treat the numeric values as inherited from stock BW.

**Static vs runtime layout, side by side.** The static `InterfaceElement` entries carry the full message descriptor — name string, length type, payload-size hint, handler pointer, reliability flag, encryption-required flag, and assorted metadata. At Nub initialization, the static entries are *projected* into a smaller runtime form keyed by `msg_id`: only the runtime-hot fields are kept (`lengthType`, `lengthValue`, `handler*`, `isEntityMessage` flag). The runtime array's index is the `msg_id` byte itself, so a dispatch is a single `nub->elements[msg_id]` load — no name-based lookup, no hash. The 256 `msg_id` slots map: 0x00–0x7F to system-message slots, 0x80–0xFD to entity-method slots (with `0xBD` and `0xFD` reserved as the sub-slot extended-encoding sentinels), and `0xFF` to the reply-message slot. The runtime form's per-entry size is the same inherited-from-stock-BW value flagged above — not independently confirmed for SGW.

**Entity messages override the table.** Any message with `msg_id >= 0x80` is an entity-method or property message and *always* uses `WORD_LENGTH`, regardless of the table's declared length type for that ID. This is enforced in `BundleUnpacker::next` (decode side) and in `Mercury::Bundle::newMessage` at `ghidra://SGW.exe@0x0157ac90` (encode side). The reason: entity messages carry their own variable-size argument list whose total size cannot be known statically.

**Compressed-length encoding for interface elements with extreme size variation.** A separate variable-width scheme exists for the rare case where a message's payload size is usually small (fits in 1 byte) but must occasionally extend to a wider field. The `InterfaceElement::compressLength` family handles the switch:

| Function | Address | Role |
|---|---|---|
| `InterfaceElement::compressLength` | `ghidra://SGW.exe@0x0158acc0` | Decide compressed-length width from value |
| `InterfaceElement::expandLength` | `ghidra://SGW.exe@0x0158b770` | Read compressed-length field at parse time |
| `InterfaceElement::compressLength_write` | `ghidra://SGW.exe@0x0158b120` | Write compressed-length field at emit time |

**Confidence: medium.** V5 confirms the four width options (1-byte, 2-byte, 3-byte, 4-byte) via `mercury-protocol-internals.md` §"All Mercury Functions" — `InterfaceElement::compressLength_write` is described as "Write length (1/2/3/4 byte)". What V5 does *not* enumerate is the threshold byte values that decide which width to emit for a given length value. The thresholds are the open piece (Q1 in §1.15), not the existence of the four-width scheme. The closest comparable scheme in the same binary is `ProcessMessage::writeComponentsVarLen` at `ghidra://SGW.exe@0x01586180` (the MachineGuard component-ID encoder), which uses a single threshold: IDs `≤ 0xfe` are written as 1 byte; IDs `> 0xfe` are written as `0xff` prefix + 3 bytes. The InterfaceElement scheme *may* be similar, but the bible canonizes evidence not analogy — pin to `medium` until `0x0158b120` is decompiled and the threshold constants are extracted. See open Q1 in §1.15.

Note that compressed-length encoding is *not* what entity messages use — entity messages always use `WORD_LENGTH` (the fixed 2-byte `u16` prefix). The compressed scheme is for system messages whose maximum-size envelope is large but whose typical-case size is small.

### 1.6 Mercury bundle

A *bundle* is the logical unit of reliability and the container for one or more interface-element messages. A bundle can span multiple packets via fragmentation; a packet always belongs to exactly one bundle.

**Bundle construction.** `Mercury::Bundle::Bundle` at `ghidra://SGW.exe@0x0157aa40` constructs an empty bundle. `Mercury::Bundle::clear` at `ghidra://SGW.exe@0x0157a440` resets state and allocates a fresh first packet. Messages are added via three entry points:

| Entry point | Address | Role |
|---|---|---|
| `Bundle::newMessage` | `ghidra://SGW.exe@0x0157ac90` | Start new message — writes `msg_id`, computes header size, allocates new packet if needed |
| `Bundle::startMessage_fixed` | `ghidra://SGW.exe@0x0157ad80` | Fixed-length message wrapper |
| `Bundle::startMessage_request` | `ghidra://SGW.exe@0x0157adc0` | Request message — reserves space for the reply-ID + next-request-offset linked-list pointers |

After the header is written, `Bundle::addBlob` at `ghidra://SGW.exe@0x0157a990` copies payload bytes. When the current packet is full, `addBlob` auto-splits across packet boundaries, advancing to the next packet in the bundle's packet chain. The packet chain is the same `Mercury::Packet` linked-list traversed by `Packet::chain__stampSendTime` at `ghidra://SGW.exe@0x0158a3f0`.

**Finalization.** `Mercury::Bundle::finalise` at `ghidra://SGW.exe@0x0157a7a0` walks the packet chain one final time: each packet's flags byte is updated to reflect what footer fields will be appended (sets `FLAG_HAS_SEQUENCE_NUMBER`, `FLAG_HAS_ACKS` if there are queued acks, `FLAG_IS_FRAGMENT` if the bundle spans more than one packet, etc.), and the footer fields are written in flag-bit order at the end of each packet. After `finalise`, the bundle is ready to be handed to `Mercury::Nub::send` at `ghidra://SGW.exe@0x01582160`.

**Fragmentation invariants.**

- Maximum packets per bundle: **64** (`Packet::MaxFragmentsPerBundle` from stock BW; matches SGW observed behavior).
- Each fragment carries `FLAG_IS_FRAGMENT` (bit 7) in its flags byte and two `uint32` fragment IDs in its footer: `firstFragmentId` (the sequence ID of the first packet in the bundle) and `lastFragmentId` (the sequence ID of the last packet). Both fragment IDs are identical across every fragment in the bundle — they describe the bundle's bounds, not the fragment's index.
- A fragment's own position in the bundle is derived from `sequenceId - firstFragmentId`.
- The receiver allocates a vector of `(lastFragmentId - firstFragmentId + 1)` slots when the first fragment arrives and fills slots by sequence ID. The bundle is reassembled when every slot is non-null (`BundleUnpacker::isComplete`).

**Send window.** A reliable bundle's fragments each occupy one slot in the channel's fixed-size send window of in-flight reliable packets. A bundle whose fragment count exceeds the window size cannot complete without window stalling. In practice, bundles are tens of packets at most — the largest observed is the world-entry mapLoaded bundle at 27+ interface-element calls, which fits in ~5 packets, well under any plausible window size. The exact slot count is not enumerated in the current V5 evidence (see §1.2's `FLAG_IS_RELIABLE` paragraph and open question Q5 in §1.16); pin the literal count to medium confidence pending direct extraction of the window capacity constant from `ChannelInternal`'s constructor or from `UnAckedHandler::buildAndSendAckBundle` at `0x0158b2d0`.

### 1.7 Sequence numbers and reliability

Mercury sequence numbers are **28-bit** (`SEQ_SIZE = 0x10000000`). The space is 256M sequence IDs before wrap; the wrap is handled by modular arithmetic in the comparison routines. A reliable packet's sequence ID is assigned by `Mercury::Channel::send` at `ghidra://SGW.exe@0x01576f90` from a monotonic per-channel counter.

| Constant | Value | Source | Confidence |
|---|---|---|---|
| Sequence number mask | `0x0FFFFFFF` | `mercury-protocol-internals.md` §"Protocol Constants" | high |
| Null sequence number | `0x10000000` | `mercury-protocol-internals.md` §"Protocol Constants" | high |
| Max retries | 20 | Inherited from stock BigWorld 2.0.1; SGW divergence in this area not enumerated in V5 evidence; pcap verification of actual SGW retry cadence is a future task | medium |
| Ack timeout | 700 ms | Inherited from stock BigWorld 2.0.1; SGW divergence in this area not enumerated in V5 evidence; pcap verification of actual SGW timeout cadence is a future task | medium |

`0x10000000` is the null-sentinel: a packet with this sequence ID has no sequence number assigned (used for unreliable bundles that don't go in the send window). Because `0x10000000` is the very next value above the 28-bit `0x0FFFFFFF` mask, no real sequence number can collide with the sentinel.

**Reliability state lives in `ChannelInternal`**, the ~0x180-byte inner channel object constructed at `ghidra://SGW.exe@0x0158c7b0`. The send window is a fixed-size circular buffer; entries are cleared by `processAck` when their sequence ID is acknowledged. The window head slides forward only when its head slot is empty (acked or never used). A receiver's processing of incoming acks runs even when the incoming packet's own sequence ID is outside its receive window — this prevents lost acks from causing unbounded retransmissions.

**Resend timing.** `ChannelInternal::checkAndSendNubException` at `ghidra://SGW.exe@0x0158bed0` runs the timer-driven resend logic. Three rdtsc-based timeout fields live in the channel object:

| Offset | Role |
|---|---|
| `+0x160` | Receive timeout threshold (rdtsc units) |
| `+0x164` | Receive timeout last-check timestamp |
| `+0x16c` | Send-alive timeout — triggers a keepalive ack bundle if no traffic |
| `+0x170`, `+0x174` | Additional timer fields (role TBD per `mercury-protocol-internals.md` Session 5b open question 1 — see Q2 in §1.16) |

When the send-alive timer expires, `UnAckedHandler::sendAckBundle2` at `ghidra://SGW.exe@0x0158bbc0` builds an empty bundle with the reliable flag set, just to keep the channel alive. This is the Mercury keepalive — not a separate keepalive packet type.

### 1.8 Message dispatch

After packet reassembly, each interface element message in the bundle is dispatched by `Mercury::Nub::processOrderedPacket` at `ghidra://SGW.exe@0x0157c820`. The dispatch is a single lookup against the runtime `InterfaceElement` array:

```text
InterfaceElement* elem = &nub->elements[msg_id];   // single array index by msg_id byte
elem->handler->handleMessage(msg);
```

The runtime entry size is medium confidence — see §1.5; the array-indexed-by-`msg_id`-byte dispatch shape is what V5 confirms, not the literal byte stride. A reimplementation can lay out the runtime array however it likes as long as `nub.elements[msg_id]` resolves to the right descriptor in O(1).

Three classes of message ID exist:

| Range | Class | Length encoding | Wire shape |
|---|---|---|---|
| `0x00 – 0x7F` | System messages (auth, sync, control) | Per-table (`CONSTANT_LENGTH` / `WORD_LENGTH` / `DWORD_LENGTH`) | `[msg_id][length?][payload]` |
| `0x80 – 0xBD` | Cell entity method calls (and `0xBD` extended) | Always `WORD_LENGTH` | `[msg_id][u16 length][u32 entityId][args]` |
| `0xC0 – 0xFD` | Base (proxy) entity method calls (and `0xFD` extended) | Always `WORD_LENGTH` | `[msg_id][u16 length][args]` (no entityId — see callout) |
| `0xFF` | Reply message | `WORD_LENGTH` | `[0xFF][u16 length][reply data]` |

**Cell vs base: only cell methods carry an entity ID on the wire.** Per `system-protocol-wire-formats.md` §"startEntityMessage / startProxyMessage", the client emits these two call shapes from distinct code paths:

- `ServerConnection_startEntityMessage` at `ghidra://SGW.exe@0x00dd6a60` (cell, `msg_id | 0x80`): writes the `msg_id`, then `*(uint32*)channel->reserve(4) = entityId` — the 4-byte entity ID lands on the wire as the first bytes of the message body.
- `ServerConnection_startProxyMessage` at `ghidra://SGW.exe@0x00dd6980` (base / proxy, `msg_id | 0xC0`): writes the `msg_id`, then **does not write an entity ID**. Base methods implicitly target the player's own base entity — the channel binds 1:1 to that entity, so there is nothing to disambiguate.

Cell entities can be many-per-connection (the player, vehicles, NPCs in AoI for client-controlled methods), so cell methods must name their target. Base entities are one-per-connection (the player's own base proxy), so the entity ID is redundant. Getting this wrong silently corrupts the first 4 bytes of any base-method argument list.

Entity messages: the first method's `msg_id` byte encodes the method index directly (`methodId | 0x80` for cell, `methodId | 0xC0` for base). For method indices `≥ 62` (`0x3E`), the encoding switches to *extended*: the `msg_id` byte is the sentinel `0xBD` (cell) or `0xFD` (base), and an extra `u8` carrying `sub_index = methodId - 62` follows the `entityId` field (cell) or the message header (base). Method index 62 is the first index that uses extended encoding (`msg_id = 0xBD` on the wire, `sub_index = 0` in the body). The disambiguation of `msg_id = 0xBD` between "direct method index 61" and "extended sentinel for index ≥ 62" is resolved at compile time by `EntityDescription_AssignClientMethodIds` based on the entity's total method count — if the entity has fewer than 62 methods total, the parser treats `0xBD` as a direct method index; if it has 62 or more, `0xBD` is unconditionally the sentinel and the next byte is `sub_index`.

The threshold = 62 claim is V5-confirmed against `entity-property-sync.md` §13 "Sub-Slot Client Method Encoding — Final Confirmation (W-entity-desc-B)": `EntityDescription_AssignClientMethodIds` at `ghidra://SGW.exe@0x01590df0` switches to sub-slot encoding when `methodCount >= 0x3e` (62). The same threshold appears in BigWorld 2.0.1's `entity_method_descriptions.cpp::checkExposedForSubSlots()`. The full sub-slot mechanism is canonized in `spec.engine.entity-description-parse-chain`; this chapter only canonizes the wire shape.

**Worked example — direct cell-method dispatch.** A call to `onStatUpdate` (cell method index 20) on entity ID `0xCAFEBABE` with 3 bytes of arguments:

```text
[0x94]                  ← msg_id = 20 | 0x80 = 0x94 (direct encoding, cell)
[0x07 0x00]             ← word_len = 7 (u16 LE)  — 4 bytes entityId + 3 bytes args
[0xBE 0xBA 0xFE 0xCA]   ← entityId = 0xCAFEBABE (u32 LE)
[arg0 arg1 arg2]        ← serialized args
```

**Worked example — extended cell-method dispatch.** A call to `onClientMapLoad` (cell method index 117 — above the 62 threshold) on the same entity:

```text
[0xBD]                  ← msg_id = 0xBD (extended-encoding sentinel for cell)
[len_lo len_hi]         ← word_len = 4 (entityId) + 1 (sub_index) + N (args)  — u16 LE
[0xBE 0xBA 0xFE 0xCA]   ← entityId = 0xCAFEBABE (u32 LE)
[0x37]                  ← sub_index = 117 - 62 = 55 (u8)
[args...]               ← serialized args
```

> [!NOTE] **Source-doc override.** `docs/reverse-engineering/findings/world-entry-pipeline.md` §"onClientMapLoad" tabulates `sub_index: u8 = 56` for method index 117 (computing `117 - 61`), which is off-by-one. The threshold is `0x3e = 62`, not 61 — confirmed by `entity-property-sync.md` §13 and by `external/BigWorld-2.0.1/src/.../entity_method_descriptions.cpp::checkExposedForSubSlots()`. The correct sub_index for method 117 is `117 - 62 = 55 = 0x37`, as the worked example above shows. The `world-entry-pipeline.md` value is a known transcription error inherited from an earlier (pre-W-entity-desc-B) draft and should be corrected when that doc is next revised.

**Worked example — direct base-method dispatch.** A call to `playCharacter` (base method index 4) with 3 bytes of arguments. Note the absence of an entity ID — base methods target the player's own base proxy implicitly:

```text
[0xC4]                  ← msg_id = 4 | 0xC0 = 0xC4 (direct encoding, base)
[0x03 0x00]             ← word_len = 3 (u16 LE)  — 3 bytes of args, no entityId
[arg0 arg1 arg2]        ← serialized args
```

The extended encoding costs 1 extra byte per call (the sub_index byte) and is required for any method whose index is 62 or higher. Roughly 96 of the 157 client methods on `SGWPlayer` use extended encoding because the parsed order pushes most actual gameplay methods past the threshold.

**Reply messages** use `msg_id = 0xFF` (`REPLY_MESSAGE_IDENTIFIER`) with `WORD_LENGTH` (2-byte length prefix per `space-viewport-wire-formats.md` §"REPLY_MESSAGE (0xFF)" and §"Complete Server Message Table"). The body is the connection-handshake reply payload — V5 marks this as a Mercury protocol-level message used during the initial connection handshake, not during normal gameplay. Matching of in-game request/reply pairs travels through `Mercury::Nub::handleMessage` at `ghidra://SGW.exe@0x0157bd30`, but the V5 record does not enumerate a separate `replyId` field in the reply body — earlier drafts of this chapter assumed a stock-BW-style `[u32 length][u32 replyId]` shape that is not in the SGW evidence.

### 1.9 Control messages

A small set of system messages drives the connection's lifecycle. Each has a fixed `InterfaceElement` descriptor registered at static-init time and a binding to a specific server-side handler.

#### `enableEntities` (base method index 1, client → server)

`enableEntities` lives in this "Control messages" section because of its role in the world-entry handshake, but it is technically a base entity method by msg_id (`0xC1`, in the `0xC0`–`0xFD` range from §1.8's dispatch table). The generic base-method wire shape from §1.8 is `[msg_id][u16 word_len][args]` (no entity-ID prefix); `enableEntities` further specializes that via an `InterfaceElement` table override that pins it to `CONSTANT_LENGTH = 8` instead of `WORD_LENGTH`. Both `resetEntities` (msg_id `0x04`, a true system-range message) and `enableEntities` are reachable via the same logical world-entry handshake; their msg_ids land in different ranges as an artifact of which side originates each direction.

| Property | Value |
|---|---|
| Message ID | `0xC1` — *derived* from `1 \| 0xC0` per §1.8's base-method encoding rule. The derivation is convention-consistent (base method index 1 with the `0xC0` high-bit set) but is not independently wire-observed in current V5 evidence. Confidence: medium on the literal byte value; high on the method index (1) and the encoding rule. |
| Message size | **8 bytes** (`CONSTANT_LENGTH = 8`) |
| Payload | 8 bytes reserved by `startMessage_fixed`; the client does not appear to explicitly zero the buffer in V5 evidence. The server discards the payload contents (only the message arrival matters), so whether the bytes are zero, uninitialized, or stale bundle-allocator memory is not behaviorally observable from the wire. Confidence: medium on the payload-is-zero claim — earlier drafts named the field `uint64 dummy = 0`, but `world-entry-pipeline.md` only confirms that `BroadcastEntityActivation` calls `startMessage_fixed` and reserves `DAT_01ef2500->size` bytes in the bundle; no V5 evidence shows the client explicitly zeroing the reserved buffer. |
| Descriptor address | `DAT_01ef2500` |
| Initializer site | `ghidra://SGW.exe@0x017bade0`–`0x017bae07` |
| Initializer `PUSH` for size | `ghidra://SGW.exe@0x017bade9` (`PUSH 0x8`) |
| Sender (in client) | `ServerConnection::enableEntities` / `BroadcastEntityActivation` at `ghidra://SGW.exe@0x00dd9280` |
| Sets flag | `bEntitiesEnabled` at `ServerConnection+0x316` |

This is the client→server signal that completes the world-entry handshake — the client tells the server "I've reset my entity state, start streaming entity creates." See `spec.world.world-entry` for the full RESET → ENABLE handshake.

**Divergence:** stock BigWorld 2.0.1's `enableEntities` carries 1 byte (`uint8 dummy`) per `external/BigWorld-2.0.1/src/lib/connection/baseapp_ext_interface.hpp`:

```cpp
MF_BEGIN_BLOCKABLE_PROXY_MSG( enableEntities )
    uint8   dummy;
END_STRUCT_MESSAGE();
```

SGW's `enableEntities` carries 8 bytes (`uint64 dummy`) per `deprecated/cpp/src/baseapp/mercury/sgw/messages.cpp:83`:

```cpp
{Message::CONSTANT_LENGTH, 8, "ENABLE_ENTITIES", true},
```

**This is the most-contested wire-format claim in the project's RE history.** A pre-V5 finding by W-misc-gaps initially concluded 1 byte by misreading the descriptor initializer (mistaking the `MOV DWORD PTR [EAX], 0x1` at `0x017badf7` — which writes a reliability flag into the stack-allocated argument struct — for the size field). W-enable-entities (2026-05-13) re-examined the disassembly context and confirmed the size field is the `PUSH 0x8` three instructions earlier at `0x017bade9`. The calibration check is to compare against the `resetEntities` initializer at `ghidra://SGW.exe@0x017bb200`–`0x017bb225`, which uses the identical push pattern with `PUSH 0x1` at the same stack position — and `resetEntities` is documented and confirmed as 1-byte `keepBase`. The 8-byte SGW custom size is canon.

#### `resetEntities` (system message, server → client)

| Property | Value |
|---|---|
| Message ID | `0x04` |
| Length type | `CONSTANT_LENGTH = 1` |
| Payload | `uint8 keepBase` |
| Descriptor table site | `0x017bb210` (registration site) |
| Initializer | `ghidra://SGW.exe@0x017bb200`–`0x017bb225` |
| Handler in client | `PurgeAndRebuildEntityStateLists` at `ghidra://SGW.exe@0x00dda0e0` |

The server sends `resetEntities` to clear the client's entity-state lists; the client responds by clearing four linked-list sentinels at offsets `+0xF88`, `+0xF94`, `+0xFA0`, `+0xFB0` of its `ServerConnection` object, then auto-emits `enableEntities` via `BroadcastEntityActivation`. This RESET → ENABLE round-trip is the wire-level boundary between world-entry phase 5 and phase 6 in `spec.world.world-entry`.

> [!NOTE] **Source-doc handler-name disagreement.** `docs/reverse-engineering/findings/system-protocol-wire-formats.md` §"RESET_ENTITIES (0x04)" calls the handler at `0x00dda0e0` `Mercury__unknown_00dda0e0` — its raw decompile name. `docs/reverse-engineering/findings/world-entry-pipeline.md` calls the same function `PurgeAndRebuildEntityStateLists`, which is the role-derived name used in this chapter. Both names refer to the same Ghidra function at the same address; this chapter uses the role name because it conveys what the function does, but reviewers reading the system-protocol doc should know `Mercury__unknown_00dda0e0` and `PurgeAndRebuildEntityStateLists` are aliases for the same handler.

**Bundle-level constraint.** Per `entity-creation-wire-formats.md` §"1. RESET_ENTITIES (0x04)" and the cited C++ pattern (`bundle.beginMessage(BASEMSG_RESET_ENTITIES, Bundle::FLUSH); bundle << (uint8_t)0;`), `RESET_ENTITIES` must be sent in its own flushed bundle — the server explicitly flushes the current bundle before writing this message and flushes again immediately after, so the packet that carries `RESET_ENTITIES` carries no other messages. This is a wire-visible constraint: a packet containing `RESET_ENTITIES` should always have exactly that one interface element in its body.

#### `RESOURCE_FRAGMENT` (system message, msg_id `0x36`)

Streams cooked-data resources (PAK fragments) from server to client. Each fragment carries one chunk of a resource that the client reassembles before passing the whole resource to the cooked-data pipeline. Full byte-level layout is canonized in `space-viewport-wire-formats.md` §"RESOURCE_FRAGMENT (0x36)".

| Property | Value |
|---|---|
| Message ID | `0x36` |
| Length type | `WORD_LENGTH` (`u16` LE length prefix) |
| Handler in client | `ServerConnection_resourceFragment` at `ghidra://SGW.exe@0x00dddd80` |
| Max fragment body | 1000 bytes (`FragmentSize` constant) |
| Delivery CME event | `Event_Net_ProxyData` (callback ctor at `ghidra://SGW.exe@0x004269f0`) |

**Header — present on every fragment** (4 bytes):

```text
[dataId:  u16 LE]      2 bytes — per-resource transfer ID (increments per sendResource call)
[chunkId: u8]          1 byte  — fragment sequence number (0, 1, 2, …)
[flags:   u8]          1 byte  — bitfield (see below)
```

**Flags byte bits** (from `space-viewport-wire-formats.md`):

| Bit | Mask | Name | Meaning |
|---:|---|---|---|
| 0 | `0x01` | `INITIAL_FRAGMENT` | First fragment of a resource |
| 1 | `0x02` | `FINAL_FRAGMENT` | Last fragment of a resource |
| 6 | `0x40` | `BASE_FLAG` | Always set in observed traffic (selects the fragment-reassembly path inside the handler — see below) |
| 7 | `0x80` | `ERROR_FLAG` | Resource error (client sets status `0xFF`) |

**Body — first fragment only** (after the 4-byte header):

```text
[messageType: u8 = 0]   1 byte — always 0 (MESSAGE_CacheData)
[categoryId:  u32 LE]   4 bytes — resource category index (see below)
[elementId:   u32 LE]   4 bytes — resource identifier (e.g. item def ID)
[xmlBody:     bytes]    var    — start of XML document
```

**Body — subsequent fragments** (after the 4-byte header):

```text
[xmlBody: bytes]        var    — continuation of XML document
```

**Two code paths** in the client handler at `0x00dddd80`:

1. **With `BASE_FLAG` (0x40) set** — fragment reassembly path. The client allocates fragment nodes (each `11 + bodySize` bytes), chains them in receive order per `dataId`, and triggers reassembly when `FINAL_FRAGMENT` arrives. Reassembled bytes are concatenated in reverse order (the linked list builds head-first) and delivered to the resource handler via `this+0x168 vtable[0x38]`.
2. **Without `BASE_FLAG`** — direct delivery path. Uses semaphore-based synchronization, writes fragment bytes to a `FILE` handle, releases the semaphore on completion.

**Resource category IDs** (`categoryId` field, first-fragment body) — 21 categories enumerated by `space-viewport-wire-formats.md` §"Resource Category IDs":

| ID | Category | ID | Category | ID | Category |
|---:|---|---:|---|---:|---|
| 1 | `kismet_event_sequence` | 8 | `interaction_set_map` | 15 | `blueprint` |
| 2 | `ability` | 9 | `effect` | 16 | `applied_science` |
| 3 | `mission` | 10 | `text` | 17 | `discipline` |
| 4 | `item` | 11 | `error_text` | 18 | `racial_paradigm` |
| 5 | `dialog` | 12 | `world_info` | 19 | `special_words` |
| 6 | `kismet_event_set` | 13 | `stargate` | 20 | `interaction` |
| 7 | `char_creation` | 14 | `container` | | |

(ID 0 is reserved.) Confidence: high — V5-confirmed via Ghidra decompilation of the handler at `0x00dddd80` plus the category-ID enumeration in `space-viewport-wire-formats.md`.

#### Reply messages (`msg_id = 0xFF`)

```text
[0xFF: u8] [length: u16 LE] [reply data: bytes]
```

Per `space-viewport-wire-formats.md` §"REPLY_MESSAGE (0xFF)" and the §"Complete Server Message Table" entry, `REPLY_MESSAGE` is `WORD_LENGTH` (2-byte `u16` length prefix), not `DWORD_LENGTH`. The V5 doc characterizes the message as "a Mercury protocol-level message used during the initial connection handshake, not during normal gameplay." No separate `replyId` field is documented inside the reply body for SGW — the request/reply pairing for stock BigWorld's `BW_HTONL`-encoded reply IDs is not surfaced as a distinct field in the V5 evidence.

The next-request-offset linked-list in the request packet's footer (`FLAG_HAS_FIRST_REQUEST_OFFSET` at bit 0, `firstRequestOffset` field in the footer) lets the receiver walk all request messages in a packet without having to look at message bodies. Matching of those requests with their replies travels through `Mercury::Nub::handleMessage` at `ghidra://SGW.exe@0x0157bd30`; the exact wire shape of in-game (post-handshake) reply bodies — if any are emitted at all in SGW's running protocol — is not enumerated in the current V5 record.

#### 1.9.1 `bandwidthNotification` (server → client, msg_id `0x01`)

The advertised maximum bandwidth from server to client. SGW does not consume the value — there is no bandwidth mutator in the SGW client — but the message is still emitted by `messages.cpp` and decoded by the client, so reimplementations must produce a byte-correct packet to match the registered descriptor.

| Property | Value |
|---|---|
| Message ID | `0x01` |
| Length type | `CONSTANT_LENGTH = 4` |
| Payload size | 4 bytes (no length prefix on the wire — fixed) |
| Handler in client | `ClientMessageHandler<bandwidthNotificationArgs>` (RTTI at `0x01e52088` per `space-viewport-wire-formats.md` §"BANDWIDTH_NOTIFICATION (0x01)") |
| Trigger (server) | During connection setup, before entity-system init |
| Notable behavior | Not used by SGW — the client has no bandwidth mutator wired up |

**Wire layout** per `space-viewport-wire-formats.md` §"BANDWIDTH_NOTIFICATION (0x01)":

```text
[msg_id:    0x01]    1 byte
[bandwidth: u32 LE]  4 bytes  — max bandwidth in bps (server source: `messages.cpp:134` writes a single u32)
```

The server's emit site at `messages.cpp:134` is a single `bundle << (uint32_t)bandwidth;` write. SGW carries the message through the dispatch table for parity with stock BigWorld but the templated handler at RTTI `0x01e52088` has no game-layer side effect — the value is read off the wire and discarded.

Confidence: high for the wire layout and length type; high for the "not used by SGW" claim per `space-viewport-wire-formats.md` §"BANDWIDTH_NOTIFICATION (0x01)" which states "Not used by SGW (no bandwidth mutator)".

#### 1.9.2 `updateFrequencyNotification` (server → client, msg_id `0x02`)

The server's tick resolution advertised to the client at connection setup. The single byte encodes the server tick rate as ticks-per-second (typically 10 for a 100ms tick interval). Sent once per connection, first message after the connection is established.

| Property | Value |
|---|---|
| Message ID | `0x02` |
| Length type | `CONSTANT_LENGTH = 1` |
| Payload size | 1 byte (no length prefix on the wire — fixed) |
| Handler in client | `ClientMessageHandler<updateFrequencyNotificationArgs>` (RTTI at `0x01e520e0` per `space-viewport-wire-formats.md` §"UPDATE_FREQUENCY_NOTIFICATION (0x02)") |
| Trigger (server) | First message after connection setup |

**Wire layout** per `space-viewport-wire-formats.md` §"UPDATE_FREQUENCY_NOTIFICATION (0x02)":

```text
[msg_id:     0x02]   1 byte
[resolution: u8]     1 byte  — ticks per second (typically `1000 / tickRate = 10`)
```

The server's emit site at `client_handler.cpp:46-53` computes `uint8_t updateFreq = (1000 / CellManager::instance().tickRate());` and writes the result as a single byte. The client uses this to derive the size of the game-time delta carried by `TICK_SYNC` (§1.9.4) and `SET_GAME_TIME` (§1.9.3).

Confidence: high for the wire layout, length type, and tick-rate derivation (templated handler + explicit C++ emit source).

#### 1.9.3 `setGameTime` (server → client, msg_id `0x03`)

The current game-time tick counter, sent during the connection setup sequence (immediately after `TICK_SYNC` per `space-viewport-wire-formats.md`). The client snaps its local game clock to the advertised value so that subsequent `TICK_SYNC` deltas resolve correctly.

| Property | Value |
|---|---|
| Message ID | `0x03` |
| Length type | `CONSTANT_LENGTH = 4` |
| Payload size | 4 bytes (no length prefix on the wire — fixed) |
| Handler in client | `ClientMessageHandler<setGameTimeArgs>` (RTTI at `0x01e52138`; templated message — see `system-protocol-wire-formats.md` §"TICK_SYNC (0x0D) and SET_GAME_TIME (0x03) -- RTTI Evidence") |
| Descriptor table site | `0x017bb180` (registration site for `setGameTime` per `system-protocol-wire-formats.md` §"Registration Name Strings") |
| Trigger (server) | During connection setup, immediately after `TICK_SYNC` |

**Wire layout** per `space-viewport-wire-formats.md` §"SET_GAME_TIME (0x03)":

```text
[msg_id:   0x03]      1 byte
[gameTime: u32 LE]    4 bytes  — current game time in ticks (resolution set by `updateFrequencyNotification`, §1.9.2)
```

The server's emit site at `client_handler.cpp:61-63` is a single `bundle << (uint32_t)ticks;` write. Because SGW uses templated `ClientMessageHandler<setGameTimeArgs>` rather than a standalone named handler function, there is no Ghidra anchor for a handler-side decode — the dispatch is inlined and the arg struct is a direct memcpy of the 4 payload bytes.

Confidence: high for the wire layout and length type; high for the "immediately after `TICK_SYNC`" ordering per `space-viewport-wire-formats.md`.

#### 1.9.4 `tickSync` (server → client, msg_id `0x0D`)

Heartbeat sent at the configured tick rate (10 Hz default — every 100ms). Carries the current game-tick counter plus the tick interval in milliseconds, keeping the client's clock in sync with the server's tick scheduler. The message can be emitted on the unreliable channel if the server's `unreliable_tick_sync` config flag is set.

| Property | Value |
|---|---|
| Message ID | `0x0D` |
| Length type | `CONSTANT_LENGTH = 8` |
| Payload size | 8 bytes (no length prefix on the wire — fixed) |
| Handler in client | `ClientMessageHandler<tickSyncArgs>` (RTTI at `0x01e52270`; templated — see `system-protocol-wire-formats.md` §"TICK_SYNC (0x0D) and SET_GAME_TIME (0x03) -- RTTI Evidence") |
| Descriptor table site | `0x017bb720` (registration site for `tickSync` per `system-protocol-wire-formats.md` §"Registration Name Strings") |
| Trigger (server) | Every game tick (10 Hz default); also sent on the unreliable channel if `unreliable_tick_sync = true` |

**Wire layout** per `entity-creation-wire-formats.md` §"8. TICK_SYNC (0x0D)" and `space-viewport-wire-formats.md` §"TICK_SYNC (0x0D)":

```text
[msg_id:   0x0D]     1 byte
[gameTime: u32 LE]   4 bytes  — current game tick counter
[tickRate: u32 LE]   4 bytes  — tick interval in milliseconds (typically 100 = 10 Hz)
```

The server's emit site at `client_handler.cpp:486-488`:

```cpp
bundle.beginMessage(BASEMSG_TICK_SYNC);
bundle << (uint32_t)time << (uint32_t)CellManager::instance().tickRate();
bundle.endMessage();
```

The client uses the advertised `tickRate` to scale local game-side timing — animation playback, ability cooldowns, regeneration ticks. A reimplementation must emit a stable `tickRate` value across `TICK_SYNC` messages on the same channel; changing the tick rate mid-session would force the client to renormalize all pending timer state, and there is no V5 evidence that SGW ever does so.

Confidence: high for the wire layout, length type, and emit cadence.

#### 1.9.5 `restoreClient` (server → client, msg_id `0x34`)

The server tells the client to restore its local state to a previously known snapshot — entity ID, space, vehicle binding, position, velocity, direction. Used by the deprecated server during BaseApp restart or similar fault-recovery scenarios. Marked "Untested" in `space-viewport-wire-formats.md` §"Complete Server Message Table"; the client decompile is documented but observed traffic does not include this message in normal gameplay.

| Property | Value |
|---|---|
| Message ID | `0x34` |
| Length type | `WORD_LENGTH` (`u16` LE length prefix) |
| Payload size | Variable; the canonical 48-byte body is documented below |
| Handler in client | `ServerConnection_restoreClient` at `ghidra://SGW.exe@0x00dd8ae0` |
| Trigger (server) | BaseApp / cell restoration (server-side fault recovery) |
| Notable behavior | Client auto-emits a `restoreClientAck` reply on receipt |

**Wire layout** per `system-protocol-wire-formats.md` §"RESTORE_CLIENT (0x34) -- Client State Restore" and `space-viewport-wire-formats.md` §"RESTORE_CLIENT (0x34)":

```text
[msg_id:    0x34]       1 byte
[word_len:  u16 LE]     2 bytes  (payload size; canonical = 48)
[entityId:  u32 LE]     4 bytes
[spaceId:   u32 LE]     4 bytes
[vehicleId: u32 LE]     4 bytes
[direction: 3 × f32 LE] 12 bytes  — direction Vec3 (X, Y, Z), read as one 12-byte block
[position:  3 × f32 LE] 12 bytes  — position Vec3 (X, Y, Z), read via `FUN_015846a0` (3 × `read(4)`)
[velocity:  3 × f32 LE] 12 bytes  — velocity Vec3 (X, Y, Z), the remainder of the stream after position
```

The handler at `0x00dd8ae0` reads the four scalars and two of the three Vec3s explicitly (`entityId`, `spaceId`, `vehicleId`, direction-block via `stream.read(12)`, position via `FUN_015846a0`), and the trailing `velocity` Vec3 is what the client decompile in `space-viewport-wire-formats.md` lists as the third Vec3 field at offset 36. Note that the V5 evidence for the read order is unambiguous (direction before position), but `space-viewport-wire-formats.md`'s table at §"RESTORE_CLIENT (0x34)" labels the offsets `position, velocity, direction` in their byte order on the wire — both views are consistent: the wire byte order is `entityId, spaceId, vehicleId, direction, position, velocity` even though the C++ reader reads them in a different sequence.

**Auto-reply mechanic.** Per `system-protocol-wire-formats.md` §"RESTORE_CLIENT (0x34)" the handler at `0x00dd8ae0` auto-emits a `restoreClientAck` message back to the server before returning:

```c
if (*(int*)(this + 0x30c) != 0) {
    channel = Mercury_Channel_2(this);
    channel->writeHeader(DAT_01ef250c);  // restoreClientAck message descriptor
    *(uint32*)channel->reserve(4) = 0;   // ack payload = 0 (single u32 zero)
    Mercury_Nub_7(this);                  // flush channel
}
```

The ack descriptor at `DAT_01ef250c` carries a fixed 4-byte body whose payload is always `0u32` — the server uses the ack's *arrival* (not its contents) as the signal that the client has accepted the restore. A reimplementation must register the ack-reply path; emitting `RESTORE_CLIENT` to a Rust client that does not auto-reply will not crash the server but will leave the server's restore handshake permanently incomplete.

Confidence: high for the wire layout, length type, and auto-reply mechanic; medium for the SGW runtime emit path because the V5 record marks the message "Untested" — the byte format is fully decompiled but observed pcaps of an actual restore scenario are not in the V5 record.

#### 1.9.6 `loggedOff` (server → client, msg_id `0x37`)

The server-initiated forced disconnect. The 1-byte payload carries the reason code, which the client logs and then tears down its connection without sending a courtesy DISCONNECT back.

| Property | Value |
|---|---|
| Message ID | `0x37` |
| Length type | `CONSTANT_LENGTH = 1` |
| Payload size | 1 byte (no length prefix on the wire — fixed) |
| Handler in client | `ServerConnection_loggedOff` at `ghidra://SGW.exe@0x00dd8c20` |
| Trigger (server) | Forcible disconnect — admin kick, server shutdown, idle timeout, auth revocation |
| Notable behavior | Client tears down connection silently — `sendDisconnectMsg = false` in the call to the disconnect handler at `0x00dd8630` |

**Wire layout** per `space-viewport-wire-formats.md` §"LOGGED_OFF (0x37)" and `system-protocol-wire-formats.md` §"LOGGED_OFF (0x37)":

```text
[msg_id: 0x37]   1 byte
[reason: u8]     1 byte  — disconnect reason code
```

The handler decompile is short:

```c
void __fastcall ServerConnection_loggedOff(void *param_1)
{
    LOG("ServerConnection::loggedOff: The server has disconnected us. reason = %d\n");
    Mercury__unknown_00dd8630(param_1, '\0');  // disconnect(sendMsg = false)
}
```

The single-byte `reason` is read at `0x00dd8c2f` as `MOVZX EDX, byte ptr [ECX]` (zero-extended for the printf log) and discarded after logging — the client does not branch on the value. The disconnect handler at `0x00dd8630` destroys the connection object at `+0x30c`, frees pending resource requests, and clears the handler pointer at `+0x168`. The `\0` second argument means "do not send a DISCONNECT message back" — a reimplementation does not need to read a courtesy reply on this channel.

Confidence: high for the wire layout, length type, and "silent teardown" behavior.

### 1.10 Entity creation and position messages

A small set of system messages carries the wire-level entity lifecycle: creating the player's base proxy, creating the cell proxy, attaching it to a space viewport, ghost-entity AoI creation, and the authoritative position-snap mechanism. Each has a fixed `InterfaceElement` descriptor and a Ghidra-anchored handler in the SGW client. The full canonical wire-formats live below; the entries in the §1.14 divergence consolidation table reference these subsections. Position/movement messages on the steady-state plane (`UPDATE_AVATAR` variants, `detailedPosition`) are canonized in §1.11.

> [!NOTE] **Scope note.** These messages are documented here because their wire bytes ride the Mercury packet envelope canonized in §§1.1–1.9 — they are part of the Mercury wire-format contract. The *semantic* role of each message (when the server sends it during world entry, what the client does with the result) is canonized in `spec.world.world-entry`. Treat this section as the byte-level reference and the world-entry chapter as the lifecycle reference.

#### 1.10.1 `createBasePlayer` — base proxy creation (server → client, msg_id `0x05`)

The first entity message the server sends after auth. Creates the player's base-side proxy object. The client uses the resulting `entityId` as the routing key for every subsequent entity-method call.

| Property | Value |
|---|---|
| Message ID | `0x05` |
| Length type | `WORD_LENGTH` |
| Payload size | 6 bytes (`word_len = 6`) |
| Handler in client | `ServerConnection_CreateBasePlayer` at `ghidra://SGW.exe@0x00dddca0` |
| Trigger event (client) | `Event_NetOut_PlayCharacter` (CME string at `0x019bf4f8`) |

**Wire layout** per `entity-creation-wire-formats.md` §"2. CREATE_BASE_PLAYER (0x05)" and `system-protocol-wire-formats.md` §"CREATE_BASE_PLAYER (0x05) -- Stream Read Details":

```text
[msg_id:    0x05]        1 byte
[word_len:  u16 LE = 6]  2 bytes  (payload size)
[entityId:  u32 LE]      4 bytes  — player entity ID assigned by BaseApp
[classId:   u16 LE]      2 bytes  — entity class index (low byte = classId; high byte = 0 = propCount)
```

**`classId` width — settled at u16 from the client's read, with a layered server-source aside.** The client decompilation in `system-protocol-wire-formats.md` is explicit: `PUSH 0x2; MOV ECX, ESI; CALL EAX; MOVZX EAX, word ptr [EAX]` — the handler at `ghidra://SGW.exe@0x00dddca0` reads 2 bytes and zero-extends them as a `u16`. That is the wire contract.

The C++ server source visible in `entity-creation-wire-formats.md` §"2. CREATE_BASE_PLAYER" emits the same 2 bytes as `(uint8_t)classDef()->index() << (uint8_t)0` — a `u8 classId` followed by a `u8 propCount`. At world entry `propCount` is always 0, so the high byte of the `u16` the client reads is always 0, and the wire-level `u16` value equals the original `u8 classId`. Both descriptions are simultaneously true at different abstraction layers: the wire field is `u16`, and the server happens to compose it as `(u8 classId)(u8 propCount = 0)`. Earlier drafts of this chapter framed the two as competing interpretations; the V5 evidence resolves them as the same shape viewed from different ends of the pipe.

Confidence: high.

**Divergence from stock BigWorld 2.0.1.** Stock BW's `createBasePlayer` per `external/BigWorld-2.0.1/src/lib/connection/baseapp_ext_interface.hpp` carries an `EntityID entityID; EntityTypeID type;` pair where `EntityTypeID` is `uint16`. SGW's wire-level layout is identical — a `u32 entityId` followed by a `u16` class field. The divergence is not in the wire bytes but in the server's C++ emit style: SGW's server source writes the `u16` as two adjacent `u8` writes (`classId` then `propCount`) rather than a single `u16` write. The client decodes the same two bytes either way; this is a code-style divergence rather than a wire-format divergence. Roll-up entry in §1.13 reflects this.

**Out-of-order arrival is tolerated.** The client's `createBasePlayer` handler at `ghidra://SGW.exe@0x00dddca0` checks `ServerConnection+0xfdc` (the `cellPlayerBuffer_` field, per `system-protocol-wire-formats.md` §"ServerConnection Field Map") for a buffered `createCellPlayer` message. If `createCellPlayer` (msg_id `0x06`, see §1.10.2) arrived earlier in the same Mercury bundle — before the entity that `createCellPlayer` targets had been registered via `createBasePlayer` — the cell-side payload is stashed in `+0xfdc` and replayed once the base entity is registered. The debug string `"ServerConnection::createBasePlayer: Playing buffered createCellPlayer message"` is emitted when the buffered playback fires. This is a wire-format-relevant ordering invariant: a reimplementation that requires strict `createBasePlayer → createCellPlayer` arrival order on the wire is wrong; the protocol is robust to either order within a single bundle, and the client side handles the reorder. SGW's server typically emits the two in the canonical order (base before cell), so the buffering path is exercised mainly under packet loss + retransmission edge cases.

#### 1.10.2 `createCellPlayer` — cell proxy creation (server → client, msg_id `0x06`)

The server sends this after the client emits `enableEntities` (see §1.9). Creates the player's cell-side proxy with its initial position, vehicle binding (always 0 at world entry), and orientation. The client's space viewport is bound to this entity.

| Property | Value |
|---|---|
| Message ID | `0x06` |
| Length type | `WORD_LENGTH` |
| Payload size | 32 bytes (`word_len = 32`) |
| Handler in client | `ServerConnection_CreateCellPlayer` at `ghidra://SGW.exe@0x00dda2e0` |
| Rotation reader (internal) | `FUN_015846a0` |

**Wire layout** per `world-entry-pipeline.md` §"CREATE_CELL_PLAYER":

```text
[msg_id:     0x06]        1 byte
[word_len:   u16 LE = 32] 2 bytes
[spaceId:    u32 LE]      4 bytes   — destination space identifier
[vehicleId:  u32 LE = 0]  4 bytes   — always 0 at world entry
[posX:       f32 LE]      4 bytes
[posY:       f32 LE]      4 bytes   — vertical
[posZ:       f32 LE]      4 bytes
[rotX:       f32 LE]      4 bytes   — pitch
[rotZ:       f32 LE]      4 bytes   — yaw    *** Y/Z SWAPPED ***
[rotY:       f32 LE]      4 bytes   — roll   *** Y/Z SWAPPED ***
```

**Y/Z rotation swap — confirmed.** SGW's rotation triplet is written in the order `rotX, rotZ, rotY` — a deliberate divergence from stock BigWorld's `Direction3D` ordering (`roll, pitch, yaw`). The swap is confirmed three ways:

1. `world-entry-pipeline.md` §"Audit Findings vs `world-entry-phases.md`": "CREATE_CELL_PLAYER rotation: X, Z, Y (Y/Z swapped) — CONFIRMED via `FUN_015846a0` rotation reader."
2. The internal `FUN_015846a0` rotation reader applies the X-Z-Y ordering at parse time (Ghidra-confirmed).
3. The legacy `deprecated/cpp/src/baseapp/mercury/sgw/client_handler.cpp` pattern emits `rotX << rotZ << rotY` for this message, mirroring the same ordering at the server side.

Confidence: high.

**Divergence from stock BigWorld 2.0.1.** Stock BW's `createCellPlayer` writes a 3-float `Direction3D` (`roll, pitch, yaw`) at the end of the message. SGW swaps the Y and Z components in the wire stream — the field positions are identical (offsets `+0x14`, `+0x18`, `+0x1C` from start of position triplet), but the *semantic* assignment is `rotX → +0x14, rotZ → +0x18, rotY → +0x1C`. Any reimplementation that writes `roll, pitch, yaw` straight from a stock-BW-compatible buffer will produce a packet the SGW client mis-orients.

#### 1.10.3 `spaceData` — space metadata broadcast (server → client, msg_id `0x07`)

The server-pushed space metadata channel. Carries a `(spaceId, spaceEntryId, key, value)` tuple; the client stores or applies the (key, value) pair against the named space. **Unused in current SGW builds** — the V5 record marks this as superseded by `SGWPlayer.onClientMapLoad` (a cell-method RPC, not a system message). Documented here for completeness because the handler is still registered and the descriptor still sits in the dispatch table.

| Property | Value |
|---|---|
| Message ID | `0x07` |
| Length type | `WORD_LENGTH` (`u16` LE length prefix) |
| Payload size | Variable; minimum 14 bytes (header before the value string) |
| Handler in client | `ServerConnection_spaceData` at `ghidra://SGW.exe@0x00dda540` |
| Notable behavior | Unused in current SGW builds (per `space-viewport-wire-formats.md` §"SPACE_DATA (0x07)") |

**Wire layout** per `space-viewport-wire-formats.md` §"SPACE_DATA (0x07)":

```text
[msg_id:       0x07]      1 byte
[word_len:     u16 LE]    2 bytes  (payload size; minimum 14, total varies with value-string length)
[spaceId:      u32 LE]    4 bytes  — space identifier
[spaceEntryId: u64 LE]    8 bytes  — space entry ID (read as two u32s by the handler)
[key:          u16 LE]    2 bytes  — space-data key
[value:        bytes]     var      — remaining payload bytes interpreted as the value string
```

The handler at `0x00dda540` reads the four scalars via four `stream.read(...)` calls (`read(4)` for `spaceId`, `read(8)` for `spaceEntryId`, `read(2)` for `key`) and then consumes the remaining bytes as the `value` string. The debug string `"ServerConnection::spaceData: space %d key %d"` is emitted on receipt. The C++ server source in `messages.cpp:189-190` registers the message as `WORD_LENGTH` and documents the field set as `SpaceID, SpaceEntryID, Key, Value`.

Because SGW's running protocol replaces this message with the `onClientMapLoad` cell-method RPC, a reimplementation does not need to emit `spaceData` to drive any client-visible behavior. Pin to high confidence on the wire layout; high confidence on the "unused" status per the V5 doc; the precise circumstances under which the deprecated server *would* have emitted this message are not enumerated in V5 evidence and remain out of scope for this chapter.

#### 1.10.4 `spaceViewportInfo` — viewport binding (server → client, msg_id `0x08`)

Tells the client which entity (its own player) is bound to which space viewport. Sent in the same Mercury packet as `createCellPlayer` and `forcedPosition` (see `spec.world.world-entry` for the bundling).

| Property | Value |
|---|---|
| Message ID | `0x08` |
| Length type | `CONSTANT_LENGTH = 13` |
| Payload size | 13 bytes (no length prefix on the wire — fixed) |
| Handler in client | `ServerConnection_SpaceViewportInfo` at `ghidra://SGW.exe@0x00dda6c0` |

**Wire layout** per `entity-creation-wire-formats.md` §"4. SPACE_VIEWPORT_INFO (0x08)" and `space-viewport-wire-formats.md` §"SPACE_VIEWPORT_INFO (0x08)":

```text
[msg_id:     0x08]    1 byte
[entityId:   u32 LE]  4 bytes   — controlling entity ID (per C++ server source); client decompile labels this `field0` and notes "gate/unknown" — the semantic identity is inferred from server emit-side, not confirmed by client read-side
[entityId2:  u32 LE]  4 bytes   — viewport target entity ID (same as entityId when opening)
[spaceId:    u32 LE]  4 bytes   — space identifier
[viewportId: u8  = 0] 1 byte    — viewport index (always 0 in SGW)
```

**Decompile-level naming ambiguity.** The C++ server source in `entity-creation-wire-formats.md` §"SPACE_VIEWPORT_INFO" emits the first u32 as `entityId` (the controlling entity, typically the player). The client decompile of `ServerConnection_SpaceViewportInfo` at `0x00dda6c0` labels this field `field0 (u32) — gate/unknown` and treats it as opaque except for storage at `puVar5+0`. Wire-level byte position and width are unambiguous (`u32 LE` at offset 1); the semantic role is inherited from the server source rather than verified independently from the client side. The chapter uses the server-source label for clarity.

The two entity-ID fields are stock-BigWorld's *viewport-owner* + *viewport-target* pair: the owner is the entity owning the viewport (typically the local player); the target is the entity the viewport is anchored to (usually the same as the owner, but in stock BW different when one entity observes another — spectator camera, replay viewer, GM-overseen client).

**Open vs close semantics — driven by `entityId2`.** Per `entity-creation-wire-formats.md` §"Viewport Operations" and the corroborating §"Operations" table in `space-viewport-wire-formats.md`:

| `entityId2` value | Behavior |
|---|---|
| Non-zero (typically equal to `entityId`) | **Open** or **update** the viewport. The client maps `viewportId → (entityId, spaceId)` in its viewport table at `ServerConnection+0xf84`. |
| `0` | **Close** the viewport. `spaceId` is also zero in this case. The client removes the mapping. |

Updating an existing `viewportId` with a different `spaceId` triggers the debug warning `"ServerConnection::spaceViewportInfo: Server wants us to re-use space viewport %d changing space from %d to %d!"`. This is the wire-level open/close/update protocol — it is not driven by a separate "close viewport" message type.

During world entry both fields equal the player's own entity ID (open viewport for the player's own anchor). During cleanup or teardown the server can send `entityId2 = 0` to close. SGW's running game never carries a third pattern (`entityId ≠ entityId2 ≠ 0`) — the stock-BW spectator-camera distinction is not exercised — but the field exists on the wire because the stock layout reserves it.

`viewportId = 0` is the unique viewport SGW uses; there is no multi-viewport mode in the running game. The field exists on the wire because stock BigWorld supports multiple simultaneous viewports per client (split-screen / picture-in-picture). SGW reserves the field but the running game never reads or emits a non-zero value.

#### 1.10.5 `createEntity` — ghost-entity AoI creation (server → client, msg_id `0x09`)

The wire-level message that announces a non-player entity entering the client's Area of Interest. Used for NPCs, other players, world objects, and any entity the client must instantiate as a ghost (server-authoritative, client-side-rendered). The payload carries the new entity's ID, the class index (from `entities.xml`), and a 1-byte ID alias plus two reserved bytes. After the create, the server immediately sends an `UPDATE_AVATAR_NO_ALIAS_FULL_POS_YAW_PITCH_ROLL` (msg_id `0x10`, §1.11) to seat the new entity at its initial position and orientation.

| Property | Value |
|---|---|
| Message ID | `0x09` |
| Length type | `WORD_LENGTH` (`u16` LE length prefix) |
| Payload size | 5 bytes (`word_len = 5`) |
| Handler in client | Via entity-message dispatch — no standalone Ghidra-anchored handler in `space-viewport-wire-formats.md` |
| Trigger (server) | Entity enters the player's AoI; immediately followed by `UPDATE_AVATAR (0x10)` to deliver initial position |

**Wire layout** per `entity-creation-wire-formats.md` §"6. CREATE_ENTITY (0x09)" and `space-viewport-wire-formats.md` §"CREATE_ENTITY (0x09)":

```text
[msg_id:    0x09]     1 byte
[word_len:  u16 LE = 5] 2 bytes  (payload size)
[entityId:  u32 LE]   4 bytes  — newly assigned entity ID
[idAlias:   u8 = 0xFF] 1 byte   — ID alias for compression (0xFF means "no alias assigned")
                                  — used as a 1-byte stand-in for the 4-byte entityId by the
                                    aliased `UPDATE_AVATAR` variants in the `0x20–0x2F` range
[classId:   u8]       1 byte    — entity class index, lookup into entities.xml class table
[unknown1:  u8 = 0]   1 byte    — always zero in observed traffic
[unknown2:  u8 = 0]   1 byte    — always zero in observed traffic
```

The C++ server source at `client_handler.cpp:497-499`:

```cpp
bundle.beginMessage(BASEMSG_CREATE_ENTITY);
bundle << entityId << (uint8_t)0xff << classId << (uint8_t)0x00 << (uint8_t)0x00;
bundle.endMessage();
```

The two trailing zero bytes (`unknown1`, `unknown2`) have not been pinned to a named field in V5 evidence — they may be stock-BigWorld reserved fields whose role atrophied as SGW narrowed the protocol, or padding for a struct that was eventually shortened. Both bytes are observed to be exactly zero across captured traffic; a reimplementation should emit `0x00 0x00` and verify the client accepts the packet.

**Follow-up position message.** After `CREATE_ENTITY`, the server *unconditionally* sends one of the `UPDATE_AVATAR` variants (§1.11) to deliver the entity's initial position and orientation. The canonical pairing from `client_handler.cpp:548-556` uses msg_id `0x10` (NoAlias + FullPos + YawPitchRoll, 25-byte payload). The two messages are not part of the same bundle by protocol contract, but observed traffic shows the server consistently pairs them within the same packet whenever bundle space allows.

**Divergence from stock BigWorld 2.0.1.** Stock BW emits `CREATE_ENTITY` with a slightly different field set: the V5 record does not surface the exact stock-BW layout, but the SGW form documented above has been observed across multiple sessions and is canonical for SGW. The wire format inherits the `WORD_LENGTH` framing from stock BW; the 5-byte fixed payload size is observed-constant for SGW.

Confidence: high for the wire layout, the length type, and the field set; medium for the precise semantic role of `unknown1`/`unknown2` (the bytes exist on the wire and are always zero — what they would carry in a richer protocol variant is not in the V5 record).

#### 1.10.6 `forcedPosition` — authoritative position snap (server → client, msg_id `0x31`)

The authoritative "you are here" message. Sent by the server when the client's position must be hard-set (world entry, gate travel, anti-cheat correction, teleport). Carries position, velocity, orientation, and a physics-mode byte. Unlike `avatarUpdate` (the client's position-broadcast) or normal entity-method calls, `forcedPosition` is a system-level wire-format message with a fixed 49-byte payload.

| Property | Value |
|---|---|
| Message ID | `0x31` |
| Length type | `CONSTANT_LENGTH = 49` |
| Payload size | 49 bytes (no length prefix on the wire — fixed) |
| Handler in client | `ServerConnection_ForcedPosition` at `ghidra://SGW.exe@0x00dd9ee0` |

**Wire layout** per `entity-creation-wire-formats.md` §"5. FORCED_POSITION (0x31)", `position-movement-wire-formats.md` §"forcedPosition (msg_id 0x31, 49 bytes)", and `space-viewport-wire-formats.md` §"FORCED_POSITION (0x31)":

```text
[msg_id:    0x31]        1 byte
[entityId:  u32 LE]      4 bytes
[spaceId:   u32 LE]      4 bytes
[vehicleId: u32 LE = 0]  4 bytes
[posX:      f32 LE]      4 bytes
[posY:      f32 LE]      4 bytes
[posZ:      f32 LE]      4 bytes
[velX:      f32 LE]      4 bytes   — 0 at world entry; non-zero for in-flight corrections
[velY:      f32 LE]      4 bytes
[velZ:      f32 LE]      4 bytes
[rot_a:     f32 LE]      4 bytes   — see rotation note below
[rot_b:     f32 LE]      4 bytes
[rot_c:     f32 LE]      4 bytes
[physics:   u8]          1 byte    — physics/movement mode (NOT a reserved flags byte)
```

**The trailing byte is `physics`, not a generic flags field.** Per `position-movement-wire-formats.md` §"Field Notes" the byte at offset 48 "encodes the current physics mode (walking, flying, swimming, etc.). Stored per-entity in `sentPhysics_[]`." The world-entry C++ emit path (`client_handler.cpp:407-413` per `entity-creation-wire-formats.md`) writes `(uint8_t)0x01` — value `0x01`, not `0x00` — and the handler at `0x00dd9ee0` asserts `sentPhysics_[args.id] == args.physics`. The byte is consumed as per-entity mutable state, not discarded.

> [!NOTE] **Source-doc override.** `docs/reverse-engineering/findings/world-entry-pipeline.md` §"FORCED_POSITION" labels the byte at offset 48 as `flags: u8 = 0`, which is incorrect. The C++ server source extracted in `entity-creation-wire-formats.md` §"5. FORCED_POSITION (0x31)" shows the world-entry emit path writes `(uint8_t)0x01`, and the client decompile evidence in `position-movement-wire-formats.md` §"Field Notes" plus the assertion `sentPhysics_[args.id] == args.physics` in the handler at `0x00dd9ee0` confirms the byte is consumed as the per-entity physics-mode field, not as a reserved flags slot. This chapter follows the C++ source and the position-movement-wire-formats doc; the `world-entry-pipeline.md` value is a known transcription error and should be corrected when that doc is next revised.

A second source-doc conflict touches the same message at a different field — the rotation annotation:

> [!NOTE] **Source-doc rotation annotation conflict.** `docs/reverse-engineering/findings/entity-creation-wire-formats.md` §"5. FORCED_POSITION (0x31)" (client decompile rows for offsets 36/40/44) labels the three rotation fields `rotX/rotY/rotZ` and adds the annotation "NOT swapped here (unlike createCellPlayer)". That annotation is misleading. `system-protocol-wire-formats.md` §"FORCED_POSITION (0x31) -- Rotation Order Evidence" confirms that the `ServerConnection_addMove` call in the handler at `0x00dd9ee0` maps `param_1[10]` (wire offset 40) to the `rotZ` argument and `param_1[11]` (wire offset 44) to the `rotY` argument — i.e. the wire *does* carry the Z-component at the byte position the decompile struct labels Y. The chapter's neutral `rot_a/rot_b/rot_c` labels in the wire-layout table above avoid the conflict. The C++ emit-side and the parse-side both agree that the world-entry path's three rotation bytes are written in the order `(rotation.x, rotation.z, rotation.y)`, which is consistent with the `createCellPlayer` convention. The decompile struct's `rotX/rotY/rotZ` field names are decompile-tool placeholders, not protocol-canonical names — the protocol-canonical interpretation is in `system-protocol-wire-formats.md`'s addMove mapping.

**Rotation order is per call site, not a protocol-wide convention.** SGW emits `forcedPosition` from two distinct C++ call sites per `entity-creation-wire-formats.md` §"C++ Server Source":

| Call site | C++ rotation emit | Wire byte order at offsets 36–47 |
|---|---|---|
| `client_handler.cpp:407-413` (world-entry path during `createCellPlayer`) | `rotX << rotZ << rotY` | `rotX, rotZ, rotY` (Y/Z swapped) |
| `client_handler.cpp:566-572` (standalone `forcedPosition` from `ServerConnection::forcedPosition()`) | `rotation.x << rotation.y << rotation.z` | caller's responsibility — V5 comment: "caller's responsibility" |

The client at `0x00dd9ee0` reads the three floats positionally — offset 36 to `param_1[9]`, offset 40 to `param_1[10]`, offset 44 to `param_1[11]` — and the handler shuffles them as `addMove(yaw = param[11], pitch = param[10], roll = param[9])` (per `system-protocol-wire-formats.md` §"FORCED_POSITION (0x31) -- Rotation Order Evidence"). That positional read works correctly *only when the caller writes Y/Z swapped on the wire* — which the world-entry path does and the standalone path does not by default.

The same applies more broadly: rotation order is per call site, not a protocol-wide convention. `createCellPlayer` (§1.10.2) writes `rotX, rotZ, rotY` (Y/Z swapped) at the world-entry path. The `UPDATE_AVATAR` family (msg_id `0x10–0x2F`) encodes rotation as three packed `u8` quanta `yaw, pitch, roll` — a completely different layout (`(u8)(rotation.y / 0.024543693f)` etc., per `position-movement-wire-formats.md`). Each message's rotation byte order belongs to that message's subsection, not to a global rule. Confidence: high for the world-entry path; medium for the standalone-path rotation interpretation pending pcap capture of an in-flight correction.

**Divergence from stock BigWorld 2.0.1.** Stock BW's `forcedPosition` carries 36 bytes: `entityID (4) + spaceID (4) + vehicleID (4) + Position3D (12) + Direction3D (12) = 36`. SGW expands this to 49 bytes by:

1. Inserting a 12-byte velocity `Vec3` between position and rotation (zero at world entry; non-zero for in-flight position corrections — the wire byte counts are V5-confirmed; the conditions that produce non-zero velocity outside world entry are open Q3 in §1.16).
2. Appending a 1-byte `physics` field (value `0x01` at world entry; per-entity mutable state at runtime).

Both additions are SGW-specific. The Cimmeria server must emit the full 49-byte payload; emitting the stock 36-byte payload would fail the `CONSTANT_LENGTH = 49` table check in the client's `InterfaceElement` decoder and the packet would be dropped silently. Confidence: high for the 49-byte total and the world-entry wire bytes; medium for the in-flight velocity semantics (Q3 in §1.16).

#### 1.10.7 `AUTHENTICATE` — Mercury-handshake key delivery (server → client, msg_id `0x00`)

The only V5-confirmed `DWORD_LENGTH` interface element in the running protocol. Carries the session key the client uses to verify the channel was negotiated by the expected SOAP authority. Sent once per channel, during the initial Mercury handshake — never during gameplay. Documented here so the §1.5 length-type table is not misleading about `DWORD_LENGTH`'s scope; the full lifecycle (SOAP `SessionKey` → AES key derivation → first packet over the cipher envelope) is canon for `spec.protocol.cipher-and-auth`, which is the right home for the auth flow.

| Property | Value |
|---|---|
| Message ID | `0x00` |
| Length type | `DWORD_LENGTH` (4-byte `u32` LE length prefix) |
| Handler in client | `ServerConnection_authenticate` at `ghidra://SGW.exe@0x00dd8510` |
| Payload | Packed string — `[1 byte: len or 0xFF][if 0xFF: 3 bytes extended len][len bytes: data]` |
| Sent | During the initial connection handshake, before any entity-message traffic |

Per `system-protocol-wire-formats.md` §"AUTHENTICATE (0x00) -- Server-to-Client Key Exchange", the handler reads a packed string (the session key) via the utility at `0x00de3770`, compares it against the stored key at `ServerConnection+0x08`, and logs `"ServerConnection::authenticate: Unexpected key! (%s, wanted %s)"` on mismatch. The packed-string reader uses a 1-byte length with `0xFF`-escape to a 3-byte extended length, so the inner string length is variable; the outer `DWORD_LENGTH` is the framing the Mercury decoder applies to find the message boundary.

Confidence: high for the length type and the handler-side decoder; the cipher key handling and the session-key end-to-end flow are out of scope for this chapter.

### 1.11 Position and movement messages

The position-update plane carries the steady-state per-entity location traffic — three logical message families share the role: the 32 `UPDATE_AVATAR` variants (msg_ids `0x10–0x2F`) for compressed AoI movement broadcasts; `DETAILED_POSITION` (msg_id `0x30`) for full-precision non-controlled entity snaps; and `FORCED_POSITION` (msg_id `0x31`, canon at §1.10.6 because of its world-entry role) for authoritative client-position snaps. This section canonizes the `UPDATE_AVATAR` family at the protocol-level and the `DETAILED_POSITION` byte format; the full per-variant table for the 32 `UPDATE_AVATAR` byte layouts is reserved for the future `spec.protocol.position-updates` chapter.

#### 1.11.1 `UPDATE_AVATAR` variants — AoI movement broadcasts (server → client, msg_ids `0x10–0x2F`)

The compressed AoI position update. Each of the 32 variants encodes a position update for one ghost entity (an entity in the client's Area of Interest, server-authoritative, client-side-rendered). The variant index is a 5-bit field encoded into the `msg_id` byte itself; the 5 bits select which subset of `(idAlias, position, direction)` fields are present on the wire, trading flexibility for byte count.

| Property | Value |
|---|---|
| Message ID | `0x10 – 0x2F` (32 variants) |
| Length type | `CONSTANT_LENGTH` (per-variant; 7–25 bytes depending on encoding) |
| Length range | 7 bytes (msg_id `0x2F`: Alias + NoPos + NoDir) — 25 bytes (msg_id `0x10`: NoAlias + FullPos + YawPitchRoll) |
| Handler in client | One handler per variant, all in the `FUN_00ddb???` and `FUN_00de1???` ranges per `position-movement-wire-formats.md` §"All 32 Variant Handlers" |
| Trigger (server) | Server-side position update for any AoI ghost entity; emitted at the tick rate while the entity moves |
| Notable behavior | **Does not work on client-controlled entities** — use `forcedPosition` (§1.10.6) for those |

**Variant encoding.** The 32 variants map a 5-bit index onto a 2×4×4 matrix of encoding choices:

| Dimension | Options | Wire-byte impact |
|---|---|---|
| Entity ID width | `NoAlias` (4-byte u32) or `Alias` (1-byte u8) | Saves 3 bytes when an alias has been assigned via `CREATE_ENTITY` |
| Position width | `FullPos` (12 B, 3 × f32), `OnChunk` (12 B but Y ignored), `OnGround` (12 B but Y ignored), `NoPos` (0 B) | Saves 12 bytes when omitted |
| Direction width | `YawPitchRoll` (3 B), `YawPitch` (2 B), `Yaw` (1 B), `NoDir` (0 B) | Saves 0–3 bytes depending on which angles are unchanged |

The `msg_id` byte itself selects which combination is on the wire. From `position-movement-wire-formats.md` §"All 32 Variant Sizes":

| Range | Alias | Position | Direction variants | Per-variant sizes |
|---|---|---|---|---|
| `0x10–0x13` | NoAlias (4 B) | FullPos (12 B) | YPR / YP / Y / None | 25 / 24 / 23 / 22 |
| `0x14–0x17` | NoAlias (4 B) | OnChunk (12 B) | YPR / YP / Y / None | 25 / 24 / 23 / 22 |
| `0x18–0x1B` | NoAlias (4 B) | OnGround (12 B) | YPR / YP / Y / None | 25 / 24 / 23 / 22 |
| `0x1C–0x1F` | NoAlias (4 B) | NoPos (0 B) | YPR / YP / Y / None | 13 / 12 / 11 / 10 |
| `0x20–0x23` | Alias (1 B) | FullPos (12 B) | YPR / YP / Y / None | 22 / 21 / 20 / 19 |
| `0x24–0x27` | Alias (1 B) | OnChunk (12 B) | YPR / YP / Y / None | 22 / 21 / 20 / 19 |
| `0x28–0x2B` | Alias (1 B) | OnGround (12 B) | YPR / YP / Y / None | 22 / 21 / 20 / 19 |
| `0x2C–0x2F` | Alias (1 B) | NoPos (0 B) | YPR / YP / Y / None | 10 / 9 / 8 / 7 |

The byte count includes the entity-ID/alias field, position (when present), velocity (always 5 bytes), 1-byte flags, and 0–3 direction bytes. The message body never carries a length prefix — each variant is `CONSTANT_LENGTH` and its size is registered statically.

**Canonical variant — `UPDATE_AVATAR_NO_ALIAS_FULL_POS_YAW_PITCH_ROLL` (msg_id `0x10`, 25 bytes).** This is the variant the SGW server emits in most observed traffic; it is also the variant the C++ server source defaults to in `client_handler.cpp:548-556`. The other 31 variants are byte-shorter compressions of the same field set.

```text
[msg_id:   0x10]              1 byte
[entityId: u32 LE]            4 bytes  — full entity ID (NoAlias variant)
[posX:     f32 LE]            4 bytes  — world X position
[posY:     f32 LE]            4 bytes  — world Y position (vertical)
[posZ:     f32 LE]            4 bytes  — world Z position
[velocity: packed 5 bytes]    5 bytes  — packXYZ-compressed velocity; see below
[flags:    u8]                1 byte   — movement flags (`0x01` typical, stock-BW reserved bits)
[yaw:      u8]                1 byte   — `(u8)(rotation.y / 0.024543693f)` — 256 steps over 2π rad
[pitch:    u8]                1 byte   — `(u8)(rotation.x / 0.024543693f)`
[roll:     u8]                1 byte   — `(u8)(rotation.z / 0.024543693f)`
```

**Quantized direction angles.** Each direction angle is a `u8` encoding 256 evenly-spaced steps over `2π` radians. The encode constant `0.024543693 = 2π / 256` is anchored at `DAT_01816a84` (medium confidence on the address; high confidence on the value, which is decompile-confirmed in every per-variant handler at `position-movement-wire-formats.md` §"Direction quantization"). To decode: `angle_rad = byte * 0.024543693`. The wire order is `yaw, pitch, roll` but the encoded *source* axes are `rotation.y, rotation.x, rotation.z` respectively — the SGW server source explicitly writes `(rotation.y / k), (rotation.x / k), (rotation.z / k)` per `client_handler.cpp:548-556`.

**Compressed velocity (`packXYZ`).** Velocity is 5 bytes: a packed `u32` plus a tail `u8`. The encoding extracts mantissa bits from each component's IEEE 754 representation, adds a bias of `2.0` to the absolute value (avoiding zero-encoding), and concatenates sign/magnitude fields. The exact bit layout from `position-movement-wire-formats.md` §"Velocity Compression":

```text
packed1 (u32 LE):
  bits [31:24]:  Y delta high byte
  bit  [23]:     X sign (1 = negative)
  bits [22:12]:  X mantissa (11 bits)
  bit  [11]:     Z sign (1 = negative)
  bits [10:0]:   Z mantissa (11 bits)

packed2 (u8):
  bit  [7]:      Y sign (1 = negative)
  bits [6:0]:    Y delta low 7 bits
```

A reimplementation must replicate the bias-then-extract pipeline exactly — emitting raw IEEE 754 bytes will produce a client-side velocity off by ~`2.0` in each axis.

**Position-type semantics.** Even though the three "with position" variants (`FullPos`, `OnChunk`, `OnGround`) all carry 12 wire bytes of position, the per-variant handler differs in how it interprets the Y component:

- `FullPos` handlers (e.g. `FUN_00ddb0c0`): read all three floats as-is (`local_8 = param_1[2]`).
- `OnChunk` handlers (e.g. `FUN_00ddb220`): discard the wire Y and substitute the sentinel at `DAT_019d1a44` (likely `FLT_MAX`); the client derives Y from the chunk's height map.
- `OnGround` handlers (e.g. `FUN_00ddb830`): discard the wire Y and substitute the same sentinel; the client derives Y from terrain ray-cast.

The 4 wire bytes at the Y offset are still present in every variant — the difference is purely how the handler consumes them. This means a reimplementation can always emit the same 12 position bytes regardless of variant; the variant choice is the server's signal to the *client* about how to interpret Y, not a wire-format change.

**Direction option ordering.** Within each `(alias, position)` pair, the four direction variants always appear in the order `YPR / YP / Y / NoDir` as the `msg_id` low 2 bits increase. So `0x10` is `NoAlias + FullPos + YPR`, `0x11` is `NoAlias + FullPos + YP`, `0x12` is `NoAlias + FullPos + Y`, `0x13` is `NoAlias + FullPos + NoDir`. This ordering is consistent across all 8 `(alias, position)` rows.

**Unreliable-channel emission.** The server's `unreliable_movement_update` config flag controls whether `UPDATE_AVATAR` is emitted on the reliable Mercury channel (default) or the unreliable channel. In SGW the flag is typically true for AoI position spam to avoid retransmission overhead; this is a server-side configuration, not a wire-format property.

> [!NOTE] **Full per-variant byte-layout table reserved.** The 32 variants share the same field set but differ in byte offsets per variant. A complete per-variant byte-layout table (offsets for each of the 32 `msg_ids`) is out of scope for this chapter — the `UPDATE_AVATAR` family belongs to `spec.protocol.position-updates`, a future chapter dedicated to the position-update plane. The canonical-variant table above + the all-32-variant-sizes table is sufficient for any reimplementation that needs to decode any one variant: subtract the absent-field byte counts from the canonical 25-byte layout in the order `idAlias (3 saved if Alias)`, `position (12 saved if NoPos)`, `direction (3/2/1/0)`.

Confidence: high for the variant taxonomy (the 2×4×4 matrix), the canonical 25-byte layout, the `packXYZ` velocity encoding, and the quantized-angle encoding; high for the position-type sentinel behavior and the unreliable-channel emit option.

#### 1.11.2 `detailedPosition` — full-precision non-controlled entity snap (server → client, msg_id `0x30`)

The full-precision sibling to `forcedPosition`. Carries `entityId`, position, velocity, and rotation as full `f32` values plus a 1-byte physics-mode field — but unlike `forcedPosition`, it does *not* carry `spaceId` or `vehicleId`. The omitted fields are preserved from the entity's current state, which is why this message is used for full-precision position updates that do not change the entity's space or vehicle assignment.

| Property | Value |
|---|---|
| Message ID | `0x30` |
| Length type | `CONSTANT_LENGTH = 41` |
| Payload size | 41 bytes (no length prefix on the wire — fixed) |
| Handler in client | `FUN_00dd9e00` at `ghidra://SGW.exe@0x00dd9e00` |
| Trigger (server) | Full-precision position update for a non-controlled entity (NPC, vehicle, observer-viewable player) |
| Notable behavior | **Does not work on client-controlled entities** — use `forcedPosition` (§1.10.6) for those |

**Wire layout** per `position-movement-wire-formats.md` §"detailedPosition (msg_id 0x30, 41 bytes)" and `space-viewport-wire-formats.md` §"DETAILED_POSITION (0x30)":

```text
[msg_id:   0x30]    1 byte
[entityId: u32 LE]  4 bytes
[posX:     f32 LE]  4 bytes
[posY:     f32 LE]  4 bytes  — vertical
[posZ:     f32 LE]  4 bytes
[velX:     f32 LE]  4 bytes
[velY:     f32 LE]  4 bytes
[velZ:     f32 LE]  4 bytes
[roll:     f32 LE]  4 bytes  — rotation about Z axis (radians)
[pitch:    f32 LE]  4 bytes  — rotation about X axis (radians)
[yaw:      f32 LE]  4 bytes  — rotation about Y axis (radians)
[physics:  u8]      1 byte   — physics/movement mode (same per-entity field as `forcedPosition`)
```

**Relationship to `forcedPosition`.** `detailedPosition` is the 41-byte sibling of the 49-byte `forcedPosition` (§1.10.6). The byte count differs by 8 bytes — `forcedPosition` adds `spaceId` (4 B) and `vehicleId` (4 B) immediately after `entityId`, before the position triplet. The motivation is reach: `forcedPosition` can change the entity's space and vehicle binding atomically with the position snap; `detailedPosition` cannot. From the client's perspective, both messages carry the same physics-mode byte at the end and the same `addMove`-style consumption pattern; from the server's perspective, `detailedPosition` is the cheaper of the two for the common case where the entity stays in its current space.

**Rotation order — `roll, pitch, yaw` on the wire.** Unlike `forcedPosition`'s `rotX, rotZ, rotY` order (which the addMove handler shuffles internally), `detailedPosition` writes its rotation triplet in the conventional `roll, pitch, yaw` order — `position-movement-wire-formats.md` §"detailedPosition" labels the three offset rows as `roll (radians)`, `pitch (radians)`, `yaw (radians)`. The rotation field order here matches stock-BW `Direction3D` convention, not SGW's `createCellPlayer`/`forcedPosition` Y/Z swap. A reimplementation must use the message-specific rotation order — there is no protocol-wide convention.

**Handler behavior** per `position-movement-wire-formats.md` §"detailedPosition / Handler Behavior":

1. Resolves the entity via `FUN_00dd9d20` (SVID follow logic — the message is rejected if the entity is client-controlled).
2. If the position is for the entity we control, stores position in the entity record at offset `+0x10` (12 bytes).
3. Invokes the callback with full position/velocity/rotation data.

**Divergence from stock BigWorld 2.0.1.** Stock BW's analogous full-precision position message carries the rotation triplet in the same `roll, pitch, yaw` order — no divergence on rotation. The SGW form is byte-compatible with the stock BW shape modulo the trailing physics byte (which is the same SGW addition documented in §1.10.6 for `forcedPosition`).

Confidence: high for the wire layout, length type, rotation order, and "does-not-work-on-client-controlled-entities" constraint; the constraint is V5-confirmed in `position-movement-wire-formats.md` §"detailedPosition" and again at §"forcedPosition" via the symmetric "use forcedPosition for client-controlled entities" callout.

### 1.12 Nub — endpoint object

The *nub* is the Mercury endpoint. Every process has exactly one. The SGW client nub is constructed once at startup; the server nub is constructed once when the BaseApp starts listening. The nub owns the UDP socket, the network thread, the connection map, the listener registrations, and the channel table.

**Constructor.** `Mercury::Nub::Nub` at `ghidra://SGW.exe@0x015841d0` is a 24-step constructor. Highlights from the V5 reconstruction (full step-by-step is in `mercury-protocol-internals.md` §"Mercury::Nub::Nub"):

1. Create a `tbb::concurrent_queue<ClientMessage*>` for the inbound queue.
2. Create a second `tbb::concurrent_queue` for outbound queue work items.
3. Spawn the network thread named `"NetworkThread for ExternalNub"`.
4. Initialize the connection map via `Nub::initConnectionMap` at `ghidra://SGW.exe@0x01580620`.
5. Create the UDP socket via `Nub::addListeningSocket` at `ghidra://SGW.exe@0x01583440` (socket + bind + register).
6. Initialize rdtsc-based timer state.
7. Stamp vtables for `Mercury::Nub`, `Mercury::BaseNub`, and the `TimerExpiryHandler` base.
8. ...steps 8–24: see `mercury-protocol-internals.md` for the full inventory.

The nub's `processPendingEvents` at `ghidra://SGW.exe@0x01581ab0` is the main recv loop: blocking `recvfrom`, then enqueue each packet onto the inbound `tbb::concurrent_queue`. A second thread drains the queue and runs `processFilteredPacket` → `processFilteredPacket_inner` → `processPacket` → `processOrderedPacket` → handler dispatch.

**The send path is the inverse.** `ServerConnection::send` at `ghidra://SGW.exe@0x00dd8930` is the game-level entry; it calls `Mercury::Channel::send` at `ghidra://SGW.exe@0x01576f90`, which calls `Bundle::finalise` and `Nub::send` at `ghidra://SGW.exe@0x01582160`, which finally calls `Nub::writeConnection` at `ghidra://SGW.exe@0x01583a90` for the actual `sendto()`. The cipher envelope is applied somewhere between `Bundle::finalise` and `writeConnection` — `PacketEncrypter::send` (vfunc slot 1 of the cipher object) is registered as a packet filter and runs in line.

### 1.13 MachineGuard — adjacent machine-discovery protocol

MachineGuard is a *separate* UDP protocol that SGW uses for machine-level service discovery. It is not Mercury — different port, different message types, different deserializer — but it lives in the same binary range (`[0x01585000, 0x0158efff]`) and is sometimes conflated with Mercury in older docs.

| Property | Value |
|---|---|
| Port | `0x4e36` *or* `0x4c36` — **disputed**: V5 docs paired `0x4e36` with decimal `19510`, but `0x4E36 = 20022` and `0x4C36 = 19510`. Pin to medium pending direct read of the Ghidra string-literal constant at the MachineGuard listen-socket bind site (see Q4 in §1.16). |
| Master deserializer | `ghidra://SGW.exe@0x01588530` |
| Send raw packet | `ghidra://SGW.exe@0x01588ec0` |
| Message types | At least 8 documented in V5; dispatcher switches on type bytes in the range `0x01–0x0c + 0x40` (see "partial enumeration" note below) |

**Port hex/decimal inconsistency.** `mercury-protocol-internals.md` §"MachineGuard Protocol" carries the pairing `0x4e36 (19510)` and `docs/reverse-engineering/v5-campaign/CAMPAIGN_STATUS.md` echoes the same pair, but these two values do not match: `0x4E36` is `20022` in decimal and `0x4C36` is `19510`. The hex/decimal disagreement originated upstream in the V5 source docs and was propagated forward without flagging. Without a direct read of the Ghidra string-literal constant at the MachineGuard listen-socket bind site, we cannot tell whether the hex (`0x4E36`) is correct and the decimal (`19510`) is wrong, or whether the decimal is correct and the hex should be `0x4C36`. Confidence on the port number is medium pending that direct read; see Q4 in §1.16.

The master deserializer at `0x01588530` switches on a single type byte. **Message types — partial enumeration.** The dispatcher's switch range is `0x01–0x0c` plus `0x40`. Eight slots are documented in V5 (table below); five slots (`0x03`, `0x08`, `0x09`, `0x0a`, `0x0c`) have no named handler in current V5 evidence and may be either unused or pending Ghidra recovery. The "13 message types" claim in `mercury-protocol-internals.md` §"MachineGuard Protocol" reflects the dispatcher's address range, not the count of recovered handlers. This chapter pins the canonized count to "at least 8 documented" and lists the gaps explicitly.

| Type byte | Class | Constructor |
|---|---|---|
| `0x01` | `WholeMachineMessage` | `0x01587d30` |
| `0x02` | `ProcessMessage` | (read at `0x015896d0`, write at `0x01586180`) |
| `0x03` | `<unknown>` | Not enumerated in V5; switch slot exists but no handler name recovered |
| `0x04` | `ListenerMessage` | `0x01586410` |
| `0x05` | `CreateMessage` | `0x01586590` |
| `0x06` | `SignalMessage` | `0x01586710` |
| `0x07` | `TagsMessage` | `0x01587ef0` |
| `0x08` | `<unknown>` | Not enumerated in V5; switch slot exists but no handler name recovered |
| `0x09` | `<unknown>` | Not enumerated in V5; switch slot exists but no handler name recovered |
| `0x0a` | `<unknown>` | Not enumerated in V5; switch slot exists but no handler name recovered |
| `0x0b` | `ErrorMessage` | `0x015867c0` |
| `0x0c` | `<unknown>` | Not enumerated in V5; switch slot exists but no handler name recovered |
| `0x40` | `MachinedAnnounceMessage` | (named in earlier session) |

**Variable-length ID encoding in `ProcessMessage`:** component IDs `≤ 0xfe` are written as 1 byte; IDs `> 0xfe` are written as `0xff` prefix + 3 bytes. See `ProcessMessage::writeComponentsVarLen` at `ghidra://SGW.exe@0x01586180`. This is the closest analog to the (un-pinned) `InterfaceElement::compressLength_write` threshold mentioned in §1.5.

MachineGuard is mentioned here because the V5 finding doc recovered it alongside Mercury and it shares wire-format vocabulary (variable-length IDs, type-byte dispatch). It does not yet have a bible chapter; the glossary marks it `→ N/A (no chapter yet)`. Cimmeria does not need to emulate MachineGuard for client compatibility — it is internal server-machine discovery.

### 1.14 Wire-format divergences from stock BigWorld 2.0.1 — consolidated

Every SGW divergence from stock BigWorld 2.0.1 affecting Mercury wire format, in one place:

| Surface | Stock BigWorld 2.0.1 | SGW |
|---|---|---|
| Packet flags | `uint16` (2 bytes), network order | `uint8` (1 byte) |
| Footer byte order | Network (big-endian) via `BW_HTONS` / `BW_HTONL` | Little-endian |
| Encryption | Blowfish ECB + XOR chaining + `0xdeadbeef` magic + wastage byte | AES-256-CBC + HMAC-MD5 |
| Encryption KDF | (Blowfish key from session setup) | None — 32-byte SOAP `SessionKey` used verbatim as both AES and HMAC key |
| IV | (Blowfish ECB has no IV) | 16-byte zero IV, reused every packet |
| Cipher library | (BW-internal Blowfish) | CryptoPP (`HMAC<Weak1::MD5>`, `Rijndael::Enc`, `CBC_Encryption`) |
| Sub-slot method threshold (§1.8) | 62 in `checkExposedForSubSlots()` | 62 (identical) — no SGW divergence here despite earlier drafts claiming a one-lower threshold |
| Base (proxy) method wire shape (§1.8) | `[msg_id][u16 len][u32 entityId][args]` per stock BW | `[msg_id][u16 len][args]` — proxy methods do not write an entity ID (`startProxyMessage` at `0x00dd6980`) |
| `REPLY_MESSAGE (0xFF)` length type (§1.9) | `DWORD_LENGTH` (stock-BW reference) | `WORD_LENGTH` per `space-viewport-wire-formats.md` |
| `enableEntities` payload (§1.9) | 1 byte (`uint8 dummy`) | 8 bytes (`uint64 dummy`) |
| `createBasePlayer` class field (§1.10.1) | `uint16` (2 bytes) | `uint16` on the wire (same width as stock); server-source style writes it as `(u8 classId)(u8 propCount = 0)` — a code-style difference, not a wire divergence |
| `createCellPlayer` rotation (§1.10.2) | `roll, pitch, yaw` (`Direction3D` order) | `rotX, rotZ, rotY` (Y/Z swapped) — at this message's wire offsets only; not a protocol-wide convention |
| `forcedPosition` payload (§1.10.6) | 36 bytes (entityID + spaceID + vehicleID + pos + direction) | 49 bytes (adds velocity `Vec3` + physics `u8`) |
| `forcedPosition` rotation order (§1.10.6) | `roll, pitch, yaw` | Per call site: world-entry path writes `rotX, rotZ, rotY` (Y/Z swapped); standalone `forcedPosition()` writes `rotation.x, rotation.y, rotation.z` (caller's responsibility) |
| `detailedPosition` rotation order (§1.11.2) | `roll, pitch, yaw` (stock-BW `Direction3D`) | `roll, pitch, yaw` — **no Y/Z swap** for this message, unlike `forcedPosition` and `createCellPlayer`. Rotation order is per-message-site, not protocol-wide |
| `detailedPosition` payload (§1.11.2) | (stock-BW analog full-precision) | 41 bytes — adds trailing physics-mode `u8` (same SGW addition as `forcedPosition`) |
| `spaceViewportInfo` size (§1.10.4) | Variable (viewport-owner / viewport-target distinct) | Fixed `CONSTANT_LENGTH = 13`; both entity-ID fields equal during open; `entityId2 = 0` closes the viewport |
| `AUTHENTICATE` length type (§1.10.7) | (BW-internal Blowfish handshake) | `DWORD_LENGTH`, packed-string body |
| `RESET_ENTITIES` bundling (§1.9) | Bundled freely | Must be in its own flushed bundle per `entity-creation-wire-formats.md` |
| `bandwidthNotification` / `spaceData` (§1.9.1, §1.10.3) | Active in stock BW | Registered but **unused** in SGW — handlers exist, behavior is no-op |
| `FLAG_HAS_CHECKSUM` | Available (CRC32 in footer) | Omitted (HMAC-MD5 supersedes) |
| `FLAG_HAS_CUMULATIVE_ACK` | Available | Omitted |
| `FLAG_INDEXED_CHANNEL` | Available (indexed-channel routing) | Reserved-unused; bit 7 means `FLAG_IS_FRAGMENT` |
| Piggyback packets | Generated and consumed | Format inherited; Cimmeria Rust rejects on receive |

The divergences cluster in three themes: **security** (Blowfish → AES + HMAC), **wire compactness** (uint16 flags → uint8, omitted flags), and **wire-format simplifications** for SGW's narrower set of supported gameplay modes (no indexed channels, no multi-viewport, no checksum-redundant footer field). The footer-byte-order divergence is the one most likely to silently break a reimplementation; the `enableEntities` 8-byte divergence is the most-contested historically; the per-call-site rotation order in `forcedPosition` is the most subtle.

### 1.15 Source-of-truth crosswalk

For section-by-section verification of every claim above:

| Claim | Primary V5 source | Secondary cross-check |
|---|---|---|
| 1-byte flags, footer little-endian | `mercury-protocol-internals.md` §"Packet Flags Byte" | `protocol-comparison.md` (stock-BW comparison reference) |
| 8-bit flag definitions | `mercury-protocol-internals.md` §"Packet Flags Byte" | `external/BigWorld-2.0.1/src/lib/network/packet.hpp` low byte |
| AES-256-CBC + HMAC-MD5, zero IV, no KDF | `mercury-protocol-internals.md` §"Cipher Key Derivation (Session 5 Verification)" | RTTI strings at `0x01e93b70`–`0x01ea3c5c` (`HMAC_Base@CryptoPP`, `HMAC@VMD5@Weak1@CryptoPP`, `Rijndael::Enc@CryptoPP`) |
| Bundle/packet/message functions | `mercury-protocol-internals.md` §"4 Target Functions" | `mercury-protocol-internals.md` §"All Mercury Functions" address inventory |
| InterfaceElement length encoding (CONSTANT/WORD/DWORD) | `mercury-protocol-internals.md` §"InterfaceElement" | InterfaceElement table addresses (`0x0158acc0`, `0x0158b770`, `0x0158b120`) |
| Bundle fragmentation, 64-packet cap | `mercury-protocol-internals.md` §"Implications for Cimmeria" | `external/BigWorld-2.0.1/src/lib/network/packet.hpp` (`Packet::MaxFragmentsPerBundle`) |
| 28-bit sequence numbers | `mercury-protocol-internals.md` §"Protocol Constants" | — |
| Sub-slot threshold = 62 (§1.8) | `entity-property-sync.md` §13 ("Sub-Slot Client Method Encoding — Final Confirmation") | `external/BigWorld-2.0.1/src/.../entity_method_descriptions.cpp` (`checkExposedForSubSlots()`) |
| Cell vs base entity-message wire shape (§1.8) | `system-protocol-wire-formats.md` §"startEntityMessage / startProxyMessage" | — |
| `enableEntities` 8-byte payload (§1.9) | `world-entry-pipeline.md` §"ENABLE_ENTITIES Payload Reconciliation" | `deprecated/cpp/src/baseapp/mercury/sgw/messages.cpp:83` |
| `resetEntities` 1-byte payload + own-flushed-bundle (§1.9) | `entity-creation-wire-formats.md` §"1. RESET_ENTITIES (0x04)" | Initializer at `0x017bb200`; `space-viewport-wire-formats.md` §"RESET_ENTITIES (0x04)" |
| `RESOURCE_FRAGMENT` byte layout (§1.9) | `space-viewport-wire-formats.md` §"RESOURCE_FRAGMENT (0x36)" | — |
| `REPLY_MESSAGE` (`0xFF`) is `WORD_LENGTH` (§1.9) | `space-viewport-wire-formats.md` §"REPLY_MESSAGE (0xFF)" and §"Complete Server Message Table" | — |
| `createBasePlayer` 6-byte payload, `u16` class field (§1.10.1) | `entity-creation-wire-formats.md` §"2. CREATE_BASE_PLAYER (0x05)" | `system-protocol-wire-formats.md` §"CREATE_BASE_PLAYER (0x05) -- Stream Read Details" (Ghidra `MOVZX EAX, word ptr [EAX]`) |
| `createCellPlayer` 32-byte payload + Y/Z swap (§1.10.2) | `entity-creation-wire-formats.md` §"3. CREATE_CELL_PLAYER (0x06)" | `world-entry-pipeline.md` §"Audit Findings vs `world-entry-phases.md`"; rotation reader `FUN_015846a0` |
| `spaceViewportInfo` 13-byte fixed payload + close-viewport (§1.10.4) | `entity-creation-wire-formats.md` §"4. SPACE_VIEWPORT_INFO (0x08)" | `space-viewport-wire-formats.md` §"SPACE_VIEWPORT_INFO (0x08)" — viewport operations table |
| `forcedPosition` 49-byte fixed payload + physics byte (§1.10.6) | `entity-creation-wire-formats.md` §"5. FORCED_POSITION (0x31)" | `position-movement-wire-formats.md` §"forcedPosition (msg_id 0x31, 49 bytes)"; `space-viewport-wire-formats.md` §"FORCED_POSITION (0x31)" |
| `forcedPosition` rotation order per call site (§1.10.6) | `entity-creation-wire-formats.md` §"5. FORCED_POSITION (0x31)" — two C++ call sites | `system-protocol-wire-formats.md` §"FORCED_POSITION (0x31) -- Rotation Order Evidence" |
| `AUTHENTICATE` (`0x00`) is `DWORD_LENGTH` (§1.10.7) | `system-protocol-wire-formats.md` §"AUTHENTICATE (0x00) -- Server-to-Client Key Exchange" | `space-viewport-wire-formats.md` §"Complete Server Message Table" |
| `bandwidthNotification` 4-byte payload, unused (§1.9.1) | `space-viewport-wire-formats.md` §"BANDWIDTH_NOTIFICATION (0x01)" | `messages.cpp:134` server emit |
| `updateFrequencyNotification` 1-byte resolution (§1.9.2) | `space-viewport-wire-formats.md` §"UPDATE_FREQUENCY_NOTIFICATION (0x02)" | `client_handler.cpp:46-53` server emit (`uint8_t updateFreq = 1000 / tickRate`) |
| `setGameTime` 4-byte u32 ticks (§1.9.3) | `space-viewport-wire-formats.md` §"SET_GAME_TIME (0x03)" | `system-protocol-wire-formats.md` §"TICK_SYNC (0x0D) and SET_GAME_TIME (0x03) -- RTTI Evidence"; descriptor at `0x017bb180` |
| `tickSync` 8-byte (gameTime + tickRate) (§1.9.4) | `entity-creation-wire-formats.md` §"8. TICK_SYNC (0x0D)" | `space-viewport-wire-formats.md` §"TICK_SYNC (0x0D)"; descriptor at `0x017bb720` |
| `restoreClient` 48-byte body + auto-reply (§1.9.5) | `system-protocol-wire-formats.md` §"RESTORE_CLIENT (0x34) -- Client State Restore" | `space-viewport-wire-formats.md` §"RESTORE_CLIENT (0x34)" |
| `loggedOff` 1-byte reason, silent disconnect (§1.9.6) | `system-protocol-wire-formats.md` §"LOGGED_OFF (0x37) -- Server Disconnect" | `space-viewport-wire-formats.md` §"LOGGED_OFF (0x37)" |
| `spaceData` 14+-byte payload, unused (§1.10.3) | `space-viewport-wire-formats.md` §"SPACE_DATA (0x07)" | `messages.cpp:189-190` server registration |
| `createEntity` 5-byte payload (§1.10.5) | `entity-creation-wire-formats.md` §"6. CREATE_ENTITY (0x09)" | `space-viewport-wire-formats.md` §"CREATE_ENTITY (0x09)" |
| `UPDATE_AVATAR` variants — 32 entries, 2×4×4 matrix (§1.11.1) | `space-viewport-wire-formats.md` §"UPDATE_AVATAR variants (0x10 - 0x2F)" and §"All 32 Variant Sizes" | `position-movement-wire-formats.md` §"avatarUpdate Messages (msg_id 0x10-0x2F)" |
| `detailedPosition` 41-byte payload (§1.11.2) | `position-movement-wire-formats.md` §"detailedPosition (msg_id 0x30, 41 bytes)" | `space-viewport-wire-formats.md` §"DETAILED_POSITION (0x30)" |
| MachineGuard message types | `mercury-protocol-internals.md` §"MachineGuard Protocol" | — |
| Mercury::Nub construction | `mercury-protocol-internals.md` §"Mercury::Nub" address table | — |

### 1.16 Open questions

Five unresolved questions remain. Each has a state, a path to resolution, and a description of what stays uncertain until it lands. (Earlier drafts of this chapter carried questions about `createBasePlayer` `typeID` width, reply-ID endianness, `RESOURCE_FRAGMENT` byte layout, and `spaceViewportInfo` second-entityId semantics — all four have been resolved by the V5 evidence corpus surfaced during the 2026-05 review pass and folded into Sections 1.8–1.10.)

#### Q1 — InterfaceElement compressed-length thresholds (§1.5)

**Question:** What are the exact byte threshold values at which `InterfaceElement::compressLength_write` switches between 1-byte, 2-byte, 3-byte, and 4-byte representations?

**State:** The three functions (`compressLength`, `expandLength`, `compressLength_write`) are V5-confirmed and addressed at `0x0158acc0`, `0x0158b770`, `0x0158b120`. Their threshold constants are not enumerated in `mercury-protocol-internals.md`. The closest analog is `ProcessMessage::writeComponentsVarLen` at `0x01586180` (single threshold at `0xfe`).

**Path to resolution:** Decompile `0x0158b120` and read the compare-and-branch constants.

**Impact if unresolved:** Section 1 cannot fully canonize system-message length encoding for `msg_id` slots that use compressed length. Entity messages are unaffected (they always use `WORD_LENGTH`). Confidence stays `medium` for §1.5.

#### Q2 — ChannelInternal `+0x170` / `+0x174` timer fields (§1.7)

**Question:** What are the roles of the timer fields at offsets `+0x170` and `+0x174` of the `ChannelInternal` struct?

**State:** `mercury-protocol-internals.md` Session 5b open question 1 flagged these as "additional timer fields whose role is TBD." Three other timer fields at `+0x160`, `+0x164`, and `+0x16c` are role-confirmed (recv timeout threshold, recv timeout last-check timestamp, send-alive timeout). Two more fields exist at adjacent offsets but their roles in `ChannelInternal::checkAndSendNubException` at `0x0158bed0` were not chased.

**Path to resolution:** Decompile `checkAndSendNubException` and follow the read sites for `+0x170` and `+0x174`.

**Impact if unresolved:** Section 1's §1.7 reliability-state subsection is canon for the three confirmed timer fields but admits "additional timer fields (role TBD)". A reimplementation that runs without these timer behaviors may diverge from observed Mercury reconnect / keepalive cadence — most likely on long-idle channels.

#### Q3 — `forcedPosition` velocity `Vec3` semantics outside world entry (§1.10.6)

**Question:** Under what conditions does the server emit a `forcedPosition` with a non-zero velocity `Vec3`? Does the client apply the velocity as a delta-replacement (`entity.velocity = packet.velocity`) or as an additive impulse (`entity.velocity += packet.velocity`)? And does the standalone (non-world-entry) call site at `client_handler.cpp:566-572` emit rotation in `rotation.x, rotation.y, rotation.z` order verbatim, or do its callers pre-swap Y/Z to match what the handler at `0x00dd9ee0` reads positionally?

**State:** §1.10.6 documents that the velocity field is always zero in observed world-entry traffic and that the world-entry call site (`client_handler.cpp:407-413`) writes Y/Z swapped. The standalone call site writes `rotation.x, rotation.y, rotation.z` per the C++ source, but the client handler interprets offsets 36/40/44 positionally — so either the standalone callers are pre-swapping their `rotation` argument (matching the world-entry wire convention) or the V5 evidence's "caller's responsibility" comment hides a wire-format bug in SGW's standalone path. No pcap capture of an in-flight position correction is currently in the V5 record.

**Path to resolution:** (a) Capture a pcap of a gate-travel or anti-cheat snap and compare bytes at offsets 36/40/44 against the entity's known orientation. (b) Cross-reference every call site of the standalone `ServerConnection::forcedPosition()` in `deprecated/cpp/...` and check whether callers pre-swap. (c) Decompile `ServerConnection_addMove` at `0x00dd9330` to confirm the receive-side rotation interpretation matches §1.10.6.

**Impact if unresolved:** Position-snap behavior outside world entry is currently underdocumented. A reimplementation that always emits zero velocity and Y/Z-swapped rotation at *every* `forcedPosition` call site will match observed world-entry behavior but may produce rotation-incorrect snaps if the standalone path on SGW's deprecated server actually emitted the unswapped order to a client that read it positionally as Y/Z-swapped. Confidence on §1.10.6 stays high for world entry, medium for non-world-entry snaps.

#### Q4 — MachineGuard port hex/decimal mismatch (§1.13)

**Question:** What is the actual MachineGuard listen port — `0x4E36` (20022), `0x4C36` (19510), or some third value? Upstream V5 docs pair `0x4e36` with decimal `19510`, but those two values do not match: `0x4E36 = 20022` and `0x4C36 = 19510`.

**State:** `mercury-protocol-internals.md` §"MachineGuard Protocol" carries the pair `0x4e36 (19510)` and `docs/reverse-engineering/v5-campaign/CAMPAIGN_STATUS.md` echoes the same pair, propagating the inconsistency. No direct read of the port constant from the MachineGuard listen-socket bind site is currently in the V5 record. Both interpretations are plausible — the hex could be a transcription typo for `0x4C36`, or the decimal could be a transcription typo for `20022`.

**Path to resolution:** Locate the `bind()` or `setsockopt()` call site for the MachineGuard UDP listener (likely near `0x01588530` or in the MachineGuard initializer chain), read the literal port constant, and reconcile against the SGW deployment scripts under `external/SGW-server-binary/` for any matching port number in the launch configuration.

**Impact if unresolved:** §1.13 cannot pin the MachineGuard port to high confidence. Cimmeria does not need to emulate MachineGuard for client compatibility (it is internal server-machine discovery), so this question is documentation-correctness, not implementation-blocking. Confidence on §1.13's port row stays medium until directly verified.

#### Q5 — `ChannelInternal` send-window slot count (§1.2, §1.6)

**Question:** What is the exact slot count of the `ChannelInternal` reliable-packet send window? Earlier drafts of this chapter named "45 slots" / "45 packets" in two places (§1.2's `FLAG_IS_RELIABLE` paragraph and §1.6's "Send window" paragraph). The literal `45` is not enumerated in the V5 record — `mercury-protocol-internals.md` §"Channel Internal Layout" describes the `UnAckedHandler` hash-table region at `ChannelInternal+0x40/+0x44` as the backing structure for the send window, but does not pin a slot count.

**State:** The send window's *existence* is high confidence (the hash-table region is V5-anchored), but the *capacity* is not. The number 45 may be an inherited-from-stock-BW value, a transcription from an external doc, or a guess that propagated forward. This chapter currently uses the phrase "fixed-size circular buffer" in §1.2 and §1.6 with a medium-confidence flag.

**Path to resolution:** Decompile the `ChannelInternal` constructor at `ghidra://SGW.exe@0x0158c7b0` and look for an allocation site that allocates `N × sizeof(slot)` bytes — the literal `N` is the answer. Cross-check against `UnAckedHandler::buildAndSendAckBundle` at `0x0158b2d0` for a loop bound that matches.

**Impact if unresolved:** Section 1's reliability-state subsection cannot pin the exact size of the in-flight reliable-packet window. A reimplementation will work correctly as long as its window is large enough for normal bundle sizes (the largest observed bundle is the world-entry mapLoaded bundle at ~5 packets) and ≤ the actual stock-BW or SGW capacity. Mismatches only become observable under extreme bundle-size pressure or sustained high-fan-out broadcasts.

---

## Section 2 — Client findings

N/A — pending Section 1 review. The client's expectation of the Mercury wire format is implicit in its packet decoder, which lives in the same SGW.exe binary already cited in Section 1. This section will catalogue what the *client-side configuration* (`game/sgw/Working/SGWGame/Config/*.ini`) and any compiled UnrealScript wire-handling code expect at parameters (MTU, ack timeout, retry limits, channel idle timeout). The cipher envelope's session-key delivery is a client-side artifact (Phase 1/2 of world entry; covered in `spec.protocol.cipher-and-auth`), so Section 2 will focus narrowly on the wire-format-relevant client surface — not the auth handshake.

---

## Section 3 — Deprecated server

N/A — pending Section 1 review. `deprecated/cpp/src/baseapp/mercury/sgw/` is the legacy implementation; this section will reconstruct what the original C++ Mercury did wire-side from `messages.hpp` / `messages.cpp` / `client_handler.cpp` and flag the small set of behaviors Cimmeria intentionally diverges from. The 8-byte `ENABLE_ENTITIES` (line 83 of `messages.cpp`) and the 49-byte `forcedPosition` are SGW-custom and need explicit calling-out as preserved behaviors.

---

## Section 4 — Expected implementation in Rust

N/A — pending Section 1 review. Derived from Sections 1–3; will name the Rust symbols that must encode/decode each wire shape, using the no-line-numbers rule (`cimmeria-mercury::packet::Packet::deserialize`, `cimmeria-mercury::bundle::Bundle::finalise`, `cimmeria-mercury::encryption::MercuryEncryption::from_session_key`, etc.).

---

## Section 5 — Actual implementation in Rust

N/A — pending Section 1 review. Catalogues current Rust state in `crates/mercury/` and `crates/services/src/mercury/`, flags divergences from Section 4. The known item to verify before authoring: the `encryption.rs` doc-comment that says "OpenSSL" — should say "RustCrypto" (and the implementation it's emulating uses CryptoPP, not OpenSSL).
