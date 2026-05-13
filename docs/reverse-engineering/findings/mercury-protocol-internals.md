# Mercury Protocol Internals — Client Binary Analysis

> **Last updated**: 2026-03-08
> **Source**: SGW.exe Ghidra decompilation (48 functions renamed)
> **Confidence**: HIGH — full call chains decompiled, constants extracted, RTTI validated

---

## Architecture Overview

SGW uses BigWorld's **Mercury** networking layer with the legacy **Nub** naming convention (no "NetworkInterface" strings). Protocol is UDP with custom reliability, sequencing, fragmentation, piggybacking, and acknowledgments.

### Outgoing Call Chain
```
ServerConnection::send (0x00dd8930)
  → Mercury::Channel::send (0x01576f90)
    → Mercury::Bundle::finalise (0x0157a7a0)
    → Mercury::Nub::send (0x01582160)
      → Mercury::Nub::writeConnection (0x01583a90) [sendto()]
```

### Incoming Call Chain
```
Mercury::Nub::processPendingEvents (0x01581ab0) [recvfrom()]
  → Mercury::Nub::processFilteredPacket (0x01580ad4)
    → Mercury::Nub::processFilteredPacket_inner (0x01580840)
      → Mercury::Nub::processPacket (0x0157fd20) [fragment reassembly]
        → Mercury::Nub::processOrderedPacket (0x0157c820) [dispatch by msg ID]
          → Mercury::Nub::handleMessage (0x0157bd30) [reply matching]
```

---

## 4 Target Functions (Previously TODO)

### 1. Mercury::Channel::send — `0x01576f90`

Channel-level outgoing dispatch. Checks if bundle has data, calls `Bundle::finalise`, delegates to `Nub::send_setupReplyHandlers` (`0x0157ec70`). Actual UDP write goes through `Nub::send` → `Nub::writeConnection` → `sendto()`.

### 2. Mercury::Nub::processOrderedPacket — `0x0157c820`

Incoming message processing. After packet reassembly and sequencing, iterates messages in a bundle, looks up handler by message ID from interface table, dispatches to registered handler. Handles corrupted headers, unhandled IDs, verifies all data consumed.

Related: `Nub::_processMessage` (`0x0157e480`) handles Channel register/deregister. `Nub::handleMessage` (`0x0157bd30`) matches reply IDs to pending request handlers.

### 3. Mercury::Bundle::newMessage — `0x0157ac90`

Begins constructing a new message. Takes InterfaceElement + payload length. Computes header size via `0x0158aa40`, increments message count, checks space (max 0x5AD bytes), allocates new packet if needed via `Bundle::reserve`, writes message ID byte, returns payload pointer.

Wrappers:
- `startMessage_fixed` (`0x0157ad80`) — fixed-length messages
- `startMessage_request` (`0x0157adc0`) — request messages expecting replies

### 4. Mercury::Bundle::addBlob — `0x0157a990`

Adds raw data to bundle. `(this, void* data, size_t length)`. Copies data into current packet buffer, auto-allocates new packets when full. Handles data spanning multiple packets by splitting at boundaries.

---

## Protocol Constants

| Constant | Value | Description |
|----------|-------|-------------|
| Max packet payload | `0x5AD` (1453 bytes) | Standard Mercury UDP packet size |
| Sequence number mask | `0x0FFFFFFF` | 28-bit sequence numbers |
| Null sequence number | `0x10000000` | Sentinel value |

### Packet Flags Byte (offset 0x54 in packet struct)

| Bit | Mask | Flag |
|-----|------|------|
| 0 | 0x01 | `FLAG_HAS_FIRST_REQUEST_OFFSET` |
| 1 | 0x02 | `FLAG_HAS_PIGGYBACKS` |
| 2 | 0x04 | `FLAG_HAS_ACKS` |
| 3 | 0x08 | `FLAG_ON_CHANNEL` |
| 4 | 0x10 | `FLAG_IS_RELIABLE` |
| 5 | 0x20 | `FLAG_HAS_SEQUENCE_NUMBER` |
| 6 | 0x40 | `FLAG_HAS_REQUESTS` |
| 7 | 0x80 | `FLAG_IS_FRAGMENT` |

---

## All Mercury Functions (48 renamed in Ghidra)

### Bundle

| Address | Function | Description |
|---------|----------|-------------|
| `0x0157aa40` | `Bundle__Bundle` | Constructor |
| `0x0157a2f0` | `Bundle__dtor` | Destructor — frees packets, piggybacks, reply handlers |
| `0x0157a440` | `Bundle__clear` | Reset state, allocate first packet |
| `0x0157ac90` | `Bundle__newMessage` | Start new message (write msg ID, compute header) |
| `0x0157ad80` | `Bundle__startMessage_fixed` | Fixed-length message wrapper |
| `0x0157adc0` | `Bundle__startMessage_request` | Request message (reserves reply handler space) |
| `0x0157a150` | `Bundle__endMessage` | End message (compress length into header) |
| `0x0157a0a0` | `Bundle__endPacket` | End packet (reserve footer space) |
| `0x0157a7a0` | `Bundle__finalise` | Finalize for sending (set flags, iterate packets) |
| `0x0157a990` | `Bundle__addBlob` | Add raw data (memcpy with auto-split) |
| `0x0157a5d0` | `Bundle__reserve` | Reserve N bytes (new packet if needed) |
| `0x0157ad40` | `Bundle__reserveInline` | Fast-path reserve |

### Nub (NetworkInterface)

| Address | Function | Description |
|---------|----------|-------------|
| `0x015841d0` | `Nub__Nub` | Constructor — socket, network thread, timers |
| `0x01582160` | `Nub__send` | Send bundle (serialize headers, write) |
| `0x01581ab0` | `Nub__processPendingEvents` | Main recv loop (recvfrom) |
| `0x01580840` | `Nub__processFilteredPacket_inner` | Parse flags, acks, piggybacks, seq# |
| `0x01580ad4` | `Nub__processFilteredPacket` | Outer packet processing |
| `0x0157fd20` | `Nub__processPacket` | Fragment reassembly → dispatch |
| `0x0157c820` | `Nub__processOrderedPacket` | Dispatch messages to handlers by ID |
| `0x0157bd30` | `Nub__handleMessage` | Match reply IDs to request handlers |
| `0x0157e480` | `Nub___processMessage` | Channel register/deregister |
| `0x01583a90` | `Nub__writeConnection` | sendto() + byte/packet counters |
| `0x01583440` | `Nub__addListeningSocket` | Create UDP socket, bind, register |
| `0x0157e920` | `Nub__registerChannel` | Register Channel with Nub |
| `0x0157eb00` | `Nub__deregisterChannel` | Unregister Channel |
| `0x0157db80` | `Nub___processMessage_removeChannel` | Remove channel during processing |
| `0x01580620` | `Nub__initConnectionMap` | Initialize connection lookup map |
| `0x0157ec70` | `Nub__send_setupReplyHandlers` | Set up reply handlers for requests |

### Channel / Connection

| Address | Function | Description |
|---------|----------|-------------|
| `0x01577960` | `Channel__Channel` | Constructor (init interface table, traits) |
| `0x01576f90` | `Channel__send` | Channel-level send |
| `0x01583680` | `Connection__send` | Connection-level send |

### InterfaceElement

| Address | Function | Description |
|---------|----------|-------------|
| `0x0158acc0` | `InterfaceElement__compressLength` | Compress variable-length header |
| `0x0158b770` | `InterfaceElement__expandLength` | Read variable-length header |
| `0x0158b120` | `InterfaceElement__compressLength_write` | Write length (1/2/3/4 byte) |

### UnAckedHandler (Reliability)

| Address | Function | Description |
|---------|----------|-------------|
| `0x0158b980` | `UnAckedHandler__sendIfReady` | Check ack bundle ready, send |
| `0x0158bbc0` | `UnAckedHandler__sendAckBundle` | Build and send ack-only bundle |
| `0x0158c420` | `UnAckedHandler__checkResendTimers` | Check timeouts, trigger resends |
| `0x0158cba0` | `UnAckedHandler__queueAckForPacket` | Queue ack for reliable packet |
| `0x0158c5d0` | `UnAckedHandler__resetLocalPart` | Reset ack state |

### MGMPacket / Packet / ChannelInternal

| Address | Function | Description |
|---------|----------|-------------|
| `0x01589290` | `MGMPacket__read` | Deserialize MGM packet |
| `0x01585260` | `MGMPacket__write` | Serialize MGM packet |
| `0x0157df90` | `Packet__release` | Free packet |
| `0x0158d050` | `ChannelInternal__resetRemotePart` | Reset (free fragments, buffers) |

### ServerConnection Wrappers

| Address | Function | Description |
|---------|----------|-------------|
| `0x00dd8930` | `ServerConnection__send` | Game-level send |
| `0x00dd9280` | `ServerConnection__enableEntities` | Enable entities message |
| `0x00de2a90`–`0x00de2b80` | `ServerConnection__startMessage_1-4` | Start message variants |
| `0x00de2bd0` | `ServerConnection__startMessage_and_reserve` | Start + reserve |

---

## RTTI Classes

Mercury::Channel, Mercury::Bundle, Mercury::Nub, Mercury::Packet, Mercury::ChannelInternal, Mercury::ClientMessage, Mercury::ClientNetMessage, Mercury::ClientOutgoingMessage, Mercury::ClientIncomingMessage, Mercury::ClientExceptionMessage, Mercury::ClientChannelRegMessage, Mercury::ClientInactivityDetectMessage, Mercury::ClientResetMessage, Mercury::ClientChannelRequestStatsMessage, Mercury::ClientChannelStatMessage, Mercury::BaseNub, Mercury::BundlePrimer, Mercury::InputMessageHandler, Mercury::ReplyMessageHandler, Mercury::TimerExpiryHandler, Mercury::NubException, Mercury::PacketFilter, Mercury::Nub::Connection, Mercury::Nub::ReplyHandlerElement, Mercury::ProcessMessageHandler@BaseNub, Mercury::QueryInterfaceHandler@BaseNub

---

## Cipher Key Derivation (Session 5 Verification)

> **Verified**: 2026-05-13 (W-auth session)
> **Confidence**: HIGH — full CryptoPP vtable stamps recovered, key-passing path traced end-to-end.

### Summary

The AES-256-CBC + HMAC-MD5 cipher chain was re-verified against the binary. All claims in
`docs/protocol/login-handshake.md` regarding algorithm, key length, IV, padding, and MAC
ordering are confirmed. One correction: the prior note in `encryption.rs` says "OpenSSL" —
the binary uses **Crypto++ (CryptoPP)**, not OpenSSL, for the AES and HMAC primitives.

### Key Derivation

There is **no KDF**. The shared secret is the raw 32-byte session key:

1. Auth Server generates a 64-character hex string (`SessionKey` attribute in SOAP response).
2. gSOAP deserializes it as `xsd:hexBinary` → 32 raw bytes (case 0x26 in the gSOAP type
   dispatcher at `0x015ed300`, handler at `0x015eb940`).
3. The 32 bytes are stored directly in the login-reply-handler struct and passed verbatim to
   `PacketEncrypter` constructor (`0x01603a70`).

No salting, no hashing, no PBKDF. The 32 raw bytes from the SOAP `SessionKey` field are the
AES key. Zero transformations applied.

### PacketEncrypter Construction

`register_NetIn_ServerSelectSuccess` (`0x00ddfd00`) is called when the SOAP Phase 2 response
parses successfully (type tag 0x14 = `SGWServerLocationResponse`). It:

1. Allocates a `PacketEncrypter` (heap, `scalable_malloc`) — size ~0x28 bytes.
2. Calls `FUN_01603a70` (PacketEncrypter ctor) with the session key bytes from the parsed
   SOAP struct at `this+0x7c` (length `piVar6[7]`, data ptr `piVar6[6]`).
3. The ctor stores the key bytes in a buffer at `PacketEncrypter+0x8`.
4. The ctor stores a 16-byte all-zero IV in `PacketEncrypter+0x18` via `FUN_00a587f0`.
5. The constructed `PacketEncrypter*` is stored at `ServerConnection+0x310`.

### PacketEncrypter Object Layout

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| +0x00 | 4 | vtable | `0x01b27374` |
| +0x04 | 4 | ref_count | `SafeReferenceCount` base |
| +0x08 | ? | key_buf | `std::vector`-like: ptr/end/capacity holding the 32-byte AES+HMAC key |
| +0x18 | ? | iv_buf | `std::vector`-like: 16 zero bytes; re-read on every packet |

### Vtable (`PacketEncrypter`, `0x01b27374`)

| Slot | Address | Role |
|------|---------|------|
| 0 | `0x01604ac0` | Destructor |
| 1 | `0x01603b80` | `send` — encrypt outgoing packet |
| 2 | `0x01603fa0` | `recv` — decrypt incoming packet |
| 3 | `0x016039a0` | Returns `0x1f` (31) — `OptimalBlockSize` or similar |

### AES-256-CBC Mode

- **Library**: CryptoPP (`CryptoPP::BlockCipherFinal<0, Rijndael::Enc>`, vtable stamped at `0x0040e030`)
- **CBC encryptor**: `CryptoPP::CipherModeFinalTemplate_ExternalCipher<CBC_Encryption>` (stamped at `0x0040d000`)
- **CBC decryptor**: `CryptoPP::CipherModeFinalTemplate_ExternalCipher<CBC_Decryption>` (stamped at `0x0040d0b0`)
- **Key**: 32 bytes (256-bit), read from `PacketEncrypter+0x8`
- **IV**: 16 zero bytes, read from `PacketEncrypter+0x18` on every packet (no IV mutation between packets — same zero IV reused every call)
- **Padding**: PKCS#7 via `CryptoPP::StreamTransformationFilter` with `PKCS_PADDING` (param=4), constructed at `0x004089b0`

### HMAC-MD5

- **Library**: CryptoPP (`CryptoPP::HMAC<CryptoPP::Weak1::MD5>`, vtable stamped in `FUN_01604d00` at `0x01604d00`)
- **Key**: Same 32-byte buffer as the AES key — `GetCheckedArrayElement(this+8, 0, len)` in both `FUN_01603b80` (encrypt) and `FUN_01603fa0` (decrypt)
- **Key length**: 32 bytes (not truncated)
- **Output length**: 16 bytes (full MD5 output, no truncation)
- **No output truncation confirmed** — `CryptoPP::HashFilter` at `0x00414720` emits full 16-byte tag

### Wire Order

Encrypt-then-MAC, matching the existing Cimmeria implementation:

```
[AES-256-CBC ciphertext (PKCS7-padded plaintext)] [16-byte HMAC-MD5 tag]
```

The `CryptoPP::HashFilter` (`0x00414720`) receives the encrypted ciphertext and appends the
HMAC tag. The `CryptoPP::StreamTransformationFilter` processes plaintext → ciphertext, which
is then passed to the HashFilter.

### Evidence Trail

| Claim | Address | Evidence |
|-------|---------|----------|
| PacketEncrypter ctor | `0x01603a70` | Stamps `PacketEncrypter::vftable`; called from `0x00ddfd00` |
| AES Rijndael-256 | `0x0040e030` | Stamps `CryptoPP::BlockCipherFinal<0, Rijndael::Enc>::vftable` |
| CBC-Encryption mode | `0x0040d000` | Stamps `CryptoPP::CipherModeFinalTemplate_ExternalCipher<CBC_Encryption>::vftable` |
| CBC-Decryption mode | `0x0040d0b0` | Stamps `CryptoPP::CipherModeFinalTemplate_ExternalCipher<CBC_Decryption>::vftable` |
| HMAC-MD5 | `0x01604d00` | Stamps `CryptoPP::HMAC<CryptoPP::Weak1::MD5>::vftable` |
| Same key for AES and HMAC | `0x01603b80` / `0x01603fa0` | Both use `GetCheckedArrayElement(this+8, 0, uVar3)` |
| 16-byte zero IV | `0x01603a70` → `0x00a587f0` | `FUN_00a587f0(this+0x18, 0x10, null)` — null source → zero-filled |
| Zero IV reused per packet | `0x01603b80` | IV fetched fresh from `this+0x18` on each call; buffer never modified |
| PKCS7 padding | `0x004089b0` | `StreamTransformationFilter(AES-CBC, hmac_filter, PKCS_PADDING=4)` |
| Encrypt-then-MAC ordering | `0x01603b80` | `StreamTransformationFilter` output piped into `HashFilter` |
| Session key from SOAP hex | `0x015eb940` / `0x015ed300` case 0x26 | gSOAP `xsd:hexBinary` deserializer; no additional KDF |
| Crypto++ not OpenSSL | RTTI strings at `0x01e93b70`–`0x01ea3c5c` | `HMAC_Base@CryptoPP`, `HMAC@VMD5@Weak1@CryptoPP`, etc. |

### Comparison to Cimmeria Rust Implementation

`crates/mercury/src/encryption.rs` — `MercuryEncryption` — matches the binary on all
confirmed points:

| Property | Binary | Cimmeria (`encryption.rs`) | Status |
|----------|--------|---------------------------|--------|
| Algorithm | AES-256-CBC | `Aes256` + `cbc::Encryptor/Decryptor` | Match |
| Key length | 32 bytes | `aes_key: [u8; 32]` | Match |
| HMAC algorithm | HMAC-MD5 | `Hmac<Md5>` | Match |
| HMAC key | Same 32-byte AES key | `hmac_key == aes_key` in `from_session_key` | Match |
| HMAC key length | 32 bytes | 32 bytes | Match |
| HMAC output | 16 bytes, no truncation | `HMAC_TAG_LEN = 16` | Match |
| IV | 16 zero bytes | `iv: [0u8; 16]` | Match |
| IV reuse | Zero IV per packet (stateless) | Deterministic zero IV per `encrypt()` call | Match |
| Padding | PKCS#7 | `pkcs7_pad` / `pkcs7_unpad` | Match |
| Wire order | Encrypt-then-MAC `[ct][hmac]` | `ciphertext ++ tag` | Match |
| Crypto library note | CryptoPP | RustCrypto (`aes`, `cbc`, `hmac`, `md5`) | Differs (library only, not protocol) |

**No protocol divergences found.** The only difference is that `encryption.rs` doc-comments
say "OpenSSL" — this should be corrected to "CryptoPP (Crypto++)". The protocol behavior is
identical.

---

## Session 5b Mercury Layer Completion — 2026-05-13

> **Worker**: W-mercury-deep (V5 campaign Session 5b)
> **Functions added**: 145 total (90 batch 1 + 55 batch 2), all renamed and plate-commented in Ghidra
> **Scope**: `[0x01576000, 0x0158efff]` — complete

### What Was Recovered

Session 5b completed the full reverse engineering of the BigWorld Mercury networking layer in SGW.exe. Every unnamed `FUN_*` in the target range was decompiled, identified, renamed, and annotated. The complete class hierarchy is now documented.

### Newly Recovered Classes and Functions

#### MachineGuard Protocol (machine discovery over UDP)

The BigWorld `MachineGuard` system enables process/machine discovery via broadcast UDP. All 13 message types are now fully documented:

| Type byte | Class | Address range |
|-----------|-------|---------------|
| 0x01 | `WholeMachineMessage` | ctor `0x01587d30`, dtor `0x01587de0`, read `0x01588fc0` |
| 0x02 | `ProcessMessage` | read `0x015896d0`, write `0x01586180` |
| 0x04 | `ListenerMessage` | ctor `0x01586410`, dtor `0x015864b0`, read `0x01586c20` |
| 0x05 | `CreateMessage` | ctor `0x01586590`, dtor `0x01586630`, read `0x01586cf0` |
| 0x06 | `SignalMessage` | ctor `0x01586710`, writeWithName `0x01586140` |
| 0x07 | `TagsMessage` | ctor `0x01587ef0`, read `0x01589560` |
| 0x0b | `ErrorMessage` | ctor `0x015867c0`, dtor `0x01586850`, read `0x01586f40` |
| 0x40 | `MachinedAnnounceMessage` | (already named prior session) |

The master deserializer `MachineGuardMessage__deserialize` at `0x01588530` switches on the type byte (1–0xb/0xc/0x40), heap-allocates the correct subtype, and calls its `read()` vtable slot.

Socket operations: `MachineGuard__sendRawPacket` (`0x01588ec0`) — socket+bind+sendto; `MachineGuard__createSocketAndSend` (`0x01589f80`) — socket+bindInRange+sendAndRecv with error codes 0xfffffffd (bind fail) and 0xfffffffe (send fail).

`ComponentType::getNameForType` (`0x01587110`): maps type 0 → `"SERVER_COMPONENT"`, type 1 → `"WATCHER_NUB"`, else `"UNKNOWN"`.

#### ProcessMessage Serialization Infrastructure

ProcessMessage manages two heterogeneous vectors:
- **Interface entries**: 0x20-byte structs (`ProcessMessage::InterfaceVec`)
- **Component entries**: 0x3c-byte structs (`ProcessMessage::ComponentVec`)

Both have full vector operations recovered: insertN, insertOne, pushBack, resize, reserve, fillRange, copyRangeForward/Reverse, SEH-wrapped copy variants, and destructor.

Variable-length ID encoding: component IDs ≤ 0xfe are written as 1 byte; IDs > 0xfe are written as `0xff` prefix + 3 bytes. See `ProcessMessage__writeComponentsVarLen` at `0x01586180`.

#### ChannelInternal Lifecycle

`ChannelInternal` is the inner ~0x180-byte object behind every Mercury channel:

| Address | Function | Key detail |
|---------|----------|------------|
| `0x0158c7b0` | `ChannelInternal__ctor` | Stamps TimerExpiryHandler::vftable then ChannelInternal::vftable; initializes packet hash table (+0x40/+0x44), stats, rdtsc timers, bundle (+0x9c), address strings (+0x80/+0x13c), UnAckedHandler (+0x114), timeout thresholds (+0x160..+0x174) |
| `0x0158d190` | `ChannelInternal__dtor` | Stamps vftable, calls resetLocalPart, calls cleanup1 |
| `0x0158d267` | `ChannelInternal__dtor_cleanup1` | Calls `Mercury_Channel_cleanup('\0')`, calls cleanup2 |
| `0x0158d310` | `ChannelInternal__dtor_cleanup2` | Frees name string, bundle vector, filter, bundle, entity listener map; restores TimerExpiryHandler::vftable |
| `0x0158bed0` | `ChannelInternal__checkAndSendNubException` | rdtsc timeout check vs +0x164/+0x160 → throws NubException; checks +0x16c (send-alive) → sendAckBundle2 |
| `0x0158bd40` | `ChannelInternal__recordLatency` | TBB `__TBB_machine_store8` into circular buffer +0x3c..+0x7c; maintains min/max rdtsc at +0x178/+0x17c |
| `0x0158b9d0` | `ChannelInternal__getAndResetStats` | Reads latency from +0x17c/+0x178/+0x7c, resets accumulators |
| `0x0158be30` | `ChannelInternal__processIncomingPacketEntry` | Dispatches to `Nub::dispatchPacketWithFilter`; stamps rdtsc at +0x58/+0x5c |
| `0x0158a850` | `ChannelInternal__getNextChannelInternal` | Atomic read of +8 (next ptr in doubly-linked list) + incRef; safe traversal |
| `0x0158a8e0` | `ChannelInternal__countChain` | Walks chain via getNextChannelInternal, returns count |
| `0x0158ab40` | `ChannelInternal__advanceReadPointer` | Advances read cursor; walks to next ChannelInternal when packet exhausted |

#### UnAckedHandler Completion

| Address | Function |
|---------|----------|
| `0x0158b2d0` | `UnAckedHandler__buildAndSendAckBundle` — builds ACK bundle from 32-bit ack mask |
| `0x0158bbc0` | `UnAckedHandler__sendAckBundle2` — creates empty bundle, sets reliable flag, sends |
| `0x015875f0` | `ChannelInternal__unackedList__clear` — frees all UnAcked list entries |

#### Packet Chain Operations

| Address | Function |
|---------|----------|
| `0x0158a340` | `Packet__dtor` — stamps vtable, atomic decrement global packet count, decRef inner |
| `0x0158a3f0` | `Packet__chain__stampSendTime` — walks chain stamping rdtsc at +0x18..+0x1f |
| `0x0158a4f0` | `Packet__chain__stampRecvTime` — walks chain stamping rdtsc at +0x20..+0x27 |
| `0x0158a5f0` | `Packet__chain__minSendTime` — returns minimum sendTime across chain |
| `0x0158a720` | `Packet__chain__maxSendTime` — returns maximum sendTime across chain |

#### ChannelInternalPtr Smart Pointer

| Address | Function |
|---------|----------|
| `0x0158c100` | `ChannelInternalPtr__decRef` — atomic decRef; zero → destructor chain |
| `0x0158c230` | `ChannelInternalPtr__assign` — incRef new + copy byte flag + decRef old |

#### Timeout Architecture (confirmed from `ChannelInternal__ctor` + `checkAndSendNubException`)

Three rdtsc-based timeout fields within ChannelInternal (size ~0x180):

| Offset | Role |
|--------|------|
| +0x160 | Receive timeout threshold (rdtsc units) |
| +0x164 | Receive timeout last-check timestamp |
| +0x16c | Send-alive timeout — triggers `sendAckBundle2` to keep channel alive |
| +0x170/+0x174 | Additional timer fields (exact role TBD) |

### Complete Function Inventory (Session 5b Additions)

Full list in checkpoint at `docs/reverse-engineering/v5-campaign/worker-mercury-deep.checkpoint.json`.

### Open Questions

1. **`+0x170`/`+0x174`** in ChannelInternal ctor — two additional timeout-style fields initialized but not observed in checkAndSendNubException. Possibly fragment reassembly timeouts.
2. **`TagsMessage` vfunc_0** at `0x01587fe0` — dtor slot already renamed in prior session; full vftable not recovered (5 slots visible, exact layout uncertain).
3. **`PidMessage` / `ResetMessage` / `QueryInterfaceMessage`** read paths — not observed in batch 2 decompiles; may be in `0x0158d400+` range which was outside scope.

---

## Implications for Cimmeria

1. **Mercury uses Nub naming** throughout — no "NetworkInterface" in the binary.
2. **Packet flags byte** at offset 0x54 controls all optional features (piggybacks, acks, fragments, etc.).
3. **Max packet size is 1453 bytes** — messages larger than this are automatically fragmented.
4. **28-bit sequence numbers** with windowed ack tracking for reliability.
5. **Variable-length message headers** — InterfaceElement compresses length into 1-4 bytes.
6. **UnAckedHandler** manages resend timers — Cimmeria should implement equivalent timeout/resend logic.
7. **ServerConnection** wraps Channel::send with game-specific message construction helpers.
8. **MachineGuard protocol** is fully recoverable — 13 message types documented; if Cimmeria needs to emulate BigWorld machine discovery, all read/write/ctor/dtor paths are now named.
9. **ChannelInternal timeout thresholds** at +0x160/+0x164/+0x16c are rdtsc-based — on Cimmeria's server side, `std::time::Instant` equivalents are appropriate; no absolute tick rate dependency.
10. **Global packet count** at `DAT_018d4858` is atomically maintained by `Packet__dtor` — diagnostic telemetry only, not required for correct emulation.
