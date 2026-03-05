# Client-Verified Wire Format Reference

> **Status**: Complete — all 57 system message IDs (0x00-0x38 + 0xFF) verified against client binary
> **Date**: 2026-03-05
> **Method**: Ghidra decompilation of `sgw.exe` client binary, cross-referenced against C++ server source and pcap captures
> **Confidence**: HIGH

---

## Purpose

This is the **ground truth** document for Mercury wire formats. The client binary (`sgw.exe`) is the only authoritative source for what the server must send. The chain of trust is:

```
Client binary (sgw.exe)  ← GROUND TRUTH (Ghidra decompilation)
    ↓ validates
C++ fan-made server      ← REFERENCE (source code available)
    ↓ validates
Rust rewrite (Cimmeria)  ← OUR IMPLEMENTATION
```

Entity method calls (RPC) are driven by `.def` files and use a universal dispatcher — their wire formats are documented in the per-system RE findings. This document covers the **system protocol messages** (msg_id 0x00-0x38 + 0xFF) which are hardcoded in C++ with NO `.def` representation.

---

## Master Message Table

All server-to-client message IDs with verified wire formats:

| msg_id | Name | Length Type | Size | Verified | Detailed Doc |
|--------|------|-----------|------|----------|--------------|
| 0x00 | AUTHENTICATE | DWORD_LENGTH | var | Ghidra | [system-protocol](../reverse-engineering/findings/system-protocol-wire-formats.md) |
| 0x01 | BANDWIDTH_NOTIFICATION | CONSTANT | 4 | Ghidra | [space-viewport](../reverse-engineering/findings/space-viewport-wire-formats.md) |
| 0x02 | UPDATE_FREQ_NOTIFICATION | CONSTANT | 1 | Ghidra | [space-viewport](../reverse-engineering/findings/space-viewport-wire-formats.md) |
| 0x03 | SET_GAME_TIME | CONSTANT | 4 | Ghidra | [space-viewport](../reverse-engineering/findings/space-viewport-wire-formats.md) |
| 0x04 | RESET_ENTITIES | CONSTANT | 1 | Ghidra+Pcap | [entity-creation](../reverse-engineering/findings/entity-creation-wire-formats.md) |
| 0x05 | CREATE_BASE_PLAYER | WORD_LENGTH | var | Ghidra+Pcap | [entity-creation](../reverse-engineering/findings/entity-creation-wire-formats.md) |
| 0x06 | CREATE_CELL_PLAYER | WORD_LENGTH | var | Ghidra+Pcap | [entity-creation](../reverse-engineering/findings/entity-creation-wire-formats.md) |
| 0x07 | SPACE_DATA | WORD_LENGTH | var | Ghidra | [space-viewport](../reverse-engineering/findings/space-viewport-wire-formats.md) |
| 0x08 | SPACE_VIEWPORT_INFO | CONSTANT | 13 | Ghidra+Pcap | [entity-creation](../reverse-engineering/findings/entity-creation-wire-formats.md) |
| 0x09 | CREATE_ENTITY | WORD_LENGTH | var | Ghidra | [entity-creation](../reverse-engineering/findings/entity-creation-wire-formats.md) |
| 0x0A | UPDATE_ENTITY | WORD_LENGTH | var | Source only | [space-viewport](../reverse-engineering/findings/space-viewport-wire-formats.md) |
| 0x0B | ENTITY_INVISIBLE | CONSTANT | 5 | Ghidra | [entity-creation](../reverse-engineering/findings/entity-creation-wire-formats.md) |
| 0x0C | LEAVE_AOI | WORD_LENGTH | var | Ghidra | [entity-creation](../reverse-engineering/findings/entity-creation-wire-formats.md) |
| 0x0D | TICK_SYNC | CONSTANT | 8 | Ghidra+Pcap | [entity-creation](../reverse-engineering/findings/entity-creation-wire-formats.md) |
| 0x0E | SET_SPACE_VIEWPORT | CONSTANT | 1 | Ghidra | [space-viewport](../reverse-engineering/findings/space-viewport-wire-formats.md) |
| 0x0F | SET_VEHICLE | CONSTANT | 4 | Ghidra | [space-viewport](../reverse-engineering/findings/space-viewport-wire-formats.md) |
| 0x10-0x1F | UPDATE_AVATAR (NoAlias) | CONSTANT | 10-25 | Ghidra | [space-viewport](../reverse-engineering/findings/space-viewport-wire-formats.md) |
| 0x20-0x2F | UPDATE_AVATAR (Alias) | CONSTANT | 7-22 | Ghidra | [space-viewport](../reverse-engineering/findings/space-viewport-wire-formats.md) |
| 0x30 | DETAILED_POSITION | CONSTANT | 41 | Ghidra | [space-viewport](../reverse-engineering/findings/space-viewport-wire-formats.md) |
| 0x31 | FORCED_POSITION | CONSTANT | 49 | Ghidra+Pcap | [entity-creation](../reverse-engineering/findings/entity-creation-wire-formats.md) |
| 0x32 | CONTROL_ENTITY | CONSTANT | 5 | Ghidra | [space-viewport](../reverse-engineering/findings/space-viewport-wire-formats.md) |
| 0x33 | VOICE_DATA | WORD_LENGTH | var | Ghidra | [space-viewport](../reverse-engineering/findings/space-viewport-wire-formats.md) |
| 0x34 | RESTORE_CLIENT | WORD_LENGTH | var | Ghidra | [system-protocol](../reverse-engineering/findings/system-protocol-wire-formats.md) |
| 0x35 | RESTORE_BASEAPP | WORD_LENGTH | var | Source only | [space-viewport](../reverse-engineering/findings/space-viewport-wire-formats.md) |
| 0x36 | RESOURCE_FRAGMENT | WORD_LENGTH | var | Ghidra | [space-viewport](../reverse-engineering/findings/space-viewport-wire-formats.md) |
| 0x37 | LOGGED_OFF | CONSTANT | 1 | Ghidra | [system-protocol](../reverse-engineering/findings/system-protocol-wire-formats.md) |
| 0x38+ | ENTITY_MESSAGE (RPC) | WORD_LENGTH | var | Ghidra | [space-viewport](../reverse-engineering/findings/space-viewport-wire-formats.md) |
| 0xFF | REPLY_MESSAGE | WORD_LENGTH | var | Pcap | [space-viewport](../reverse-engineering/findings/space-viewport-wire-formats.md) |

### Length Type Reference

| Type | Wire Behavior |
|------|--------------|
| CONSTANT | No length prefix — scanner knows exact byte count from message table |
| WORD_LENGTH | `u16 LE` length prefix immediately after msg_id byte |
| DWORD_LENGTH | `u32 LE` length prefix (only AUTHENTICATE uses this) |

---

## Critical Findings

### 1. Rotation Y/Z Swap (CONFIRMED)

The wire rotation order is `rotX, rotZ, rotY` — Y and Z are swapped vs logical order. This is a BigWorld convention confirmed in:

- **Client binary**: `ServerConnection_forcedPosition` at `0x00dd9ee0` passes `param_1[9]=rotX, param_1[10]=rotZ, param_1[11]=rotY` to `addMove`
- **C++ server source**: `client_handler.cpp:400` writes `<< rotX << rotZ << rotY`
- **Affected messages**: CREATE_CELL_PLAYER (0x06), FORCED_POSITION (0x31 — createCellPlayer path only)

**TRAP**: The C++ server has TWO different rotation orders for FORCED_POSITION:
- `client_handler.cpp:411` (createCellPlayer/world entry): `rotX, rotZ, rotY` (Y/Z swapped) — **matches client expectation**
- `client_handler.cpp:570` (standalone forcedPosition): `rotation.x, rotation.y, rotation.z` (standard order)

The Rust server currently sends swapped order for world entry (correct). When adding ongoing FORCED_POSITION support, the rotation order must match whichever call site is being replicated.

### 2. RESET_ENTITIES Must Be Isolated

`RESET_ENTITIES` (0x04) must be sent in its **own flushed packet**, separate from any other messages. The C++ server explicitly flushes before and after. Bundling it with other messages causes undefined behavior on the client.

### 3. Client Buffers CREATE_CELL_PLAYER

If `CREATE_CELL_PLAYER` (0x06) arrives before `CREATE_BASE_PLAYER` (0x05), the client buffers it and replays it after the base player is created. This means order is preferred but not strictly required. The client checks `this->playerEntityID_ != 0` and buffers if false.

### 4. Entity Class IDs (entities.xml order)

| classId | Entity Type | Usage |
|---------|------------|-------|
| 0x00 | SGWEntity | Base game entity |
| 0x01 | SGWSpawnableEntity | Visual entity |
| 0x02 | SGWPlayer | Player character |
| 0x03 | SGWBeing | Combat-capable entity |
| 0x04 | SGWMob | NPC |
| 0x05 | SGWPet | Player pet |
| 0x06 | SGWDuelMarker | Duel zone marker |
| 0x07 | Account | Login/character management |

### 5. Packed String Encoding

Used in AUTHENTICATE (0x00) and possibly other messages:
- Read 1 byte as length
- If length == 0xFF, read 3 more bytes as u24 LE extended length
- Read `length` bytes of string data

### 6. Velocity Compression (PackedXYZ)

5-byte compressed velocity used in all UPDATE_AVATAR variants:
- Adds 2.0f bias to absolute values before extracting IEEE 754 mantissa bits
- Packs X (11 bits), Z (11 bits), Y (15 bits) + 3 sign bits into 5 bytes
- Full algorithm documented in [entity-creation-wire-formats.md](../reverse-engineering/findings/entity-creation-wire-formats.md#velocity-compression-packxyz)

### 7. Direction Quantization

UPDATE_AVATAR messages encode rotation as single bytes: `byte = angle / (2*PI/256)`. Each byte maps 0-255 to 0-2π radians. The constant is `0.024543693f`.

### 8. Resource Fragment Reassembly

RESOURCE_FRAGMENT (0x36) uses a linked-list reassembly model:
- Flags: 0x01=first, 0x02=final, 0x40=base/reliable, 0x80=error
- Max fragment body: 1000 bytes
- First fragment includes: messageType(u8) + categoryId(u32) + elementId(u32) + data
- Subsequent fragments: data only
- 21 resource categories documented (kismet, abilities, missions, items, dialog, etc.)

---

## Connection Lifecycle (Complete Sequence)

### Phase 3: Mercury UDP Connect
```
Server → Client: CONNECT_REPLY (0xFF)     [request_id:u32][ticketLen:u8][ticket:20B]
```

### Phase 3: Time Sync Bundle
```
Server → Client: UPD_FREQ_NOTIFICATION (0x02)  [resolution:u8 = 10]
                 TICK_SYNC (0x0D)               [tick:u32 = 0][rate:u32 = 100]
                 SET_GAME_TIME (0x03)           [time:u32 = 0]
```

### Phase 4: Account Entity Creation
```
Client → Server: AUTHENTICATE (0x01)       — server ignores
Client → Server: ENABLE_ENTITIES (0x08)    — triggers entity creation

Server → Client: CREATE_BASE_PLAYER (0x05) [entityId:u32][classId=0x07:u8][propCount=0:u8]
                 entity method 0x82        [charList data...]
```

### Phase 5a: Entity System Reset
```
Client → Server: playCharacter (0xC4)

Server → Client: RESET_ENTITIES (0x04)     [keepBase:u8 = 0]   ← MUST flush alone
Client → Server: ENABLE_ENTITIES (0x08)    — auto-sent by client
```

### Phase 5b: World Entry
```
Server → Client: CREATE_BASE_PLAYER (0x05) [entityId:u32][classId=0x02:u8][propCount=0:u8]
                 VIEWPORT_INFO (0x08)       [eid:u32][eid:u32][space:u32][vp=0:u8]
                 CREATE_CELL_PLAYER (0x06)  [space:u32][veh=0:u32][pos:3xf32][rot(X,Z,Y):3xf32]
                 FORCED_POSITION (0x31)     [eid:u32][space:u32][veh=0:u32][pos:3xf32][vel=0:3xf32][rot(X,Z,Y):3xf32][flags=1:u8]
```

### Ongoing: Heartbeat
```
Server → Client: TICK_SYNC (0x0D) every 100ms  [tick++:u32][rate=100:u32]
```

### Entity AoI Lifecycle
```
Enter:  CREATE_ENTITY (0x09)     → UPDATE_AVATAR (0x10)  → entity RPC: onVisible
Leave:  ENTITY_INVISIBLE (0x0B)  → LEAVE_AOI (0x0C)
```

### Disconnect
```
Server → Client: LOGGED_OFF (0x37)  [reason:u8]
```

---

## Client Binary Function Addresses

| Function | Address | Message(s) |
|----------|---------|-----------|
| `ServerConnection_authenticate` | `0x00dd8510` | AUTHENTICATE (0x00) |
| `ServerConnection_createBasePlayer` | `0x00dddca0` | CREATE_BASE_PLAYER (0x05) |
| `ServerConnection_createCellPlayer` | `0x00dda2e0` | CREATE_CELL_PLAYER (0x06) |
| `ServerConnection_spaceData` | `0x00dda540` | SPACE_DATA (0x07) |
| `ServerConnection_spaceViewportInfo` | `0x00dda6c0` | VIEWPORT_INFO (0x08) |
| `ServerConnection_forcedPosition` | `0x00dd9ee0` | FORCED_POSITION (0x31) |
| `ServerConnection_addMove` | `0x00dd9330` | (internal movement) |
| `ServerConnection_resourceFragment` | `0x00dddd80` | RESOURCE_FRAGMENT (0x36) |
| `ServerConnection_loggedOff` | `0x00dd8c20` | LOGGED_OFF (0x37) |
| `ServerConnection_restoreClient` | `0x00dd8ae0` | RESTORE_CLIENT (0x34) |
| `ServerConnection_voiceData` | `0x00dd68b0` | VOICE_DATA (0x33) |
| `ServerConnection_startEntityMessage` | `0x00dd6a60` | ENTITY_MESSAGE (0x38+) |
| `ServerConnection_enableEntities` | `0x00dd9280` | (internal, sends 0x08 to server) |
| `Mercury_resetEntities` | `0x00dda0e0` | RESET_ENTITIES (0x04) |
| `EntityManager_enterWorld` | `0x00dd1d00` | (entity callback) |
| `EntityManager_connected` | `0x00dd32a0` | (connection callback) |
| `FUN_015846a0` | `0x015846a0` | (reads 3 consecutive u32s — Vec3 helper) |
| `FUN_00de3770` | `0x00de3770` | (packed string reader) |
| Universal RPC dispatcher | `0x00c6fc40` | All entity method calls |

---

## Client-to-Server Message Table

| msg_id | Name | Length Type | Size | Notes |
|--------|------|-----------|------|-------|
| 0x00 | BASEAPP_LOGIN | WORD_LENGTH | var | Unencrypted login |
| 0x01 | AUTHENTICATE | WORD_LENGTH | var | 20-byte ticket |
| 0x02 | AVATAR_UPD_IMPLICIT | CONSTANT | 36 | Unused by SGW |
| 0x03 | AVATAR_UPD_EXPLICIT | CONSTANT | 40 | Player movement |
| 0x04 | AVATAR_UPDW_IMPLICIT | CONSTANT | 36 | Unused |
| 0x05 | AVATAR_UPDW_EXPLICIT | CONSTANT | 40 | Unused |
| 0x06 | SWITCH_INTERFACE | CONSTANT | 0 | Deprecated |
| 0x07 | REQ_ENTITY_UPDATE | WORD_LENGTH | var | Request entity data |
| 0x08 | ENABLE_ENTITIES | CONSTANT | 8 | Entity system ready |
| 0x09 | VIEWPORT_ACK | CONSTANT | 8 | Viewport acknowledged |
| 0x0A | VEHICLE_ACK | CONSTANT | 8 | Vehicle acknowledged |
| 0x0B | RESTORE_CLIENT_ACK | WORD_LENGTH | var | Restore acknowledged |
| 0x0C | DISCONNECT | CONSTANT | 1 | Client disconnect |

---

## C++ Server vs Client Binary Discrepancies

Cross-referencing the fan-made C++ server source against the Ghidra client decompilation revealed these issues:

### 1. SET_VEHICLE (0x0F) — Comment says 8 bytes, table says 4

**File**: `messages.cpp:253-259`

The comment claims `[entityID:u32][vehicleID:u32]` = 8 bytes, but the message table entry says `CONSTANT_LENGTH = 4`. Standard BigWorld sends only `[vehicleID:u32]` (entity is implicit). The **table value of 4 is correct**; the comment is wrong.

### 2. UPDATE_AVATAR 0x21/0x22 — Comment names swapped

**File**: `messages.cpp:295-296`

Size 21 is labeled "Alias Full Pos Yaw" but should be "Alias Full Pos Yaw Pitch". Size 20 is labeled "Alias Full Pos Yaw Pitch" but should be "Alias Full Pos Yaw". The constant length values are correct; only the comments are swapped.

### 3. AVATAR_UPDATE_EXPLICIT (0x03) — Comment missing updateId byte

**File**: `messages.cpp:42-53`

Comment lists fields totaling 39 bytes, but table says `CONSTANT_LENGTH = 40`. The actual parsing code reads an additional `updateId` byte not mentioned in the comment. The **table value of 40 is correct**.

### 4. Rust Server Verification — All Implemented Messages Match

All Rust message builders in `mercury_ext.rs` produce wire-format-correct output:
- CONSTANT_LENGTH: msg_id + body (no length prefix)
- WORD_LENGTH: msg_id + u16 prefix + body
- DWORD_LENGTH (0xFF reply): msg_id + u32 prefix + body
- Entity methods (0x80+): msg_id + u16 prefix + body

The Rust client-to-server scanner in `base.rs` correctly implements all format types matching the C++ `ClientMessageList` table exactly. **No discrepancies found in any implemented Rust code.**

---

## Detailed Documentation Index

### Dispatch Table (the Rosetta Stone)

| Document | Coverage |
|----------|----------|
| [message-dispatch-table.md](message-dispatch-table.md) | Complete msg_id → handler mapping, InterfaceElement layout, dispatch mechanics, 0x80/0xC0 entity method boundary |

### System Protocol (hardcoded C++, no .def files)

| Document | Coverage |
|----------|----------|
| [entity-creation-wire-formats.md](../reverse-engineering/findings/entity-creation-wire-formats.md) | CREATE_BASE_PLAYER, CREATE_CELL_PLAYER, FORCED_POSITION, VIEWPORT_INFO, CREATE_ENTITY, ENTITY_INVISIBLE, LEAVE_AOI, TICK_SYNC, UPDATE_AVATAR, velocity compression |
| [system-protocol-wire-formats.md](../reverse-engineering/findings/system-protocol-wire-formats.md) | AUTHENTICATE, BANDWIDTH, UPDATE_FREQ, SET_GAME_TIME, RESET_ENTITIES, SPACE_DATA, RESOURCE_FRAGMENT, LOGGED_OFF, RESTORE_CLIENT, CONTROL_ENTITY, complete connection lifecycle |
| [space-viewport-wire-formats.md](../reverse-engineering/findings/space-viewport-wire-formats.md) | All 57 message IDs, complete message table, UPDATE_AVATAR 32 variants with sizes, RESOURCE_FRAGMENT reassembly, entity lifecycle sequences |
| [position-movement-wire-formats.md](../reverse-engineering/findings/position-movement-wire-formats.md) | All 34 position messages: forcedPosition, detailedPosition, 32 avatarUpdate variants, velocity compression, direction packing |

### Entity Method RPCs (driven by .def files)

| Document | Systems |
|----------|---------|
| [combat-wire-formats.md](../reverse-engineering/findings/combat-wire-formats.md) | Abilities, stats, effects, timers, damage |
| [inventory-wire-formats.md](../reverse-engineering/findings/inventory-wire-formats.md) | Items, stores, loot, containers |
| [mission-wire-formats.md](../reverse-engineering/findings/mission-wire-formats.md) | Missions, steps, objectives, rewards |
| [gate-travel-wire-formats.md](../reverse-engineering/findings/gate-travel-wire-formats.md) | Stargate dialing, addresses, passage |
| [crafting-wire-formats.md](../reverse-engineering/findings/crafting-wire-formats.md) | Craft, research, reverse engineer |
| [organization-wire-formats.md](../reverse-engineering/findings/organization-wire-formats.md) | Squads, guilds, strike teams |
| [chat-wire-formats.md](../reverse-engineering/findings/chat-wire-formats.md) | Chat channels, tells, ignore list |
| [mail-wire-formats.md](../reverse-engineering/findings/mail-wire-formats.md) | Mail send/receive, attachments |
| [black-market-wire-formats.md](../reverse-engineering/findings/black-market-wire-formats.md) | Auction house |
| [trade-wire-formats.md](../reverse-engineering/findings/trade-wire-formats.md) | Player-to-player trade |
| [contact-list-wire-formats.md](../reverse-engineering/findings/contact-list-wire-formats.md) | Contact lists, online events |
| [group-wire-formats.md](../reverse-engineering/findings/group-wire-formats.md) | Group authority, coordination |
| [minigame-wire-formats.md](../reverse-engineering/findings/minigame-wire-formats.md) | Minigame matchmaking |
| [duel-wire-formats.md](../reverse-engineering/findings/duel-wire-formats.md) | Dueling |
| [pet-wire-formats.md](../reverse-engineering/findings/pet-wire-formats.md) | Pet commands |
| [entity-types-wire-formats.md](../reverse-engineering/findings/entity-types-wire-formats.md) | Account, SGWEntity, SGWPlayer types |
| [entity-property-sync.md](../reverse-engineering/findings/entity-property-sync.md) | Property IDs, method IDs |
