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

This section distills `docs/reverse-engineering/findings/mercury-protocol-internals.md` (the canonical V5 finding doc) plus the ENABLE_ENTITIES reconciliation in `docs/reverse-engineering/findings/world-entry-pipeline.md`. Every claim below resolves to a Ghidra anchor in `SGW.exe` (image base `0x00400000`); the address-map provides the persistent symbol table.

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

**Maximum packet size**: `0x5AD` bytes (1453). Stamped at `ghidra://SGW.exe@0x0157ac90` as the per-packet space check in `Mercury::Bundle::newMessage` — a message that does not fit triggers `Bundle::reserve` (`ghidra://SGW.exe@0x0157a5d0`) to allocate a new packet, and the bundle fragments across packets when the bundle's total exceeds 64 packets (`Packet::MaxFragmentsPerBundle`). The 1453-byte cap is the Ethernet MTU of 1500 less the IP header (20), UDP header (8), and a margin for the AES tag + HMAC tag (32 bytes combined when encryption is enabled).

**The packet object lives at a stable offset across the codebase.** The packet flags byte is stored at offset `+0x54` of the in-memory `Mercury::Packet` struct, but on the wire it occupies byte offset 0 of the datagram. Mercury writes the in-memory struct contiguously and the +0x54 offset is purely a serialization artifact of the surrounding struct fields. Confidence: high — this matches stock BigWorld `external/BigWorld-2.0.1/src/lib/network/packet.hpp`.

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

**One flag is reserved-unused.** `FLAG_INDEXED_CHANNEL` would occupy bit 7 in stock BW (clashing with SGW's `FLAG_IS_FRAGMENT` bit 7); SGW never advertises indexed channels because the SGW baseapp connection topology does not use them. Bit 7 unambiguously means `FLAG_IS_FRAGMENT` in SGW.

**`FLAG_IS_RELIABLE` (bit 4) is the load-bearing flag for the entire reliability layer.** When set, the sender's `ChannelInternal` (the ~0x180-byte inner channel object at `ghidra://SGW.exe@0x0158c7b0`) places the packet into a 45-slot send window and starts a 700ms resend timer; the receiver schedules an ack via `UnAckedHandler::queueAckForPacket` at `ghidra://SGW.exe@0x0158cba0`. When clear, the packet is fire-and-forget — used for position-update spam and unreliable bundle flushes.

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

#### 1.3.2 Piggyback chain encoding

Piggybacks are *whole previously-sent packets* embedded in the footer area of a new outgoing packet. Format from stock BW (SGW inherits the layout; both ends of the protocol parse the same wire bytes):

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
| `WORD_LENGTH` | 2 bytes (`u16` LE) | Variable-size payload, typical entity-method call |
| `DWORD_LENGTH` | 4 bytes (`u32` LE) | Variable-size payload, only for reply messages (`msg_id = 0xFF`) |

The `InterfaceElement` table is a static array of 0x90-byte (144-byte) entries — each entry's `lengthType` field selects which of the three formats to use when parsing or emitting a message of that ID. At runtime, the Nub builds a parallel array of 0x24-byte (36-byte) runtime entries indexed directly by `msg_id`, populated from the static array. The runtime entries are read by `Mercury::Nub::processOrderedPacket` at `ghidra://SGW.exe@0x0157c820` on every incoming message.

**Static vs runtime layout, side by side.** The 0x90-byte static `InterfaceElement` entries carry the full message descriptor — name string, length type, payload-size hint, handler pointer, reliability flag, encryption-required flag, and assorted metadata. At Nub initialization, the static entries are *projected* into a smaller 0x24-byte runtime form keyed by `msg_id`: only the runtime-hot fields are kept (`lengthType`, `lengthValue`, `handler*`, `isEntityMessage` flag). The runtime array's index is the `msg_id` byte itself, so a dispatch is a single `nub->elements[msg_id]` load — no name-based lookup, no hash. The 256 `msg_id` slots map: 0x00–0x7F to system-message slots, 0x80–0xFD to entity-method slots (with `0xBD` and `0xFD` reserved as the sub-slot extended-encoding sentinels), and `0xFF` to the reply-message slot.

**Entity messages override the table.** Any message with `msg_id >= 0x80` is an entity-method or property message and *always* uses `WORD_LENGTH`, regardless of the table's declared length type for that ID. This is enforced in `BundleUnpacker::next` (decode side) and in `Mercury::Bundle::newMessage` at `ghidra://SGW.exe@0x0157ac90` (encode side). The reason: entity messages carry their own variable-size argument list whose total size cannot be known statically.

**Compressed-length encoding for interface elements with extreme size variation.** A separate variable-width scheme exists for the rare case where a message's payload size is usually small (fits in 1 byte) but must occasionally extend to a wider field. The `InterfaceElement::compressLength` family handles the switch:

| Function | Address | Role |
|---|---|---|
| `InterfaceElement::compressLength` | `ghidra://SGW.exe@0x0158acc0` | Decide compressed-length width from value |
| `InterfaceElement::expandLength` | `ghidra://SGW.exe@0x0158b770` | Read compressed-length field at parse time |
| `InterfaceElement::compressLength_write` | `ghidra://SGW.exe@0x0158b120` | Write compressed-length field at emit time |

**Confidence: medium.** The functions are V5-confirmed; their *byte-threshold constants* are not. `mercury-protocol-internals.md` names the three functions but does not enumerate the threshold values that decide which width to emit. The 1-byte path is confirmed; the wider-byte path exists but the switch threshold is unverified. The closest comparable scheme in the same binary is `ProcessMessage::writeComponentsVarLen` at `ghidra://SGW.exe@0x01586180` (the MachineGuard component-ID encoder), which uses a single threshold: IDs `≤ 0xfe` are written as 1 byte; IDs `> 0xfe` are written as `0xff` prefix + 3 bytes. The InterfaceElement scheme *may* be similar, but the bible canonizes evidence not analogy — pin to `medium` until `0x0158b120` is decompiled and the threshold constants are extracted. See open Q1 in §1.15.

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

**Send window.** A reliable bundle's fragments each occupy one slot in the channel's 45-packet send window. A bundle larger than 45 packets (~65 KB of payload) cannot be sent without window stalling. In practice, bundles are tens of packets at most — the largest observed is the world-entry mapLoaded bundle at 27+ interface-element calls, which fits in ~5 packets.

### 1.7 Sequence numbers and reliability

Mercury sequence numbers are **28-bit** (`SEQ_SIZE = 0x10000000`). The space is 256M sequence IDs before wrap; the wrap is handled by modular arithmetic in the comparison routines. A reliable packet's sequence ID is assigned by `Mercury::Channel::send` at `ghidra://SGW.exe@0x01576f90` from a monotonic per-channel counter.

| Constant | Value | Source |
|---|---|---|
| Sequence number mask | `0x0FFFFFFF` | mercury-protocol-internals.md §"Protocol Constants" |
| Null sequence number | `0x10000000` | mercury-protocol-internals.md §"Protocol Constants" |
| Max retries | 20 | stock BW default, matches Cimmeria implementation |
| Ack timeout | 700 ms | stock BW default, matches Cimmeria implementation |

`0x10000000` is the null-sentinel: a packet with this sequence ID has no sequence number assigned (used for unreliable bundles that don't go in the send window). Because `0x10000000` is the very next value above the 28-bit `0x0FFFFFFF` mask, no real sequence number can collide with the sentinel.

**Reliability state lives in `ChannelInternal`**, the ~0x180-byte inner channel object constructed at `ghidra://SGW.exe@0x0158c7b0`. The send window is a fixed-size circular buffer; entries are cleared by `processAck` when their sequence ID is acknowledged. The window head slides forward only when its head slot is empty (acked or never used). A receiver's processing of incoming acks runs even when the incoming packet's own sequence ID is outside its receive window — this prevents lost acks from causing unbounded retransmissions.

**Resend timing.** `ChannelInternal::checkAndSendNubException` at `ghidra://SGW.exe@0x0158bed0` runs the timer-driven resend logic. Three rdtsc-based timeout fields live in the channel object:

| Offset | Role |
|---|---|
| `+0x160` | Receive timeout threshold (rdtsc units) |
| `+0x164` | Receive timeout last-check timestamp |
| `+0x16c` | Send-alive timeout — triggers a keepalive ack bundle if no traffic |
| `+0x170`, `+0x174` | Additional timer fields (role TBD per `mercury-protocol-internals.md` Session 5b open question 1) |

When the send-alive timer expires, `UnAckedHandler::sendAckBundle2` at `ghidra://SGW.exe@0x0158bbc0` builds an empty bundle with the reliable flag set, just to keep the channel alive. This is the Mercury keepalive — not a separate keepalive packet type.

### 1.8 Message dispatch

After packet reassembly, each interface element message in the bundle is dispatched by `Mercury::Nub::processOrderedPacket` at `ghidra://SGW.exe@0x0157c820`. The dispatch is a single lookup against the runtime `InterfaceElement` array:

```text
InterfaceElement* elem = &nub->elements[msg_id];   // this+0xc + msg_id * 0x24
elem->handler->handleMessage(msg);
```

Three classes of message ID exist:

| Range | Class | Length encoding | Wire shape |
|---|---|---|---|
| `0x00 – 0x7F` | System messages (auth, sync, control) | Per-table (`CONSTANT_LENGTH` / `WORD_LENGTH` / `DWORD_LENGTH`) | `[msg_id][length?][payload]` |
| `0x80 – 0xBD` | Cell entity method calls (and `0xBD` extended) | Always `WORD_LENGTH` | `[msg_id][u16 length][u32 entityId][args]` |
| `0xC0 – 0xFD` | Base entity method calls (and `0xFD` extended) | Always `WORD_LENGTH` | `[msg_id][u16 length][u32 entityId][args]` |
| `0xFF` | Reply message | Always `DWORD_LENGTH` | `[0xFF][u32 length][u32 replyId][reply data]` |

Entity messages: the first method's `msg_id` byte encodes the method index directly (`methodId | 0x80` for cell, `methodId | 0xC0` for base). For method indices `≥ 61` (`0x3D`), the encoding switches to *extended*: the `msg_id` byte is the sentinel `0xBD` (cell) or `0xFD` (base), and an extra `u8` carrying `sub_index = methodId - 61` follows the `entityId` field. Method index 60 is the highest index that uses direct encoding (`msg_id = 0xBC` for cell, `msg_id = 0xFC` for base); method index 61 is the first index that uses extended encoding (`msg_id = 0xBD`, `sub_index = 0`).

The threshold = 61 claim is V5-confirmed against `world-entry-pipeline.md`'s extended-encoding worked example (`onClientMapLoad` is method index 117; the wire shows `[msg_id = 0xBD][...][sub_index = 56]` = `117 - 61`). Stock BigWorld's threshold is 62 in `external/BigWorld-2.0.1/src/lib/connection/baseapp_ext_interface.hpp` because stock BW reserves one extra slot for its own purposes; SGW does not, so its threshold sits one lower at 61. The full sub-slot mechanism — and the reason the parser zero-indexes its dispatch site such that the on-wire threshold is 61 — is canonized in `spec.engine.entity-description-parse-chain`.

**Worked example of direct vs extended entity dispatch.** A call to `onStatUpdate` (cell method index 20) on entity ID `0xCAFEBABE` with 3 bytes of arguments:

```text
[0x94]                  ← msg_id = 20 | 0x80 = 0x94 (direct encoding)
[0x07 0x00]             ← word_len = 7 (u16 LE)  — 4 bytes entityId + 3 bytes args
[0xBE 0xBA 0xFE 0xCA]   ← entityId = 0xCAFEBABE (u32 LE)
[arg0 arg1 arg2]        ← serialized args
```

A call to `onClientMapLoad` (cell method index 117 — above the 61 threshold) on the same entity:

```text
[0xBD]                  ← msg_id = 0xBD (extended-encoding sentinel for cell)
[len_lo len_hi]         ← word_len = 4 (entityId) + 1 (sub_index) + N (args)  — u16 LE
[0xBE 0xBA 0xFE 0xCA]   ← entityId = 0xCAFEBABE (u32 LE)
[0x38]                  ← sub_index = 117 - 61 = 56 (u8)
[args...]               ← serialized args
```

The extended encoding costs 1 extra byte per call (the sub_index byte) and is required for any method whose index exceeds 60. Roughly 96 of the 157 client methods on `SGWPlayer` use extended encoding because the parsed order pushes most actual gameplay methods past the threshold.

**Reply messages** use `msg_id = 0xFF` (`REPLY_MESSAGE_IDENTIFIER`) with `DWORD_LENGTH` (4-byte length prefix). The reply ID itself (assigned by the requester via `RequestManager::addReplyOrders` during send and stored in network byte order in stock BW — pin to medium confidence on SGW endianness here, see open question) is the first 4 bytes of the reply body. Matching is done by `Mercury::Nub::handleMessage` at `ghidra://SGW.exe@0x0157bd30`.

### 1.9 Control messages

A small set of system messages drives the connection's lifecycle. Each has a fixed `InterfaceElement` descriptor registered at static-init time and a binding to a specific server-side handler.

#### `enableEntities` (base method index 1, client → server)

| Property | Value |
|---|---|
| Message size | **8 bytes** (`CONSTANT_LENGTH = 8`) |
| Payload | `uint64 dummy = 0` |
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

#### `RESOURCE_FRAGMENT` (system message)

Used to stream cooked-data resources (PAK fragments) from server to client. Each fragment carries one chunk of a resource that the client reassembles before passing the whole resource to the cooked-data pipeline.

**Confidence: low** for the byte-level fragment layout. The address-map entry `0x004269f0` points at `CME_MemberCallback_Ctor_ServerSource_NetProxyData` for `Event_Net_ProxyData` (the CME event that delivers RESOURCE_FRAGMENT to subscribers), but **the wire-format of the RESOURCE_FRAGMENT message itself is not enumerated in `mercury-protocol-internals.md`** and has not been read directly from the binary's InterfaceElement table. The bible canonizes evidence not analogy — until the descriptor is read, the only claim that survives Section 1 scrutiny is that the message exists, is delivered via Mercury, and reaches the CME event at `0x004269f0`. Byte layout is deferred to a future revision. See open Q3 in §1.15.

> [!NOTE] **Cimmeria implementation note (not Section 1 evidence).** The Cimmeria Rust implementation in `crates/services` decodes RESOURCE_FRAGMENT with a `u16` length prefix (per the `resource_fragment_uses_u16_length_prefix` regression test) — chosen because it matches the entity-message `WORD_LENGTH` baseline. This implementation choice does not constitute evidence for the binary's behavior; it represents the current best guess and is what gets revised if the binary descriptor turns out to use a different width. Section 4 of this chapter (when authored) will state the expected width; Section 5 will note where the current implementation matches or diverges.

#### Reply messages (`msg_id = 0xFF`)

```text
[0xFF: u8] [length: u32 LE] [replyId: u32 LE] [reply data]
```

The reply ID matches a pending request registered by the original sender via `Bundle::startMessage_request` at `ghidra://SGW.exe@0x0157adc0`. The next-request-offset linked-list in the request packet's footer (`FLAG_HAS_FIRST_REQUEST_OFFSET` at bit 0, `firstRequestOffset` field in the footer) lets the receiver walk all request messages in a packet without having to look at message bodies.

**Reply-ID endianness is an open question.** Stock BigWorld writes the reply-ID field in network byte order via the `BW_HTONL` macro. SGW's footer is little-endian (a confirmed divergence — see §1.3.3), but **whether SGW also flipped the reply-ID field to little-endian or kept stock BW's network order is not enumerated in any V5 finding**. The two interpretations differ in whether the reply ID `0x12345678` appears on the wire as `78 56 34 12` or `12 34 56 78`. Confidence: medium. See open Q2 in §1.15.

### 1.10 Entity creation and position messages

A small set of system messages carries the wire-level entity lifecycle: creating the player's base proxy, creating the cell proxy, attaching it to a space viewport, and the authoritative position-snap mechanism. Each has a fixed `InterfaceElement` descriptor and a Ghidra-anchored handler in the SGW client. The full canonical wire-formats live below; the entries in the §1.13 divergence consolidation table reference these subsections.

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

**Wire layout** per `world-entry-pipeline.md` §"Phase 3 — CREATE_BASE_PLAYER":

```text
[msg_id:    0x05]        1 byte
[word_len:  u16 LE = 6]  2 bytes  (payload size)
[entityId:  u32 LE]      4 bytes  — player entity ID assigned by BaseApp
[classId:   u8 = 0x02]   1 byte   — SGWPlayer (entities.xml position 2)
[propCount: u8 = 0]      1 byte   — no initial properties
```

**`classId` size — a V5 internal contradiction.** The wire-layout description in `world-entry-pipeline.md` §"Phase 3" shows `[classId: u8][propCount: u8]` (two 1-byte fields summing to 6 bytes with the 4-byte `entityId`). The same finding doc's adjacent "Audit Findings" table and address-map describe the handler `ServerConnection_CreateBasePlayer` at `0x00dddca0` as "reads entityId u32 + typeId u16" — implying the field is 2 bytes wide. Both interpretations produce a 6-byte payload that matches `word_len = 6`:

| Interpretation | Layout | Total |
|---|---|---|
| **A** (wire-layout doc, u8) | `entityId(4) + classId(1) + propCount(1)` | 6 bytes |
| **B** (audit-findings doc, u16) | `entityId(4) + classId(2)` | 6 bytes |

Decompiling `0x00dddca0` is what settles this — until then, the wire-layout doc and the audit-findings table point at different shapes for the same payload, and §1.13's divergence row inherits the ambiguity. Confidence: medium pending direct binary read. See open Q4 in §1.15.

**Divergence from stock BigWorld 2.0.1.** Stock BW's `createBasePlayer` per `external/BigWorld-2.0.1/src/lib/connection/baseapp_ext_interface.hpp` carries an `EntityID entityID; EntityTypeID type;` pair where `EntityTypeID` is `uint16`. SGW's wire-layout doc says SGW reduced this to `uint8` plus a `propCount` byte for a 6-byte total — a deliberate compression to fit the propCount byte. The interpretation-B reading suggests SGW kept the 2-byte typeID and omits `propCount` entirely. The divergence's *direction* (SGW differs from stock) is settled; its *shape* awaits Q4 resolution.

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

#### 1.10.3 `spaceViewportInfo` — viewport binding (server → client, msg_id `0x08`)

Tells the client which entity (its own player) is bound to which space viewport. Sent in the same Mercury packet as `createCellPlayer` and `forcedPosition` (see `spec.world.world-entry` for the bundling).

| Property | Value |
|---|---|
| Message ID | `0x08` |
| Length type | `CONSTANT_LENGTH = 13` |
| Payload size | 13 bytes (no length prefix on the wire — fixed) |
| Handler in client | `ServerConnection_SpaceViewportInfo` at `ghidra://SGW.exe@0x00dda6c0` |

**Wire layout** per `world-entry-pipeline.md` §"SPACE_VIEWPORT_INFO":

```text
[msg_id:     0x08]    1 byte
[entityId:   u32 LE]  4 bytes   — player entity ID
[entityId2:  u32 LE]  4 bytes   — same entity ID, repeated
[spaceId:    u32 LE]  4 bytes   — space identifier
[viewportId: u8 = 0]  1 byte    — always 0
```

The repeated `entityId` field is a stock-BW artifact: stock BW's `SpaceViewportInfo` carries both a *viewport-owner* entity ID and a *viewport-target* entity ID. The owner is the entity *owning* the viewport (typically the local player); the target is the entity the viewport is *attached to* — usually the same as the owner, but different when one entity is observing another (spectator camera, replay viewer, GM-overseen client). The viewport-target field lets the server tell the client "render the world from owner X but anchor the camera and AoI to entity Y."

SGW collapses this distinction. Every observed `spaceViewportInfo` packet in the running game sets both fields to the player's own entity ID. The pattern is consistent across all V5-confirmed traffic and matches the deprecated server's emit path (`deprecated/cpp/src/baseapp/mercury/sgw/...`), but it asserts a *negative*: there is no SGW gameplay mode that produces a packet with `entityId ≠ entityId2`. If a future investigation surfaces such a mode (a 2009-era spectate command, a hidden GM-watch feature), the second field would need to carry a distinct value. Treating it as always-equal is correct for current-game emulation; treating it as definitionally-equal would be wrong. See Q7 in §1.15.

`viewportId = 0` is the unique viewport SGW uses; there is no multi-viewport mode in the running game. The field exists on the wire because stock BigWorld supports multiple simultaneous viewports per client (split-screen / picture-in-picture). SGW reserves the field but the running game never reads or emits a non-zero value.

#### 1.10.4 `forcedPosition` — authoritative position snap (server → client, msg_id `0x31`)

The authoritative "you are here" message. Sent by the server when the client's position must be hard-set (world entry, gate travel, anti-cheat correction, teleport). Carries position, velocity, orientation, and a flags byte. Unlike `avatarUpdate` (the client's position-broadcast) or normal entity-method calls, `forcedPosition` is a system-level wire-format message with a fixed 49-byte payload.

| Property | Value |
|---|---|
| Message ID | `0x31` |
| Length type | `CONSTANT_LENGTH = 49` |
| Payload size | 49 bytes (no length prefix on the wire — fixed) |
| Handler in client | `ServerConnection_ForcedPosition` at `ghidra://SGW.exe@0x00dd9ee0` |

**Wire layout** per `world-entry-pipeline.md` §"FORCED_POSITION":

```text
[msg_id:    0x31]        1 byte
[entityId:  u32 LE]      4 bytes
[spaceId:   u32 LE]      4 bytes
[vehicleId: u32 LE = 0]  4 bytes
[posX:      f32 LE]      4 bytes
[posY:      f32 LE]      4 bytes
[posZ:      f32 LE]      4 bytes
[velX:      f32 LE = 0]  4 bytes   — typically 0 at snap time
[velY:      f32 LE = 0]  4 bytes
[velZ:      f32 LE = 0]  4 bytes
[rotX:      f32 LE]      4 bytes
[rotZ:      f32 LE]      4 bytes   — *** Y/Z SWAPPED *** (same convention as createCellPlayer)
[rotY:      f32 LE]      4 bytes   — *** Y/Z SWAPPED ***
[flags:     u8 = 0]      1 byte    — reserved / unused at world entry
```

**The Y/Z swap is the same convention as `createCellPlayer` (§1.10.2).** Both messages carry rotation in `rotX, rotZ, rotY` order — this is the SGW convention for any wire-level rotation triplet, not a per-message divergence. A reimplementation that gets the swap right for `createCellPlayer` and right for `forcedPosition` will get it right for every rotation-carrying message in the protocol.

**Divergence from stock BigWorld 2.0.1.** Stock BW's `forcedPosition` carries 36 bytes: `entityID (4) + spaceID (4) + vehicleID (4) + Position3D (12) + Direction3D (12) = 36`. SGW expands this to 49 bytes by:

1. Inserting a 12-byte velocity Vec3 between position and rotation (always zero at world entry; presumably non-zero for in-flight position corrections — unverified at the wire level).
2. Appending a 1-byte `flags` field (always zero in observed traffic; semantics unverified).

Both additions are SGW-specific. The Cimmeria server must emit the full 49-byte payload; emitting the stock 36-byte payload would fail the `CONSTANT_LENGTH = 49` table check in the client's `InterfaceElement` decoder and the packet would be dropped silently. Confidence: high for the 49-byte total; medium for the semantic role of the velocity Vec3 outside world entry; medium for the flags-byte semantics.

### 1.11 Nub — endpoint object

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

### 1.12 MachineGuard — adjacent machine-discovery protocol

MachineGuard is a *separate* UDP protocol that SGW uses for machine-level service discovery. It is not Mercury — different port, different message types, different deserializer — but it lives in the same binary range (`[0x01585000, 0x0158efff]`) and is sometimes conflated with Mercury in older docs.

| Property | Value |
|---|---|
| Port | `0x4e36` (19510) |
| Master deserializer | `ghidra://SGW.exe@0x01588530` |
| Send raw packet | `ghidra://SGW.exe@0x01588ec0` |
| Message types | 13 |

The master deserializer at `0x01588530` switches on a single type byte (range 0x01–0x0b + 0x40), heap-allocates the matching subtype, and calls its `read()` vtable slot. Message types:

| Type byte | Class | Constructor |
|---|---|---|
| `0x01` | `WholeMachineMessage` | `0x01587d30` |
| `0x02` | `ProcessMessage` | (read at `0x015896d0`, write at `0x01586180`) |
| `0x04` | `ListenerMessage` | `0x01586410` |
| `0x05` | `CreateMessage` | `0x01586590` |
| `0x06` | `SignalMessage` | `0x01586710` |
| `0x07` | `TagsMessage` | `0x01587ef0` |
| `0x0b` | `ErrorMessage` | `0x015867c0` |
| `0x40` | `MachinedAnnounceMessage` | (named in earlier session) |

**Variable-length ID encoding in `ProcessMessage`:** component IDs `≤ 0xfe` are written as 1 byte; IDs `> 0xfe` are written as `0xff` prefix + 3 bytes. See `ProcessMessage::writeComponentsVarLen` at `ghidra://SGW.exe@0x01586180`. This is the closest analog to the (un-pinned) `InterfaceElement::compressLength_write` threshold mentioned in §1.5.

MachineGuard is mentioned here because the V5 finding doc recovered it alongside Mercury and it shares wire-format vocabulary (variable-length IDs, type-byte dispatch). It does not yet have a bible chapter; the glossary marks it `→ N/A (no chapter yet)`. Cimmeria does not need to emulate MachineGuard for client compatibility — it is internal server-machine discovery.

### 1.13 Wire-format divergences from stock BigWorld 2.0.1 — consolidated

Every SGW divergence from stock BigWorld 2.0.1 affecting Mercury wire format, in one place:

| Surface | Stock BigWorld 2.0.1 | SGW |
|---|---|---|
| Packet flags | `uint16` (2 bytes), network order | `uint8` (1 byte) |
| Footer byte order | Network (big-endian) via `BW_HTONS` / `BW_HTONL` | Little-endian |
| Encryption | Blowfish ECB + XOR chaining + `0xdeadbeef` magic + wastage byte | AES-256-CBC + HMAC-MD5 |
| Encryption KDF | (Blowfish key from session setup) | None — 32-byte SOAP `SessionKey` used verbatim as both AES and HMAC key |
| IV | (Blowfish ECB has no IV) | 16-byte zero IV, reused every packet |
| Cipher library | (BW-internal Blowfish) | CryptoPP (`HMAC<Weak1::MD5>`, `Rijndael::Enc`, `CBC_Encryption`) |
| `enableEntities` payload (§1.9) | 1 byte (`uint8 dummy`) | 8 bytes (`uint64 dummy`) |
| `createBasePlayer` typeID (§1.10.1) | `uint16` (2 bytes) | **Disputed — `uint8` or `uint16`; both produce a 6-byte payload (see Q4 in §1.15).** |
| `createCellPlayer` rotation (§1.10.2) | `roll, pitch, yaw` (`Direction3D` order) | `rotX, rotZ, rotY` (Y/Z swapped) |
| `forcedPosition` payload (§1.10.4) | 36 bytes (entityID + spaceID + vehicleID + pos + direction) | 49 bytes (adds velocity Vec3 + flags `u8`) |
| `spaceViewportInfo` size (§1.10.3) | Variable (viewport-owner / viewport-target distinct) | Fixed `CONSTANT_LENGTH = 13`; `entityId` duplicated, target unused |
| `FLAG_HAS_CHECKSUM` | Available (CRC32 in footer) | Omitted (HMAC-MD5 supersedes) |
| `FLAG_HAS_CUMULATIVE_ACK` | Available | Omitted |
| `FLAG_INDEXED_CHANNEL` | Available (indexed-channel routing) | Reserved-unused; bit 7 means `FLAG_IS_FRAGMENT` |
| Piggyback packets | Generated and consumed | Format inherited; Cimmeria Rust rejects on receive |

The divergences cluster in two themes: **security** (Blowfish → AES + HMAC) and **wire compactness** (uint16 flags → uint8, omitted flags, smaller typeID). The footer-byte-order divergence is the one most likely to silently break a reimplementation; the `enableEntities` 8-byte divergence is the most-contested historically.

### 1.14 Source-of-truth crosswalk

For section-by-section verification of every claim above:

| Claim | Primary V5 source | Secondary cross-check |
|---|---|---|
| 1-byte flags, footer little-endian | `mercury-protocol-internals.md` §"Packet Flags Byte"; `protocol-comparison.md` | Cimmeria packet serializer (matches) |
| 8-bit flag definitions | `mercury-protocol-internals.md` §"Packet Flags Byte" | Cross-check against `external/BigWorld-2.0.1/src/lib/network/packet.hpp` low byte |
| AES-256-CBC + HMAC-MD5, zero IV, no KDF | `mercury-protocol-internals.md` §"Cipher Key Derivation (Session 5 Verification)" | Agent memory `mercury-cipher-chain.md`; Cimmeria `MercuryEncryption` matches |
| Bundle/packet/message functions | `mercury-protocol-internals.md` §"4 Target Functions"; §"All Mercury Functions" | Session 5b address inventory |
| InterfaceElement length encoding (CONSTANT/WORD/DWORD) | Existing `docs/protocol/mercury-wire-format.md`; `docs/protocol/message-dispatch-table.md` | InterfaceElement table addresses (`0x0158acc0`, `0x0158b770`, `0x0158b120`) |
| Bundle fragmentation, 64-packet cap | `mercury-protocol-internals.md` §"Implications for Cimmeria"; stock BW `Packet::MaxFragmentsPerBundle` | — |
| 28-bit sequence numbers | `mercury-protocol-internals.md` §"Protocol Constants" | — |
| `enableEntities` 8-byte payload | `world-entry-pipeline.md` §"ENABLE_ENTITIES Payload Reconciliation" | `deprecated/cpp/src/baseapp/mercury/sgw/messages.cpp:83`; `protocol-comparison.md` |
| `resetEntities` 1-byte payload | `world-entry-pipeline.md` §"RESET_ENTITIES" | Initializer at `0x017bb200` |
| `createBasePlayer` 6-byte payload | `world-entry-pipeline.md` §"Phase 3 — CREATE_BASE_PLAYER" | Handler `ServerConnection_CreateBasePlayer` at `0x00dddca0` (typeID width disputed — see Q4) |
| `createCellPlayer` 32-byte payload + Y/Z swap | `world-entry-pipeline.md` §"CREATE_CELL_PLAYER" | Rotation reader `FUN_015846a0`; audit-findings table ("CONFIRMED"); legacy `client_handler.cpp` pattern |
| `spaceViewportInfo` 13-byte fixed payload | `world-entry-pipeline.md` §"SPACE_VIEWPORT_INFO" | `entity-creation-wire-formats.md` cross-reference |
| `forcedPosition` 49-byte fixed payload | `world-entry-pipeline.md` §"FORCED_POSITION" | Handler `ServerConnection_ForcedPosition` at `0x00dd9ee0`; audit-findings table ("CONSISTENT") |
| MachineGuard message types | `mercury-protocol-internals.md` §"MachineGuard Protocol" | — |
| Mercury::Nub construction | `mercury-protocol-internals.md` §"Mercury::Nub" address table | Address-map |

### 1.15 Open questions

Six unresolved questions block this chapter from promoting out of `draft`. Each has a state, a path to resolution, and a description of what stays uncertain until it lands.

#### Q1 — InterfaceElement compressed-length thresholds (§1.5)

**Question:** What are the exact byte threshold values at which `InterfaceElement::compressLength_write` switches between 1-byte, 2-byte, 3-byte, and 4-byte representations?

**State:** The three functions (`compressLength`, `expandLength`, `compressLength_write`) are V5-confirmed and addressed at `0x0158acc0`, `0x0158b770`, `0x0158b120`. Their threshold constants are not enumerated in `mercury-protocol-internals.md`. The closest analog is `ProcessMessage::writeComponentsVarLen` at `0x01586180` (single threshold at `0xfe`).

**Path to resolution:** Decompile `0x0158b120` and read the compare-and-branch constants.

**Impact if unresolved:** Section 1 cannot fully canonize system-message length encoding for `msg_id` slots that use compressed length. Entity messages are unaffected (they always use `WORD_LENGTH`). Confidence stays `medium` for §1.5.

#### Q2 — Reply-ID endianness (§1.9)

**Question:** Are reply IDs in `msg_id = 0xFF` messages written in little-endian (matching SGW's footer convention) or in network byte order (matching stock BW's `BW_HTONL`)?

**State:** Stock BW writes reply IDs in network byte order. SGW's footer is little-endian (a confirmed divergence; see §1.3.3). Whether SGW carried the little-endian convention into the reply-ID field or left it in network order is not enumerated in any V5 finding.

**Path to resolution:** Either (a) pcap capture of an actual reply-bearing packet from the live Cimmeria→client interaction, comparing the reply-ID bytes to the request's pending-reply ID; or (b) Ghidra-pass through `Mercury::Nub::handleMessage` at `0x0157bd30` and the reply-write site in `Bundle::startMessage_request` at `0x0157adc0`.

**Impact if unresolved:** A reimplementation that picks the wrong endianness produces silent reply-mismatch failures — the request-reply pairing breaks but the cipher envelope still validates, so the failure looks like "reply never arrived" rather than a protocol error.

#### Q3 — RESOURCE_FRAGMENT wire format (§1.9)

**Question:** What is the byte-level layout of a `RESOURCE_FRAGMENT` message? Specifically: length-prefix width, fragment-ID encoding, payload-size limit, and reassembly invariants.

**State:** The CME event for RESOURCE_FRAGMENT delivery is anchored at `0x004269f0` (`CME_MemberCallback_Ctor_ServerSource_NetProxyData`). The Mercury-side InterfaceElement descriptor for the RESOURCE_FRAGMENT `msg_id` slot has not been read. The Cimmeria Rust implementation uses `u16` length prefix (matching the `WORD_LENGTH` baseline), but this is an implementation choice, not evidence for the binary's behavior.

**Path to resolution:** Decompile the RESOURCE_FRAGMENT `msg_id` slot's `InterfaceElement` registration (cross-reference the static-init code that registers the slot, similar to the `enableEntities` initializer at `0x017bade0`).

**Alternative resolution:** Move RESOURCE_FRAGMENT canonization to `spec.engine.cooked-data-pipeline` (the chapter that owns the fragment-reassembly lifecycle) and have Section 1 of `spec.protocol.mercury-wire-format` reference it. The Mercury layer would only canonize "RESOURCE_FRAGMENT exists, occupies one `msg_id` slot, uses the same packet envelope as every other Mercury message" without committing to byte-level details that belong to a different chapter's scope.

**Impact if unresolved:** Section 1 cannot mark RESOURCE_FRAGMENT at `re: high`. Cimmeria implementation correctness for cooked-data streaming relies on the `u16` length-prefix guess being correct; if the binary uses a different width, the Rust implementation may silently mis-frame fragments.

#### Q4 — `createBasePlayer` typeID width (§1.10.1)

**Question:** Is the `typeID` (`classId`) field in the `createBasePlayer` wire payload `uint8` (1 byte, leaving room for a `propCount: u8 = 0`) or `uint16` (2 bytes, no `propCount`)?

**State:** `world-entry-pipeline.md` is internally inconsistent: its "Phase 3" wire-layout description shows `[classId: u8][propCount: u8]` (interpretation A), while its "Audit Findings" table and address-map describe the handler at `0x00dddca0` as "reads entityId u32 + typeId u16" (interpretation B). Both produce a 6-byte payload matching `word_len = 6`.

**Path to resolution:** Decompile `ServerConnection_CreateBasePlayer` at `0x00dddca0` directly and read the size of the `typeId` field's read operation.

**Impact if unresolved:** Section 1 cannot canonize the `createBasePlayer` divergence from stock BW. The §1.13 divergence table entry is currently marked "Disputed". Reimplementation correctness: if interpretation A is right, a server emitting `uint16` typeID will produce a packet the client mis-parses (`propCount` byte gets swallowed into the typeID's high byte). If interpretation B is right, a server emitting `uint8 typeID + uint8 propCount` will produce a packet the client reads as `typeID = (propCount << 8) | u8_typeID` — almost certainly producing a wrong class lookup.

#### Q5 — ChannelInternal `+0x170` / `+0x174` timer fields (§1.7)

**Question:** What are the roles of the timer fields at offsets `+0x170` and `+0x174` of the `ChannelInternal` struct?

**State:** `mercury-protocol-internals.md` Session 5b open question 1 flagged these as "additional timer fields whose role is TBD." Three other timer fields at `+0x160`, `+0x164`, and `+0x16c` are role-confirmed (recv timeout threshold, recv timeout last-check timestamp, send-alive timeout). Two more fields exist at adjacent offsets but their roles in `ChannelInternal::checkAndSendNubException` at `0x0158bed0` were not chased.

**Path to resolution:** Decompile `checkAndSendNubException` and follow the read sites for `+0x170` and `+0x174`.

**Impact if unresolved:** Section 1's §1.7 reliability-state subsection is canon for the three confirmed timer fields but admits "additional timer fields (role TBD)". A reimplementation that runs without these timer behaviors may diverge from observed Mercury reconnect / keepalive cadence — most likely on long-idle channels.

#### Q6 — `spaceViewportInfo` second-entityId semantics in SGW (§1.10.3)

**Question:** Does any SGW gameplay path emit a `spaceViewportInfo` packet where the two entityId fields are *different*? §1.10.3 documents the stock-BW viewport-owner-vs-viewport-target distinction and notes that every observed SGW packet sets both fields equal — but that asserts a negative.

**State:** All V5-confirmed traffic and the deprecated server's emit path agree both fields are the player's own entity ID. No SGW gameplay mode is currently known that would produce different values (spectator camera, replay viewer, GM-watch — none are in the running game). The negative cannot be fully proven without an exhaustive Ghidra-side audit of every site that calls into the `SpaceViewportInfo` emitter.

**Path to resolution:** (a) Cross-reference every call site of the server-side `SpaceViewportInfo` writer; verify each one sets both fields equal. (b) Search the deprecated server's Python code for any setter that diverges. (c) If both audits are clean, upgrade the §1.10.3 claim from "always equal in observed traffic" to "always equal across all known emit paths."

**Impact if unresolved:** A reimplementation that treats both fields as definitionally-equal will be correct for current emulation but would break if a 2009-era SGW mode that used distinct values is ever discovered and re-enabled. Low blast-radius; flagging for completeness.

#### Q7 — `forcedPosition` velocity Vec3 semantics outside world entry (§1.10.4)

**Question:** Under what conditions does the server emit a `forcedPosition` with a non-zero velocity Vec3? Does the client apply the velocity as a delta-replacement (`entity.velocity = packet.velocity`) or as an additive impulse (`entity.velocity += packet.velocity`)?

**State:** §1.10.4 documents that the velocity field is always zero in observed world-entry traffic. The wire layout reserves 12 bytes for the Vec3, so a non-zero use is intended somewhere, but the trigger and the receive-side semantic are unverified at the wire level.

**Path to resolution:** (a) Decompile `ServerConnection_ForcedPosition` at `ghidra://SGW.exe@0x00dd9ee0` and trace the velocity field through to the entity-state update. (b) Cross-check the server-side emit path for `forcedPosition` to enumerate trigger conditions (gate travel, anti-cheat snap, teleport, etc.).

**Impact if unresolved:** Position-snap behavior outside world entry is currently undocumented at the wire layer. A reimplementation that always emits zero velocity will match observed world-entry behavior but may drift from observed-in-flight-correction behavior if any was captured in pre-V5 work. Confidence on §1.10.4 stays high for world entry, medium for non-world-entry snaps.

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
