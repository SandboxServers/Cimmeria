---
name: mercury-section-2-live-capture-findings
description: Live-process memory dumps from active SGW.exe session via x64dbg MCP — closes Gap A (canonical msg_id table) and Gap B (SHA-1 protocol_digest) and resolves the static/dynamic InterfaceElementVec architecture. Counterpart to [[mercury-section-2-track-b-evidence]] (static-RE side).
metadata:
  type: project
---

# Mercury §2 — Live-Capture Findings (2026-05-15)

**Method**: x64dbg MCP attached to live `SGW.exe` (PID 35788) at character-select after successful login. Silent-BP halt-and-resume captured `ServerConnection.this` at PopulateMessageTypeTable entry; subsequent `read_memory` calls extracted dispatch tables, statistics, and the protocol_digest string from live heap. **All reads succeeded while debuggee was Running** (the x64dbg-automate MCP does not require pause for memory read).

**ASLR offset**: `+0x80000` (runtime base `0x00480000` vs Ghidra static base `0x00400000`).

---

## Gap B (re-opened then RESOLVED at higher resolution) — TWO digests exist

### CORRECTION 2026-05-15: The earlier "SHA-1 protocol_digest" finding was wrong by being incomplete.

Cross-referencing the AteraLoader session log with the disassembly revealed **two distinct hashes**, only one of which is `protocol_digest`:

### Digest #1 — `protocol_digest` (wire) is **MD5, 32 hex chars**

**Source**: CME `Event_Net_GetProtocolDigest` event listener (RTTI at `0x01df15dc`). The C++ code in `SGWNetworkManager::<caller>` (function `FUN_00c6e3a0`, line `.\Src\SGWNetworkManager.cpp:0x21f`) raises this event; a listener (likely Python game-script or another C++ subsystem) populates the digest; SGWNetworkManager then passes it as `param_4` to `logOnBegin`.

Evidence (from `sessions/2026-05-15_12-43.log`):

```
ServerConnection::logOnBegin: server:http://47.45.19.7:8081 username:test
protocol_digest: 58AFA196AD3AC4F65CADD99BFF23B799
```

32 hex chars = 128 bits = **MD5**.

**Wire path**: this string is plugged into the SOAP body's `sgwLogin:ProtocolDigest` field (Track B finding 4.7 — multiple xrefs at `0x01b2507c`, `0x01b25384`, `0x01b25ad8`).

**Server-side implication for Cimmeria**: The server must produce/accept the same MD5 over the same entity-defs that the client's CME listener computes. The hash algorithm is MD5, not SHA-1.

### Digest #2 — Internal dispatch-table hash is **SHA-1, 40 hex chars**, stored at `ServerConnection+0x130`

**Source**: Computed INSIDE `logOnBegin` by a CryptoPP HexEncoder pipeline that runs **immediately after** `PopulateMessageTypeTable`. The encoder produces 40-char uppercase hex output. This is then stored at `ServerConnection+0x130` via `std::basic_string::operator=`.

Captured value (from session 1 PMT-BP read): `A94A8FE5CCB19BA61C4C0873D391E987982FBBD3` (40 chars uppercase = 160 bits = **SHA-1**).

**MSVC VS2005 basic_string layout at +0x130 confirmed**: `_Myproxy=0` at +0x130, `_Bx._Ptr=0x09869568` at +0x134 (heap form), `_Mysize=40 (0x28)` at +0x144, `_Myres=47 (0x2F)` at +0x148.

**Pipeline**: `local_ac` empty basic_string → CryptoPP HexEncoder with `Uppercase=true` (per `ConstructHexEncoder` at `0x00de41a0`) → sink writes to `local_ac` → `operator=((this+0x130), local_ac)`.

**The input to the pipeline** is some derivation of the post-PMT dispatch table state — possibly the table bytes themselves, or a hash of the entity-defs that the table was built from. Confirming this requires deeper analysis of the CryptoPP source/attached stages (`local_148`, `local_198`, `local_f4` buffers in `logOnBegin`).

**Wire path**: NONE confirmed. This hash is stored only at the local `ServerConnection+0x130` field. It is NOT sent on the wire. Likely used for runtime verification (e.g., checking the dispatch table hasn't been tampered with, or as a debug/log breadcrumb).

### Why the confusion

Both values are uppercase hex strings produced by CryptoPP HexEncoder. Both are "hashes of entity-def-related state." Without the AteraLoader log showing the actual wire-side digest, the +0x130 SHA-1 looked like the protocol_digest. **The Atrea log is authoritative because the client itself labels the value as `protocol_digest:`**.

### Architectural summary diagram

```
┌────────────────────────────────────────────────────────────────────────────┐
│ protocol_digest (wire-side commitment to entity-defs)                       │
│   Algorithm: MD5 (32 hex chars, uppercase)                                 │
│   Computed by: CME Event_Net_GetProtocolDigest listener (external/scripted)│
│   Stored where: passed as param to logOnBegin, then SOAP body              │
│   Sent on: HTTP POST to authentication endpoint                            │
└────────────────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────────────────┐
│ dispatch_table_hash (internal commitment to post-PMT state)                │
│   Algorithm: SHA-1 (40 hex chars, uppercase, via CryptoPP HexEncoder)      │
│   Computed by: C++ inside logOnBegin (CryptoPP pipeline, post-PMT)         │
│   Stored where: ServerConnection+0x130 (basic_string)                      │
│   Sent on: NOTHING — internal only                                         │
└────────────────────────────────────────────────────────────────────────────┘
```

---

## Atrea session-log findings (the WIRE TRACE we couldn't get from BPs)

The `sessions/<date>.log` file from `AtreaGameDebug.bat` runs is **a Mercury function-call trace with parameter values**. Far more useful than raw pcap bytes for protocol documentation, because the client labels what each thing is.

**Tick rate (NEW)**: `Nub::Nub() using thread sleep time of 10` — 10ms tick interval = **100 Hz tick rate**. This is the inner loop frequency of `Nub::tick` / `ProcessPendingEvents`.

**Endpoints observed (NEW)**:
- Auth/SOAP: `http://47.45.19.7:8081`
- BaseApp Mercury: `47.45.19.7:32832` (UDP)
- Client outbound: `172.26.240.1:55293` (bound to all interfaces, advertising eth0)

**Mercury connection lifecycle (NEW — 8-step sequence)**:

```
1. BaseNub::recreateListeningSocket: bound to all interfaces, advertising eth0
2. Nub::addListeningSocket: created connection <local:port>
3. Nub::Nub() using thread sleep time of 10
4. Nub::registerChannel: registering channel from address <remote:port>
5. BaseAppLoginHandler::BaseAppLoginHandler: calling Nub::send
6. Nub::_processMessage: registering ChannelInternal from address <remote:port>
7. Nub::send "without channel" → initial pre-channel handshake message
8. Nub::addChannelToConnection: 1 channels are using it
   ⇓
9. ServerConnection::logOn: status==LOGGED_ON
10. ServerConnection::enableEntities (i32 parameter — sometimes negative, looks like a session-token cast)
11. EntityManager::connected
12. ServerConnection::createBasePlayer: id 1 (initial entity)
13. ServerConnection::createBasePlayer: id 2 (the actual player entity)
14. ServerConnection::spaceViewportInfo: space <spaceID> svid <viewportID>
15. ServerConnection::createCellPlayer: id 2
   ⇓
[normal gameplay; out-of-order packets buffered by UnAckedHandler]
   ⇓
N. EntityManager::loggedOff
N+1. ServerConnection::loggedOff: The server has disconnected us. reason = <code>
N+2. Nub::deregisterChannel: deregistering channel from address <remote:port>
N+3. Nub::_processMessage: removing channel ... 0 channels using it
N+4. Nub::_processMessage: deregistering ChannelInternal from address <remote:port>
N+5. EntityManager::disconnected
```

**R12 out-of-order behavior live-confirmed**:

```
UnAckedHandler::queueAckForPacket( 47.45.19.7:32832 ): Buffering packet #24 above #21
UnAckedHandler::queueAckForPacket( 47.45.19.7:32832 ): Buffering packet #25 above #23
UnAckedHandler::queueAckForPacket( 47.45.19.7:32832 ): Buffering packet #481 above #480
UnAckedHandler::queueAckForPacket( 47.45.19.7:32832 ): Buffering packet #797 above #796
UnAckedHandler::queueAckForPacket( 47.45.19.7:32832 ): Buffering packet #1125 above #1124
UnAckedHandler::queueAckForPacket( 47.45.19.7:32832 ): Buffering packet #1523 above #1522
UnAckedHandler::queueAckForPacket( 47.45.19.7:32832 ): Buffering packet #2083 above #2082
UnAckedHandler::queueAckForPacket( 47.45.19.7:32832 ): Buffering packet #4529 above #4528
```

Confirms the sliding-window reorder behavior from Track B finding 1.2. Buffer offsets observed in this session: typically 1–3 positions ahead of expected (`#24` above `#21` = 3 ahead, `#1523` above `#1522` = 1 ahead).

**Disconnect reason code**:

```
ServerConnection::loggedOff: The server has disconnected us. reason = 0
```

`reason=0` = normal/clean disconnect (logoff initiated by user). Non-zero reasons map to the `LoginMessage_*` enum (Track B finding 4.6).

---

## AteraLoader operational notes (for future capture sessions)

- **Patch failure mode**: `Image has ASLR enabled; some patches may fail to apply or crash the game` — the byte-patches in `AtreaLoader.config.xml` (8 of them) FAIL to apply because runtime addresses don't match the hardcoded ones in the config. Result: `0 patch(es) of 8 applied`.
- **What still works after patch failure**: **Symbol patches** (6 of them) — these use function-address hooks rather than absolute byte addresses, so they survive ASLR. `MercuryLogger` at `0x0041C2E0`, `UnicodeLoggerStart` at `0x00866860`, etc. are functional via this path. **That's why the session log STILL has rich Mercury content even though byte patches failed.**
- **What DOESN'T work**: The packet sniffer (`<NVP Name="Sniffer" Value="true" />`) is likely implemented as a byte patch (no matching symbol patch entry in `AtreaLoader.config.xml`). **Result: no `.pcap` file is generated** in `sessions/` despite the config asking for one.
- **Workaround for pcap**: Wireshark on the host capturing UDP traffic on the BaseApp port. AES keys aren't captured this way, but the session log shows enough flow info that wire-decryption may not be strictly necessary for Mercury documentation.

## Caller of `logOnBegin`

`FUN_00c6e3a0` is the caller (per xref `0x00c6e573 in FUN_00c6e3a0 [UNCONDITIONAL_CALL]`). The function:

1. Gets the EntityManager
2. Calls `GameEntityManager_HandleDisconnect(em)` (cleans prior state)
3. Raises CME `Event_Net_GetProtocolDigest` event via `FUN_00a372f0(eventSignal, 0, &local_250[isHandled], NoSubject_RTTI, Event_Net_GetProtocolDigest_RTTI)`
4. **Assertion**: `if (local_250[isHandled] == 0) { FAIL ".\Src\SGWNetworkManager.cpp:0x21f digestEvent.isHandled"; }` — fail-fast if no listener responded
5. Pulls `CACertPath` from `g_pGEngine` (engine setting)
6. Builds `wchar_t*` strings for the 4 string params via `FUN_00423f40` (probably a wstring-to-utf8 converter or similar)
7. Calls `logOnBegin(this, &local_258, server_url, username, protocol_digest, cacertpath)`

**Refined `logOnBegin` parameter map** (corrects earlier guesswork):

```c
ServerConnection::logOnBegin(
    this,            // ServerConnection*
    int* out_result, // points to a place to store the constructed handler ptr (returned by FUN_00ddf580)
    char* server_url,        // param_2 = pUVar8[0x21] = http://host:port
    char* username,          // param_3 = pUVar7[0x21]
    char* protocol_digest,   // param_4 = pUVar6[0x21]  ← THE MD5 DIGEST
    char* ca_cert_path       // param_5 = pUVar5[0x21]  ← TLS CA cert path
)
```

**Inside `logOnBegin`, the four `operator=` stores correspond to**:

```c
this+0x0F8 = server_url       // (param_2)
this+0x114 = username         // (param_3)
this+0x130 = local_ac         // ← THE SHA-1 DISPATCH TABLE HASH (NOT the digest)
this+0x14C = ca_cert_path     // (param_5)
```

Note: `protocol_digest` (param_4) is NOT stored at `this+0x130`. It's used only to format the log message and to populate the SOAP body. After `logOnBegin` returns, the digest value is essentially gone from C++ memory (still pinned by the SOAP curl session being constructed via `SetupSGWLoginRequestCurlSession`).

---

## Gap A also CLOSED for CLIENT→SERVER direction (BaseAppExtInterface)

Found and extracted from SGW.exe static at `0x017bac00` (the static initializer that builds the table at process start). The InterfaceElementVec global is at `0x01EF24CC` (matches BSS layout — uninitialized in static binary, populated at runtime). The name string table is at `0x019D0880`.

**14 entries (msg_ids 0x00..0x0D)**, plus 0x80..0xFE dynamic EntityMethod slots:

| msg_id | Name | Wire size hint (from `add()` immediate) |
|---|---|---|
| 0x00 | `baseAppLogin` | 1 (word-prefix; pre-channel handshake) |
| 0x01 | `authenticate` | 2 (word-prefix; auth token) |
| 0x02 | `avatarUpdateImplicit` | 36 (constant) |
| 0x03 | `avatarUpdateExplicit` | 40 (constant) |
| 0x04 | `avatarUpdateWardImplicit` | (likely 36) |
| 0x05 | `avatarUpdateWardExplicit` | (likely 40) |
| 0x06 | `switchInterface` | 0 (parameterless trigger) |
| 0x07 | `requestEntityUpdate` | (word-prefix) |
| 0x08 | `enableEntities` | 8 (entity_id i32 + flags i32) |
| 0x09 | `setSpaceViewportAck` | 8 |
| 0x0A | `setVehicleAck` | 8 |
| 0x0B | `restoreClientAck` | (word-prefix) |
| 0x0C | `disconnectClient` | 1 (reason byte) |
| 0x0D | `entityMessage` | (word-prefix, entity-method dispatch envelope) |

### Live pcap-validated client→server traffic patterns

From the decrypted pcap (`sessions/2026-05-15_14-05.pcap` decrypted with `2026-05-15_14-04-keys.txt`):

1. **Per-tick gameplay packet** = `[0x01] authenticate (21B)` + `[0x03] avatarUpdateExplicit (40B)` — body=65B. Sent at ~70ms intervals (matches per-tick rate at 100Hz with client-side rate-limiting).
2. **Initial handshake** = `[0x00] baseAppLogin` (unencrypted, with 20-char session token in payload) → server replies with crypto material
3. **Version probe burst** = client sends ~22 × `[0xC0] versionInfoRequest (8B)` in a single packet at session start
4. **Entity creation** = `[0x07] requestEntityUpdate` after viewport-info → server replies with entity create messages

## Pcap decryption tool — `tools/pcap_dissect.py`

Patched on 2026-05-15 to:

1. **Use `cryptography` Python library** instead of `openssl` subprocess per packet (1000× faster — 23,591 lines decoded in 2.0s vs hanging-forever previously)
2. **`SERVER_MSG_NAMES`** rewritten with all 57 entries from `0x01F72518` capture (was 18 entries with wrong names for 0x0A/0x0B/0x0C)
3. **`SERVER_MSG_FORMAT`** rewritten with all 32 avatarUpdate variant sizes (10..25 bytes per variant) — fixes word-length fallback that mis-framed every gameplay packet
4. **`CLIENT_MSG_NAMES`** rewritten with all 14 entries from `0x01EF24CC` / `0x019D0880` capture (was 10 entries with uppercase-style names; missing 0x02, 0x04, 0x05, 0x0D)
5. **`CLIENT_MSG_FORMAT`** updated with binary-derived sizes for the new entries

Validated against the live pcap: every msg_0x?? label now matches the binary's authoritative names, and avatarUpdate body sizes parse correctly without desync.

## Updated new-symbol candidates for Ghidra

| Name | Static addr | Runtime addr | Source/Notes |
|---|---|---|---|
| `BaseAppExtInterface` global object | `0x01EF24CC` | `0x01EF24CC` (after ASLR fix) | Mirror of ClientInterface at `0x01F72518` |
| `BaseAppExtInterface::<staticInit>` | `0x017bac00` | `0x017bac00` | Static initializer that calls add() for 14 entries |
| BaseAppExt name string table | `0x019D0880` | `0x019D0880` | 14 client-side method names |
| `BaseAppLoginHandler` class | `0x01e533f8` (RTTI) | `0x01e533f8` | C++ class for the client login flow |
| `LoginReplyHandler::onBaseAppReply` log string | `0x019cf038` | `0x019cf038` | Confirms reply timeout reporting |
| `SGWNetworkManager::<beginLogin>` (caller of logOnBegin) | `0x00c6e3a0` | `0x00CEE3A0` | Xref of logOnBegin; assertion `"SGWNetworkManager.cpp:0x21f"` |
| `Event_Net_GetProtocolDigest` RTTI handler (CME bus side) | `0x01df15dc` | `0x01E715DC` | RTTI ref, confirmed Track B |
| `CME::EventSignal::NoSubject::RTTI_Type_Descriptor` | — | — | Used as event-bus subject template |
| `FUN_00a372f0` | `0x00a372f0` | `0x00AB72F0` | CME EventSignal raise helper — needs renaming |
| `FUN_00ddf580` (logOnBegin — confirmed param map) | `0x00ddf580` | `0x00E5F580` | Signature corrected (5 params) |
| `ConstructHexEncoder` (CryptoPP HexEncoder constructor) | `0x00de41a0` | `0x00E641A0` | Confirmed: builds pipeline with Uppercase=true |
| `dispatch_table_hash` field at `ServerConnection+0x130` | — | — | NEW: was misidentified as protocol_digest |
| `protocol_digest` parameter to logOnBegin (param_4) | — | — | NEW: MD5, comes from CME event listener |

---

## Gap A CLOSED — Canonical Wire-Format msg_id Table

### Static ClientInterface (msg_id 0x00..0x38, 57 entries)

Source: `BWNetDriver::ClientInterface` registered at compile time, addressed by global `0x01F72518` (`InterfaceElementVec`). Entry stride: `0x1C` (28 bytes) in static form. Data buffer at `0xFFC88100` (kernel-shared user region). Vec capacity = 256 entries.

**Entry layout (static, 0x1C stride)**:

```c
struct InterfaceElement {       // 0x1C bytes
    uint8_t  msg_id;            // +0x00
    uint8_t  flag;              // +0x01 (varies 0x00 / 0x01 — semantics TBD)
    uint16_t pad;               // +0x02 (always 0xFFFF)
    int32_t  fixed_size;        // +0x04 (payload byte count, signed; -1 = variable)
    char*    name_ptr;          // +0x08 (into packed string table at 0x01A509A8)
    void*    handler_ptr;       // +0x0C (into handler array at 0x01ED1CC0+8*msg_id)
    int32_t  unk1;              // +0x10 (varies 0..2)
    int32_t  unk2;              // +0x14 (typically 1)
    int32_t  sentinel;          // +0x18 (always -1 = 0xFFFFFFFF)
};
```

**Complete static msg_id table**:

| msg_id | Name | Size | msg_id | Name | Size |
|---|---|---|---|---|---|
| `0x00` | authenticate | 2 | `0x1D` | avatarUpdateNoAliasNoPosYawPitch | 12 |
| `0x01` | bandwidthNotification | 4 | `0x1E` | avatarUpdateNoAliasNoPosYaw | 11 |
| `0x02` | updateFrequencyNotification | 1 | `0x1F` | avatarUpdateNoAliasNoPosNoDir | 10 |
| `0x03` | setGameTime | 4 | `0x20` | avatarUpdateAliasFullPosYawPitchRoll | 25 |
| `0x04` | resetEntities | 1 | `0x21` | avatarUpdateAliasFullPosYawPitch | 24 |
| `0x05` | createBasePlayer | 2 | `0x22` | avatarUpdateAliasFullPosYaw | 23 |
| `0x06` | createCellPlayer | 2 | `0x23` | avatarUpdateAliasFullPosNoDir | 22 |
| `0x07` | spaceData | 2 | `0x24` | avatarUpdateAliasOnChunkYawPitchRoll | 25 |
| `0x08` | spaceViewportInfo | 13 | `0x25` | avatarUpdateAliasOnChunkYawPitch | 24 |
| `0x09` | createEntity | 2 | `0x26` | avatarUpdateAliasOnChunkYaw | 23 |
| `0x0A` | updateEntity | 2 | `0x27` | avatarUpdateAliasOnChunkNoDir | 22 |
| `0x0B` | entityInvisible | 5 | `0x28` | avatarUpdateAliasOnGroundYawPitchRoll | 25 |
| `0x0C` | leaveAoI | 2 | `0x29` | avatarUpdateAliasOnGroundYawPitch | 24 |
| `0x0D` | tickSync | 8 | `0x2A` | avatarUpdateAliasOnGroundYaw | 23 |
| `0x0E` | setSpaceViewport | 1 | `0x2B` | avatarUpdateAliasOnGroundNoDir | 22 |
| `0x0F` | setVehicle | 4 | `0x2C` | avatarUpdateAliasNoPosYawPitchRoll | 13 |
| `0x10` | avatarUpdateNoAliasFullPosYawPitchRoll | 25 | `0x2D` | avatarUpdateAliasNoPosYawPitch | 12 |
| `0x11` | avatarUpdateNoAliasFullPosYawPitch | 24 | `0x2E` | avatarUpdateAliasNoPosYaw | 11 |
| `0x12` | avatarUpdateNoAliasFullPosYaw | 23 | `0x2F` | avatarUpdateAliasNoPosNoDir | 10 |
| `0x13` | avatarUpdateNoAliasFullPosNoDir | 22 | `0x30` | detailedPosition | 41 |
| `0x14` | avatarUpdateNoAliasOnChunkYawPitchRoll | 25 | `0x31` | forcedPosition | 49 |
| `0x15` | avatarUpdateNoAliasOnChunkYawPitch | 24 | `0x32` | controlEntity | 5 |
| `0x16` | avatarUpdateNoAliasOnChunkYaw | 23 | `0x33` | voiceData | 2 |
| `0x17` | avatarUpdateNoAliasOnChunkNoDir | 22 | `0x34` | restoreClient | 2 |
| `0x18` | avatarUpdateNoAliasOnGroundYawPitchRoll | 25 | `0x35` | restoreBaseApp | 2 |
| `0x19` | avatarUpdateNoAliasOnGroundYawPitch | 24 | `0x36` | resourceFragment | 2 |
| `0x1A` | avatarUpdateNoAliasOnGroundYaw | 23 | `0x37` | loggedOff | 1 |
| `0x1B` | avatarUpdateNoAliasOnGroundNoDir | 22 | `0x38` | entityMessage | 2 |
| `0x1C` | avatarUpdateNoAliasNoPosYawPitchRoll | 13 | | | |

### Wire-format implications of the avatarUpdate decomposition

The 32-way decomposition (`{Alias|NoAlias} × {FullPos|OnChunk|OnGround|NoPos} × {YPR|YP|Y|NoDir}`) is the BigWorld "alias bit-packed avatar update" wire format. Each variant has a unique 1-byte msg_id, so the wire payload doesn't carry a "which fields are present" header — **the msg_id encodes that.**

Key payload-size deltas:

| Variant axis | Size delta | Conclusion |
|---|---|---|
| YPR → YP | -1 byte | Roll = 1 byte (8-bit quantized angle) |
| YP → Y | -1 byte | Pitch = 1 byte (8-bit quantized) |
| Y → NoDir | -1 byte | Yaw = 1 byte (8-bit quantized) |
| FullPos → NoPos | -12 bytes | Position = 3 × float32 (xyz) |
| OnChunk vs FullPos | 0 bytes | Same total — different reference frame, same byte width |
| OnGround vs FullPos | 0 bytes | Same total — Y is implicit (from ground height), but the byte width is preserved (possibly chunk-local coords + chunkID) |
| Alias vs NoAlias | 0 bytes | Same total — different addressing semantics, not size |

**8-bit angle quantization** (each axis = 256 steps over 360°, ~1.4° resolution) confirmed from byte-size deltas.

### Dynamic per-ServerConnection vec (msg_id 0x80..0xFE, 127 entries)

Source: PMT (PopulateMessageTypeTable) at runtime `0x00E563D0`. Located at `ServerConnection+0x190` (a sub-object containing its own vec). Entry stride: **`0x24` (36 bytes)** — adds two extra int32 fields for per-slot runtime statistics:

```c
struct InterfaceElementWithStats {   // 0x24 bytes — used in per-connection vec only
    InterfaceElement base;           // first 0x1C bytes (same as static)
    int32_t total_bytes_observed;    // +0x1C
    int32_t total_count_observed;    // +0x20
};
```

**All 127 entries share `name="entityMessage"`, handler=`0x01ED1CBC`**. The msg_id field of each slot is the literal "type tag" of the parent class (= `0x38` = entityMessage), so the slot's *index in the vec* is what disambiguates the actual entity method. Per-slot statistics confirm differential usage (e.g., msg_id `0x80` observed 0x0825 bytes across 89 messages during our session, msg_id `0x82` observed 0x092C bytes across 5 messages).

### The 0x39..0x7F gap

Indices 57..127 in the per-ServerConnection vec contain a **repeating 36-byte template pattern** (`00 02 BB EC ... 49 94 BC 01 ...`) — uninitialized/default-init. PMT does not populate these slots. No wire-protocol messages should use these msg_ids; doing so would dispatch to whatever default handler this pattern indexes (probably an error/drop).

### Why static and dynamic strides differ

The static `BWNetDriver::ClientInterface` array at `0x01F72518` is `0x1C`-stride (no statistics — it's read-only `.rdata`). When PMT initializes a `ServerConnection`, it calls `InterfaceElementVec::copyAllTo(static, dst=this+0x190)` which copies the 57 entries into a **fresh `0x24`-stride** layout in the per-connection sub-object (adding the two statistics fields). PMT then appends entries 0x80..0xFE to that sub-vec. The vec capacity is sized to 256 entries (= full 1-byte msg_id space, 0x2400 bytes total).

---

## Architectural link between protocol_digest and the dispatch table

The **same input** (entity-defs) drives both:

1. **`getProtocolVersionHash()`** computes SHA-1 over the entity-defs file content → 20 raw bytes
2. **HexEncoder(uppercase)** → 40 ASCII chars → stored at `ServerConnection+0x130`
3. **PMT** uses the same entity-defs to populate slots `0x80..0xFE` deterministically

Therefore: **server-side entity-defs that produce a matching SHA-1 digest also produce a matching slot mapping.** One commitment, two purposes. A mismatch in either commits the connection to slot-misalignment and refused dispatch.

---

## ServerConnection field map (partial, from `0xEEE2C100` dump)

| Offset | Value | Field | Notes |
|---|---|---|---|
| `+0x000` | `0x01A519D0` | vtable | 3 confirmed virtual methods (`0x00E648F0`, `0x00E583C0`, `0x00E56680`) |
| `+0x008` | `0xEBD766A0` | hash-table-like ptr | First bucket contains 20-char hex string "BD134C5F9AE86ECC8BA9" — possibly session token / per-cell hash |
| `+0x130` | basic_string (28B) | **protocol_digest** | SHA-1 hex, captured value above |
| `+0x190` | sub-object | **dispatch sub-vec** | Holds the 256-slot `InterfaceElementWithStats` array |

A full SC field map (~0x320 bytes) was dumped but not yet annotated; see `0xEEE2C100..0xEEE2C420` snapshot in session transcript for raw bytes.

---

## New symbol candidates (post-live-capture)

| Name | Runtime addr | Static addr | Notes |
|---|---|---|---|
| `ClientInterface_StaticRegistry` | `0x01F72518` | `0x01EF2518` | InterfaceElementVec global, 57 entries |
| `InterfaceElement_NameTable` | `0x01A509A8` | `0x019D09A8` | Packed null-terminated string table |
| `InterfaceElement_HandlerArray` | `0x01ED1CC0` | `0x01E51CC0` | 8-byte-stride handler ptr array |
| `EntityMessage_GenericHandler` | `0x01ED1CBC` | `0x01E51CBC` | One slot before handler array; the generic dispatcher for 0x80..0xFE |
| `InterfaceElementVec::copyAllTo` | `0x015F7F20` | `0x01577F20` | Called by PMT to seed dst vec from static |
| `InterfaceElementVec::pushBack` | `0x015F7260` | `0x015F7260` (?) | Called 127× by PMT loop |

---

## Operational notes for future live captures

1. **Silent BP gotcha persists**: `SetBreakpointFastResume=1` + `SetBreakpointCondition=0` causes the BP to *fire* (counter increments) but **suppresses** `logText`/`commandText` emission on this x64dbg-automate build. Use `condition=1` + `commandText="go"` for brief-halt-then-resume. Sub-second halt does not trip the server's 15s inactivity disconnect.

2. **Read while running works** — repeatedly confirmed during this session. Multiple `read_memory` calls (up to 2KB each) succeeded against the live heap with `Running: True`. No need to pause for memory inspection.

3. **ServerConnection lifetime**: object at `0xEEE2C100` is freed on disconnect; after-the-fact reads return reused heap data. Must capture during active session.

4. **`+0x190` is not the digest**, it's the dispatch sub-vec. The digest is at `+0x130`. The earlier static decompile's reference to `+0x130` for the basic_string was correct; the PMT-arg `this+0x190` is a separate sub-object that happens to live nearby.

---

## Tier 2 — Live Mercury packet capture (partial)

### Captured packet via singleshot BP at `Nub::processFilteredPacket` (runtime `0x01600840`)

**Method**: Singleshot BP, `wait_for_event` for hit, `get_all_registers` + `read_memory` × 2, then `go`. Auto-cleared after one hit.

**Halt duration cost**: MCP roundtrip latency (~5–10s total across the call sequence) was enough to trip the BaseApp Mercury handshake timeout. **Even singleshot is not viable for live capture on this network.** Future captures need either (a) `logText`-only with deep dereferences (no halt, but `logText` emission was unreliable with `fastResume=1` on this x64dbg-automate build), or (b) external network tap (Wireshark + decrypt keys captured offline).

### Captured first inbound packet — call frame at BP entry

**Registers (EIP=`0x01600840`, function entry pre-prologue)**:

```
eax = 0x00000005      ecx = 0xEE4DC290 (ServerConnection+0x190 sub-object)
ebx = 0x21A2F84C      edx = 0xEC147904 (pkt+4)
esp = 0x21A2F604      esi = 0xEC147900 (= arg2, caller had pkt in ESI too)
edi = 0x21A2F768
```

**Stack frame at entry**:

```
[esp+0x00] = 0x016EF6E9  ← return into mswsock callback
[esp+0x04] = 0x21A2F8B0  ← arg1 (stack ptr to channel handle)
[esp+0x08] = 0xEC147900  ← arg2 (Packet*) ← THIS IS THE REAL PACKET
[esp+0x0C] = 0x21A2F768  ← arg3 (stack ref)
```

**`Nub::processFilteredPacket` signature** (verified): `__thiscall(ecx=sub_dispatch, arg1=channel_handle*, arg2=Packet*, arg3=stack_ref)`. The `this` of this function is the ServerConnection+0x190 sub-object (not the Nub itself), confirming the dispatch flows through the per-connection InterfaceElementVec.

### Captured Packet struct at `0xEC147900` (256 bytes raw, ~0x94 non-zero)

```c
struct Packet {                       // 0x60-byte header + buffer
    void*    vtable;                  // +0x00 = 0x01B99704  ← NEW SYMBOL: Packet vtable
    uint32_t field_04;                // +0x04 = 5           ← refcount or msg-in-bundle count
    uint32_t pad_08;                  // +0x08 = 0
    uint32_t field_0C;                // +0x0C = 0x17068CDD  ← random/hash
    uint32_t field_10;                // +0x10 = 0x0003C234  ← timestamp or counter
    uint8_t  pad_14[0x10];            // +0x14..0x23 = zeros
    uint32_t payload_length;          // +0x24 = 0x23 = 35   ← likely length field
    uint8_t  pad_28[0x1F];            // +0x28..0x46 = zeros
    uint8_t  flag_47;                 // +0x47 = 0x10
    uint8_t  pad_48[0x07];            // +0x48..0x4E = zeros
    uint8_t  flag_4F;                 // +0x4F = 0x10
    uint32_t flag_50;                 // +0x50 = 0x10000000
    uint8_t  field_54;                // +0x54 = 0x58 (read by disasm but dead-code-checked)
    uint8_t  pad_55[3];               // +0x55..0x57
    uint32_t field_58;                // +0x58 = 0x99460000
    uint32_t field_5C;                // +0x5C = 0x34140000
    uint8_t  data[];                  // +0x60 onward = actual payload buffer
};
```

### Captured data bytes (+0x60..+0x94)

```
+0x60..+0x72  ASCII "76983D3F18225245676"            (19 chars — session token format unknown)
+0x73..+0x76  01 00 00 00                            (flag or length=1 little-endian)
+0x77..+0x86  78 83 3C A5 10 2D EA F8 3C 51 FF DD A9 C6 A0 2B   (16 bytes random)
+0x87..+0x93  8F B6 C3 E6 C4 F0 73 28 8E 2B 60 6C EC            (13 bytes random)
+0x94+        zeros
```

**Total non-zero data: 52 bytes** (not the 35 implied by `+0x24`). Either `+0x24=35` measures a different field or the buffer continues past the declared length.

### Interpretation candidates for the payload

| Bytes | Interpretation candidates |
|---|---|
| `"76983D3F18225245676"` (19 ASCII) | Server-issued session token; unusual length suggests a composite format like `<hash_hex8>+<nonce_dec10>+<checkdigit1>`. Not a standard hash output. |
| `01 00 00 00` | Length prefix for next field (1 byte? — but next chunk is 16 bytes), OR a flag byte + padding |
| `78 83 3C A5 10 2D EA F8 3C 51 FF DD A9 C6 A0 2B` (16 bytes) | **Strong candidate for AES-128/256 key fragment** or 128-bit IV |
| `8F B6 C3 E6 C4 F0 73 28 8E 2B 60 6C EC` (13 bytes) | Trailing bytes — possibly HMAC tag prefix, more key material, or fragment of next message |

### New Mercury symbol candidates from this capture

| Name | Runtime addr | Static addr | Source |
|---|---|---|---|
| `Mercury::Packet::vtable` (or `Bundle::vtable`) | `0x01B99704` | `0x01B19704` | First dword of Packet struct |
| `Nub::processFilteredPacket` (confirmed) | `0x01600840` | `0x01580840` | BP hit confirmed function entry |
| `Nub::processFilteredPacket+0x4A` log string | `0x01B97E98` | `0x01B17E98` | `"Nub::processFilteredPacket( %s )"` — function self-name |

### Tier 2/3 items still open (deferred)

- **More packet samples** — need either `logText`-only path or external pcap tap. Halt-based singleshot too slow on this network.
- **R14 lifetime retry cap**: BP at the abort site inside `UnAckedHandler::checkResendTimers` (runtime `0x0160C420`).
- **AES-256-CBC + HMAC-MD5 session keys**: BP at the key-install site, OR decrypt the 16-byte random in the captured packet to test the key-derivation hypothesis.
- **rdtsc baseline write site (Q2)**: hardware-write BP on `[ServerConnection+0x170]` / `+0x174`.
- **Outbound Bundle::newMessage capture**: mirror of inbound, on the send path.
