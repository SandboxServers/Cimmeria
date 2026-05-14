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

```
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

**Worked example.** A reliable, sequenced, fragmented bundle's first packet — flags byte `0xB8` (`0x80 | 0x20 | 0x10 | 0x08` = `FLAG_IS_FRAGMENT | FLAG_HAS_SEQUENCE_NUMBER | FLAG_IS_RELIABLE | FLAG_ON_CHANNEL`) — has the following on-wire shape:

```
byte 0:        0xB8                            ← flags (FLAG_IS_FRAGMENT|FLAG_HAS_SEQUENCE_NUMBER|FLAG_IS_RELIABLE|FLAG_ON_CHANNEL)
byte 1..N:     [interface element calls]       ← message body, packed end-to-end
byte N+1..N+4: firstFragmentId   (u32 LE)      ← popped third-from-end
byte N+5..N+8: lastFragmentId    (u32 LE)      ← popped second-from-end
byte N+9..N+12: sequenceId       (u32 LE)      ← popped from end
```

A non-fragmented unreliable position-update packet — flags byte `0x28` (`0x20 | 0x08` = `FLAG_HAS_SEQUENCE_NUMBER | FLAG_ON_CHANNEL`) — has only the 4-byte sequence ID in its footer. A purely unreliable broadcast — flags byte `0x00` — has no footer at all; the entire packet is `[0x00][body]`. The flags byte's bits are the contract: setting bit N obligates the sender to append a specific footer field at the end and the receiver to pop that field on parse.

**Divergence from stock BigWorld 2.0.1.** Stock BW uses a `uint16` flags field at the front of the packet (2 bytes, network-byte-order). SGW collapses this to a `uint8` (1 byte). The low byte of stock BW's `uint16` carries flags 0x01–0x80; the high byte carries 0x0100 (`FLAG_HAS_CHECKSUM`), 0x0200 (`FLAG_CREATE_CHANNEL`), and 0x0400 (`FLAG_HAS_CUMULATIVE_ACK`). SGW omits all three: no CRC32 checksum, no create-channel marker, no cumulative-ack mechanism. The full divergence inventory is in §1.4.

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

**Pop order** (from the byte just before the end of the datagram, moving toward the start):

```
┌─────────────────────────────────────────────────────────────────┐
│ if FLAG_HAS_FIRST_REQUEST_OFFSET:  firstRequestOffset (u16 LE)  │
│ if FLAG_HAS_PIGGYBACKS:            piggyback chain (see §1.3.2) │
│ if FLAG_HAS_ACKS:                  ack list (see §1.3.1)        │
│ if FLAG_HAS_SEQUENCE_NUMBER:       sequenceId (u32 LE)          │
│ if FLAG_IS_FRAGMENT:               lastFragmentId (u32 LE)      │
│ if FLAG_IS_FRAGMENT:               firstFragmentId (u32 LE)     │
└─────────────────────────────────────────────────────────────────┘
                                                ^
                                                |
                                          end of datagram
```

The pop order is **inverse of the bit order in the flags byte**. A sender writes flags first, then writes message body, then appends each footer field in flag-bit order. A receiver reads flags first, then pops each footer field in reverse flag-bit order — so the field that was *appended last* is popped *first*.

#### 1.3.1 Ack list encoding

When `FLAG_HAS_ACKS` is set, the ack list at the tail is:

```
[ ack[N-1]: u32 LE ]   ← popped second
[ ack[N-2]: u32 LE ]
...
[ ack[0]:   u32 LE ]
[ ackCount: u8     ]   ← popped first
```

Each `ack[i]` is the sequence ID of a previously received reliable packet. The receiver pops `ackCount` first (1 byte), then pops `ackCount × 4` bytes as the ack array. The sender side mirrors this in `UnAckedHandler::buildAndSendAckBundle` at `ghidra://SGW.exe@0x0158b2d0`: walks a 32-bit ack mask and writes each sequence ID into the bundle.

#### 1.3.2 Piggyback chain encoding

Piggybacks are *whole previously-sent packets* embedded in the footer area of a new outgoing packet. Format from stock BW (SGW inherits the layout; both ends of the protocol parse the same wire bytes):

```
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

```
[ AES-256-CBC ciphertext (PKCS#7-padded plaintext) ]  ← variable length
[ HMAC-MD5 tag (always 16 bytes, no truncation)    ]
```

The ciphertext is produced by `CryptoPP::StreamTransformationFilter` (`ghidra://SGW.exe@0x004089b0`) over the Mercury plaintext, then passed to `CryptoPP::HashFilter` (`ghidra://SGW.exe@0x00414720`) which appends the HMAC-MD5 tag. The HMAC covers the ciphertext, not the plaintext.

**Worked example of cipher framing.** A 21-byte Mercury plaintext (e.g. a small `enableEntities` bundle: 1 flags byte + 1 msg_id byte + 8 dummy + 4 sequence ID + 4-byte ack = 18 bytes; padded toward 21 for example purposes) becomes:

```
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

**Compressed-length encoding for interface elements with extreme size variation.** A separate scheme exists for the rare case where a message's payload size needs to fit in fewer than 2 bytes when small but extend to 3 or 4 bytes when large. The `InterfaceElement::compressLength` family handles the switch:

| Function | Address | Role |
|---|---|---|
| `InterfaceElement::compressLength` | `ghidra://SGW.exe@0x0158acc0` | Decide compressed-length width from value |
| `InterfaceElement::expandLength` | `ghidra://SGW.exe@0x0158b770` | Read compressed-length field at parse time |
| `InterfaceElement::compressLength_write` | `ghidra://SGW.exe@0x0158b120` | Write compressed-length field at emit time |

**Confidence: medium.** The exact threshold byte values at which the encoding switches from 1-byte to 2-byte to 3-byte to 4-byte are not explicitly enumerated in `mercury-protocol-internals.md` beyond "1-4 byte" — the finding doc lists the functions but not the threshold constants. The closest comparable scheme is `ProcessMessage::writeComponentsVarLen` at `ghidra://SGW.exe@0x01586180`, which is the MachineGuard component-ID encoder and uses a single threshold: IDs `≤ 0xfe` are written as 1 byte; IDs `> 0xfe` are written as `0xff` prefix + 3 bytes. The InterfaceElement compressed-length scheme is **likely** similar (with a switch around `0xff` or `0xfd`) but pin to "medium" until `0x0158b120` is decompiled and the threshold constants are extracted. **Open question for review: read `compressLength_write` and pin the exact threshold values.**

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

```
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

Entity messages: the first method's `msg_id` byte encodes the method index directly (`methodId | 0x80` for cell, `methodId | 0xC0` for base). For method indices `≥ 0x3D` (61), the encoding switches to *extended*: the message ID is `0xBD` (cell) or `0xFD` (base), and an extra `u8` carrying `methodId - 0x3D` follows the `entityId`. This is the sub-slot mechanism canonized in `spec.engine.entity-description-parse-chain` (threshold = 62 in stock BW; 61 in SGW after the parser indices are zero-indexed at the dispatch site).

**Worked example of direct vs extended entity dispatch.** A call to `onStatUpdate` (cell method index 20) on entity ID `0xCAFEBABE` with 3 bytes of arguments:

```
[0x94]                  ← msg_id = 20 | 0x80 = 0x94 (direct encoding)
[0x07 0x00]             ← word_len = 7 (u16 LE)  — 4 bytes entityId + 3 bytes args
[0xBE 0xBA 0xFE 0xCA]   ← entityId = 0xCAFEBABE (u32 LE)
[arg0 arg1 arg2]        ← serialized args
```

A call to `onClientMapLoad` (cell method index 117 — above the 61 threshold) on the same entity:

```
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

Used to stream cooked-data resources (PAK fragments) from server to client. Each fragment carries one chunk of a resource that is reassembled by the client before being passed to the cooked-data pipeline.

**Confidence: low** for the byte-level fragment layout. The address-map entry `0x004269f0` points at `CME_MemberCallback_Ctor_ServerSource_NetProxyData` for `Event_Net_ProxyData` (the CME event that delivers RESOURCE_FRAGMENT to subscribers), but the wire-format of the RESOURCE_FRAGMENT message itself is not enumerated in `mercury-protocol-internals.md`. The Cimmeria Rust implementation uses a `u16` length prefix per the `resource_fragment_uses_u16_length_prefix` regression test in `crates/services`, which matches the entity-message `WORD_LENGTH` baseline — pin this claim to medium until the binary descriptor is read. Tracked as future work in §"Open questions" below.

#### Reply messages (`msg_id = 0xFF`)

```
[0xFF: u8] [length: u32 LE] [replyId: u32 LE] [reply data]
```

The reply ID matches a pending request registered by the original sender via `Bundle::startMessage_request` at `ghidra://SGW.exe@0x0157adc0`. The next-request-offset linked-list in the request packet's footer (`FLAG_HAS_FIRST_REQUEST_OFFSET` at bit 0, `firstRequestOffset` field in the footer) lets the receiver walk all request messages in a packet without having to look at message bodies. Reply IDs in stock BW are written in network byte order; SGW endianness for the reply-ID field is **pending verification** — pin to medium for now.

### 1.10 Nub — endpoint object

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

### 1.11 MachineGuard — adjacent machine-discovery protocol

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

### 1.12 Wire-format divergences from stock BigWorld 2.0.1 — consolidated

Every SGW divergence from stock BigWorld 2.0.1 affecting Mercury wire format, in one place:

| Surface | Stock BigWorld 2.0.1 | SGW |
|---|---|---|
| Packet flags | `uint16` (2 bytes), network order | `uint8` (1 byte) |
| Footer byte order | Network (big-endian) via `BW_HTONS` / `BW_HTONL` | Little-endian |
| Encryption | Blowfish ECB + XOR chaining + `0xdeadbeef` magic + wastage byte | AES-256-CBC + HMAC-MD5 |
| Encryption KDF | (Blowfish key from session setup) | None — 32-byte SOAP `SessionKey` used verbatim as both AES and HMAC key |
| IV | (Blowfish ECB has no IV) | 16-byte zero IV, reused every packet |
| Cipher library | (BW-internal Blowfish) | CryptoPP (`HMAC<Weak1::MD5>`, `Rijndael::Enc`, `CBC_Encryption`) |
| `enableEntities` payload | 1 byte (`uint8 dummy`) | 8 bytes (`uint64 dummy`) |
| `createBasePlayer` typeID | `uint16` (2 bytes) | `uint8` (1 byte) |
| `createCellPlayer` rotation | `roll, pitch, yaw` (`Direction3D` order) | `rotX, rotZ, rotY` (Y/Z swapped) — see open Q4 |
| `forcedPosition` payload | 36 bytes (entityID + spaceID + vehicleID + pos + direction) | 49 bytes (adds velocity Vec3 + flags `u8`) |
| `FLAG_HAS_CHECKSUM` | Available (CRC32 in footer) | Omitted (HMAC-MD5 supersedes) |
| `FLAG_HAS_CUMULATIVE_ACK` | Available | Omitted |
| `FLAG_INDEXED_CHANNEL` | Available (indexed-channel routing) | Reserved-unused; bit 7 means `FLAG_IS_FRAGMENT` |
| Piggyback packets | Generated and consumed | Format inherited; Cimmeria Rust rejects on receive |

The divergences cluster in two themes: **security** (Blowfish → AES + HMAC) and **wire compactness** (uint16 flags → uint8, omitted flags, smaller typeID). The footer-byte-order divergence is the one most likely to silently break a reimplementation; the `enableEntities` 8-byte divergence is the most-contested historically.

### 1.13 Source-of-truth crosswalk

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
| MachineGuard message types | `mercury-protocol-internals.md` §"MachineGuard Protocol" | — |
| Mercury::Nub construction | `mercury-protocol-internals.md` §"Mercury::Nub" address table | Address-map line 723 |

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
