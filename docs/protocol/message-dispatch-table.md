# Mercury Message Dispatch Table

> **Last updated**: 2026-03-05
> **RE Status**: COMPLETE — verified against both C++ server source and Ghidra client binary analysis
> **Sources**: `src/baseapp/mercury/sgw/messages.hpp`, `src/baseapp/mercury/sgw/messages.cpp`, Ghidra analysis of sgw.exe ClientInterface

---

## Overview

The Mercury protocol uses a single-byte message ID (0x00-0xFF) to identify each message within a bundle. The message ID determines both the handler function and how the message length is encoded on the wire.

### Message ID Ranges

| Range | Purpose | Length Encoding |
|-------|---------|-----------------|
| 0x00-0x7F | System messages (fixed dispatch table) | Per-message (see table below) |
| 0x80-0xBF | Entity cell method calls | Always WORD_LENGTH (u16 prefix) |
| 0xC0-0xFE | Entity base/proxy method calls | Always WORD_LENGTH (u16 prefix) |
| 0xFF | Reply message (special) | WORD_LENGTH (u16 prefix) |

### Length Types

| Type | Value | Wire Format | Description |
|------|-------|-------------|-------------|
| CONSTANT_LENGTH | 0 | No length prefix | Fixed-size payload; length from table |
| WORD_LENGTH | 2 | `u16 LE` prefix | Variable-size; u16 byte count before payload |
| DWORD_LENGTH | 4 | `u32 LE` prefix | Variable-size; u32 byte count before payload |

### Dispatch Logic (from `src/mercury/bundle.cpp`)

```cpp
if (messageId >= 0x80)
    type = Message::WORD_LENGTH;           // Entity methods: always u16 length prefix
else if (messageId < table_.size)          // System messages: lookup in table
{
    type = table_.messages[messageId].lengthType;
    length = table_.messages[messageId].length;
}
else
    // ERROR: unhandled message
```

---

## Server-to-Client Messages (ClientInterface)

These are messages sent FROM the server TO the client. The client's `ServerConnection` class handles them.

Table size: `0x39` (57 entries, indices 0x00-0x38). Defined in `ServerMessageList[]`.

### System Messages (0x00-0x0F)

| Msg ID | Name | Length Type | Const Len | Entity? | Handler Address | Description |
|--------|------|------------|-----------|---------|-----------------|-------------|
| 0x00 | authenticate | DWORD_LENGTH | — | No | `ServerConnection_authenticate` 0x00dd8510 | Session auth (never observed on wire) |
| 0x01 | bandwidthNotification | CONSTANT | 4 | No | (unnamed) | Max bandwidth in bps |
| 0x02 | updateFrequencyNotification | CONSTANT | 1 | No | (unnamed) | Server tick rate (usually 10) |
| 0x03 | setGameTime | CONSTANT | 4 | No | (unnamed) | Initial game time in ticks |
| 0x04 | resetEntities | CONSTANT | 1 | No | (unnamed) | Tear down client entity system |
| 0x05 | createBasePlayer | WORD | — | No | `ServerConnection_createBasePlayer` 0x00dddca0 | Create base part of player entity |
| 0x06 | createCellPlayer | WORD | — | No | `ServerConnection_createCellPlayer` 0x00dda2e0 | Create cell part of player entity |
| 0x07 | spaceData | WORD | — | No | `ServerConnection_spaceData` 0x00dda540 | Space data key/value (legacy) |
| 0x08 | spaceViewportInfo | CONSTANT | 13 | No | `ServerConnection_spaceViewportInfo` 0x00dda6c0 | Viewport entity + space binding |
| 0x09 | createEntity | WORD | — | No | (unnamed) | Create NPC/object entity |
| 0x0A | updateEntity | WORD | — | No | (unnamed) | Update entity properties |
| 0x0B | entityInvisible | CONSTANT | 5 | No | (unnamed) | Entity about to leave AoI |
| 0x0C | leaveAoI | WORD | — | No | (unnamed) | Entity exits area of interest |
| 0x0D | tickSync | CONSTANT | 8 | No | (unnamed) | Game tick synchronization (10 Hz) |
| 0x0E | setSpaceViewport | CONSTANT | 1 | No | (unnamed) | Switch active viewport |
| 0x0F | setVehicle | CONSTANT | 4 | No | (unnamed) | Entity enters/exits vehicle |

### Avatar Update Messages (0x10-0x2F)

32 variants encoding position updates with different precision/compression. Each variant encodes a combination of:
- **Alias**: Whether the entity uses an ID alias (1 byte) or full ID (4 bytes)
- **Position**: FullPos (absolute), OnChunk (relative), OnGround (height-mapped), NoPos (direction only)
- **Direction**: YawPitchRoll (3 bytes), YawPitch (2 bytes), Yaw (1 byte), NoDir (0 bytes)

All are CONSTANT_LENGTH and are entity messages (dispatched to entity). Only 0x10 is used by the SGW server.

| Msg ID | Name | Const Len | Description |
|--------|------|-----------|-------------|
| 0x10 | avatarUpdateNoAliasFullPosYawPitchRoll | 25 | **Primary movement update** |
| 0x11 | avatarUpdateNoAliasFullPosYawPitch | 24 | |
| 0x12 | avatarUpdateNoAliasFullPosYaw | 23 | |
| 0x13 | avatarUpdateNoAliasFullPosNoDir | 22 | |
| 0x14 | avatarUpdateNoAliasOnChunkYawPitchRoll | 25 | |
| 0x15 | avatarUpdateNoAliasOnChunkYawPitch | 24 | |
| 0x16 | avatarUpdateNoAliasOnChunkYaw | 23 | |
| 0x17 | avatarUpdateNoAliasOnChunkNoDir | 22 | |
| 0x18 | avatarUpdateNoAliasOnGroundYawPitchRoll | 25 | |
| 0x19 | avatarUpdateNoAliasOnGroundYawPitch | 24 | |
| 0x1A | avatarUpdateNoAliasOnGroundYaw | 23 | |
| 0x1B | avatarUpdateNoAliasOnGroundNoDir | 22 | |
| 0x1C | avatarUpdateNoAliasNoPosYawPitchRoll | 13 | |
| 0x1D | avatarUpdateNoAliasNoPosYawPitch | 12 | |
| 0x1E | avatarUpdateNoAliasNoPosYaw | 11 | |
| 0x1F | avatarUpdateNoAliasNoPosNoDir | 10 | |
| 0x20 | avatarUpdateAliasFullPosYawPitchRoll | 22 | |
| 0x21 | avatarUpdateAliasFullPosYawPitch | 21 | |
| 0x22 | avatarUpdateAliasFullPosYaw | 20 | |
| 0x23 | avatarUpdateAliasFullPosNoDir | 19 | |
| 0x24 | avatarUpdateAliasOnChunkYawPitchRoll | 22 | |
| 0x25 | avatarUpdateAliasOnChunkYawPitch | 21 | |
| 0x26 | avatarUpdateAliasOnChunkYaw | 20 | |
| 0x27 | avatarUpdateAliasOnChunkNoDir | 19 | |
| 0x28 | avatarUpdateAliasOnGroundYawPitchRoll | 22 | |
| 0x29 | avatarUpdateAliasOnGroundYawPitch | 21 | |
| 0x2A | avatarUpdateAliasOnGroundYaw | 20 | |
| 0x2B | avatarUpdateAliasOnGroundNoDir | 19 | |
| 0x2C | avatarUpdateAliasNoPosYawPitchRoll | 10 | |
| 0x2D | avatarUpdateAliasNoPosYawPitch | 9 | |
| 0x2E | avatarUpdateAliasNoPosYaw | 8 | |
| 0x2F | avatarUpdateAliasNoPosNoDir | 7 | |

### Extended System Messages (0x30-0x38)

| Msg ID | Name | Length Type | Const Len | Entity? | Handler Address | Description |
|--------|------|------------|-----------|---------|-----------------|-------------|
| 0x30 | detailedPosition | CONSTANT | 41 | No | (unnamed) | Full position+velocity+rotation for non-player entities |
| 0x31 | forcedPosition | CONSTANT | 49 | No | `ServerConnection_forcedPosition` 0x00dd9ee0 | Teleport/force position for controlled entities |
| 0x32 | controlEntity | CONSTANT | 5 | No | (unnamed) | Toggle local control of entity |
| 0x33 | voiceData | WORD | — | No | `ServerConnection_voiceData` 0x00dd68b0 | Voice chat (unused by SGW) |
| 0x34 | restoreClient | WORD | — | No | `ServerConnection_restoreClient` 0x00dd8ae0 | Restore client state after reconnect |
| 0x35 | restoreBaseApp | WORD | — | No | (unnamed) | BaseApp recovery notification |
| 0x36 | resourceFragment | WORD | — | No | `ServerConnection_resourceFragment` 0x00dddd80 | Cooked data fragment delivery |
| 0x37 | loggedOff | CONSTANT | 1 | No | `ServerConnection_loggedOff` 0x00dd8c20 | Server disconnection with reason code |
| 0x38 | entityMessage | WORD | — | Yes | (entity dispatch) | Catch-all for entity client method calls |

### Entity Method Messages (0x80-0xFF)

Messages >= 0x80 are entity method calls. They always use WORD_LENGTH encoding.

| Range | Purpose | Base Offset | Description |
|-------|---------|-------------|-------------|
| 0x80-0xBF | Cell entity methods | `msg_id & 0x3F` | Cell methods on the entity's server-side cell |
| 0xC0-0xFE | Base/proxy entity methods | `msg_id & 0x3F` | Base methods on the entity's server-side proxy |
| 0xFF | Reply message | — | Internal Mercury request-reply mechanism |

The method index within each range is `msg_id & 0x3F`. The actual method called depends on the entity type's `.def` file.

### Special: Reply Message (0xFF)

Handled by `Mercury::ReplyMessageHandler`, not through the regular dispatch table. Used internally for Mercury's request-reply mechanism (e.g., baseAppLogin response).

---

## Client-to-Server Messages (ServerInterface)

These are messages sent FROM the client TO the server. The server's `client_handler.cpp` processes them.

Table size: `0x0D` (13 entries, indices 0x00-0x0C). Defined in `ClientMessageList[]`.

| Msg ID | Name | Length Type | Const Len | Entity? | Description |
|--------|------|------------|-----------|---------|-------------|
| 0x00 | baseAppLogin | WORD | — | No | Initial connection (unencrypted, 41 bytes: u32 accountId + 32-byte AES key + overhead) |
| 0x01 | authenticate | WORD | — | No | Session ticket (20-byte ticket; server ignores per C++ line 131) |
| 0x02 | avatarUpdateImplicit | CONSTANT | 36 | Yes | Implicit position update (unused by SGW) |
| 0x03 | avatarUpdateExplicit | CONSTANT | 40 | Yes | **Primary client movement update** |
| 0x04 | avatarUpdateWardImplicit | CONSTANT | 36 | Yes | Ward entity position (unused by SGW) |
| 0x05 | avatarUpdateWardExplicit | CONSTANT | 40 | Yes | Ward entity position (unused by SGW) |
| 0x06 | switchInterface | CONSTANT | 0 | No | Deprecated — not used |
| 0x07 | requestEntityUpdate | WORD | — | Yes | Request entity data for entity entering AoI |
| 0x08 | enableEntities | CONSTANT | 8 | Yes | Notify server that client entity system is ready |
| 0x09 | viewportAck | CONSTANT | 8 | No | Acknowledge viewport change |
| 0x0A | vehicleAck | CONSTANT | 8 | No | Acknowledge vehicle update |
| 0x0B | restoreClientAck | WORD | — | No | Acknowledge client restoration |
| 0x0C | disconnect | CONSTANT | 1 | No | Client-initiated disconnection (1-byte reason) |

### Client Entity Method Messages (0x80-0xFF)

Same as server-to-client: messages >= 0x80 are entity method calls with WORD_LENGTH encoding.

| Range | Encoding | Description |
|-------|----------|-------------|
| 0x80-0xBF | `msg_id = cellMethodIndex \| 0x80` | Cell method call (spatial/physics) |
| 0xC0-0xFE | `msg_id = baseMethodIndex \| 0xC0` | Base method call (persistent state) |

Evidence from client binary (Ghidra):
- `ServerConnection_startEntityMessage` at 0x00dd6a60: sets byte to `param_1 | 0x80`
- `ServerConnection_startProxyMessage` at 0x00dd6980: sets byte to `param_1 | 0xC0`

---

## Dispatch Table Structure

### Client Binary (Ghidra)

The client uses a static array of `InterfaceElement` objects for the ClientInterface (server-to-client messages). Each element is **0x90 (144) bytes** in the static initializer data.

**Table location in sgw.exe**:

| Component | Address Range | Stride | Description |
|-----------|--------------|--------|-------------|
| ServerInterface header | 0x017bac00-0x017bac6f | — | Interface name + entity handler |
| ServerInterface entries (C->S) | 0x017bac70-0x017baf0c | 0x30 (48 bytes) | 14 client-to-server entries |
| ClientInterface header | 0x017baf10-0x017bafcf | — | Interface name ("ClientInterface") + entity handler |
| ClientInterface entries (S->C) | 0x017bafd0-0x017bcf4f | 0x90 (144 bytes) | 57 server-to-client entries |

Each static InterfaceElement entry contains:
- Message name string pointer (at known offset within entry)
- Handler function pointer (vtable-based dispatch)
- Length type and constant length
- RTTI type info (for C++ `ClientMessageHandler<XxxArgs>` template)

### Runtime Dispatch (Nub::processOrderedPacket)

At runtime, the Nub maintains a vector of **0x24 (36) byte** runtime InterfaceElement objects, indexed directly by message ID byte. The dispatch in `Mercury_Nub_6` (processOrderedPacket) at 0x0157c820:

```c
// Read message ID byte
uint msgId = (uint)(byte)readByte(stream);

// Index into elements array (36 bytes per entry)
InterfaceElement* elem = &nub->elements[msgId];  // this+0xc + msgId * 0x24

// Check if handler exists
if (elem->handler == NULL)
    // ERROR: unhandled message

// Dispatch to handler via vtable
elem->handler->handleMessage(source, stream, msg, elem->userData);
```

### Server Code (Cimmeria C++)

The server uses `Message::Table` and `Message::Format` structs from `src/mercury/message.hpp`:

```cpp
struct Format {
    Length lengthType;       // CONSTANT_LENGTH, WORD_LENGTH, or DWORD_LENGTH
    unsigned int length;     // Fixed length (for CONSTANT_LENGTH only)
    const char * name;       // Debug name
    bool isEntityMessage;    // Dispatched to entity?
};

struct Table {
    unsigned int size;             // Number of entries
    Format const * messages;       // Array of Format entries
    Format const entityMessage;    // Overflow handler for entity methods (>= 0x80)
};
```

---

## Known Entity Method IDs

These are the entity method IDs used by the Cimmeria server, derived from `.def` files and confirmed via pcap analysis.

### ClientCache Interface (Base Methods, prefix 0xC0)

| Wire ID | Method Index | Method Name | Direction |
|---------|-------------|-------------|-----------|
| 0xC0 | 0 | versionInfoRequest | C->S |
| 0xC1 | 1 | elementDataRequest | C->S |

### Account Entity — Base Methods (prefix 0xC0)

| Wire ID | Method Index | Method Name | Direction |
|---------|-------------|-------------|-----------|
| 0xC2 | 2 | logOff | C->S |
| 0xC3 | 3 | createCharacter | C->S |
| 0xC4 | 4 | playCharacter | C->S |
| 0xC5 | 5 | deleteCharacter | C->S |
| 0xC6 | 6 | requestCharacterVisuals | C->S |
| 0xC7 | 7 | onClientVersion | C->S |

### Account Entity — Client Methods (prefix 0x80)

| Wire ID | Method Index | Method Name | Direction |
|---------|-------------|-------------|-----------|
| 0x80 | 0 | onVersionInfo | S->C |
| 0x81 | 1 | onCookedDataError | S->C |
| 0x82 | 2 | onCharacterList | S->C |
| 0x83 | 3 | onCharacterCreateFailed | S->C |
| 0x84 | 4 | onCharacterVisuals | S->C |
| 0x85 | 5 | onCharacterLoadFailed | S->C |

> **Note**: Method indices only count `<Exposed/>` methods. Non-exposed methods (e.g., `logOffInternal`, `onPlayerFailedToLoad`) are SKIPPED in the client-server indexing. See MEMORY.md for details.

---

## Wire Format Examples

### System Message (CONSTANT_LENGTH)

```
[msg_id: 1 byte] [payload: N bytes]     (N = table lookup)

Example: tickSync (0x0D, CONSTANT_LENGTH=8)
  0D                                      -- msg_id
  XX XX XX XX                             -- tick: u32 LE
  XX XX XX XX                             -- tickRate: u32 LE
```

### System Message (WORD_LENGTH)

```
[msg_id: 1 byte] [length: u16 LE] [payload: length bytes]

Example: createBasePlayer (0x05, WORD_LENGTH)
  05                                      -- msg_id
  06 00                                   -- length: 6 bytes
  XX XX XX XX                             -- entityId: u32 LE
  XX                                      -- classId: u8
  XX                                      -- propertyCount: u8
```

### Entity Method Call (always WORD_LENGTH)

```
[msg_id: 1 byte] [length: u16 LE] [payload: length bytes]

Example: playCharacter (0xC4 = base method index 4 | 0xC0)
  C4                                      -- msg_id (base method 4)
  04 00                                   -- length: 4 bytes
  XX XX XX XX                             -- charDefId: u32 LE
```

---

## Cross-References

- [Mercury Wire Format](mercury-wire-format.md) — packet-level protocol details
- [Login Handshake](login-handshake.md) — connection establishment sequence
- [Message Catalog](message-catalog.md) — full entity method event catalog (975 messages)
- [Position Updates](position-updates.md) — avatar update wire format details
- C++ source: `src/baseapp/mercury/sgw/messages.hpp` and `messages.cpp`
- Rust source: `crates/mercury/src/messages.rs`
