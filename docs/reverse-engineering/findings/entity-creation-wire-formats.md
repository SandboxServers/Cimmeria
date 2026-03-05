# Entity Creation Wire Formats

> **Date**: 2026-03-05
> **Phase**: 5 — World Entry RE
> **Confidence**: HIGH (cross-verified: Ghidra client binary + C++ server source + pcap captures)
> **Sources**: Client binary (`ServerConnection_createBasePlayer`, `ServerConnection_createCellPlayer`, `ServerConnection_forcedPosition`, `ServerConnection_spaceViewportInfo`), server source (`messages.hpp`, `messages.cpp`, `client_handler.cpp`, `cell_handler.cpp`)

---

## Overview

This document covers the server-to-client wire formats for entity creation and world entry messages. These messages are sent during two phases:

1. **Phase 4 (Login/Character Select)**: `CREATE_BASE_PLAYER` for the Account entity + character list method call
2. **Phase 5 (World Entry)**: `RESET_ENTITIES` -> `CREATE_BASE_PLAYER` for SGWPlayer -> `SPACE_VIEWPORT_INFO` -> `CREATE_CELL_PLAYER` -> `FORCED_POSITION`

All multi-byte integers are **little-endian**. All floats are **IEEE 754 single-precision (32-bit)**, little-endian.

---

## Message Table Reference

From `messages.hpp` / `messages.cpp` (server source, confirmed against client binary):

| msg_id | Name | Length Type | Constant Size | Notes |
|--------|------|------------|---------------|-------|
| 0x04 | RESET_ENTITIES | CONSTANT_LENGTH | 1 | Tear down entity system |
| 0x05 | CREATE_BASE_PLAYER | WORD_LENGTH | - | Create base entity on client |
| 0x06 | CREATE_CELL_PLAYER | WORD_LENGTH | - | Create cell entity with position |
| 0x08 | SPACE_VIEWPORT_INFO | CONSTANT_LENGTH | 13 | Set up viewport for space |
| 0x09 | CREATE_ENTITY | WORD_LENGTH | - | Create ghost entity (AoI) |
| 0x0D | TICK_SYNC | CONSTANT_LENGTH | 8 | Heartbeat / time sync |
| 0x31 | FORCED_POSITION | CONSTANT_LENGTH | 49 | Authoritative position set |

**Length types**:
- `CONSTANT_LENGTH`: No length prefix; the scanner knows the exact size from the table.
- `WORD_LENGTH`: `u16 LE` length prefix immediately after the msg_id byte; payload follows.

---

## 1. RESET_ENTITIES (0x04)

**Length type**: CONSTANT_LENGTH = 1
**Purpose**: Tears down the client entity system. Sent before switching entities (e.g., from Account to SGWPlayer). The client responds with `ENABLE_ENTITIES` (0x08) when ready.

**C++ source**: `client_handler.cpp:472` — `bundle.beginMessage(BASEMSG_RESET_ENTITIES, Bundle::FLUSH); bundle << (uint8_t)0;`

**Critical**: Must be sent in its own flushed bundle (separate packet), not combined with other messages. The C++ server explicitly flushes before writing RESET_ENTITIES to a new bundle.

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 1 | u8 | keepBase | `0x00` = destroy all entities; `0x01` = keep base player (unused by SGW) |

**Total wire size**: 1 byte (no length prefix)

---

## 2. CREATE_BASE_PLAYER (0x05)

**Handler**: `ServerConnection_createBasePlayer` @ `0x00dddca0`
**Length type**: WORD_LENGTH (u16 prefix)
**Purpose**: Creates the base part of the player entity on the client. Sent twice during a session: once for the Account entity (Phase 4), once for SGWPlayer (Phase 5).

### Client Decompilation Analysis

From `ServerConnection_createBasePlayer`:
```
read(4)  -> entityId (u32) — stored at this+0x16c
read(2)  -> typeId (u16)   — entity class index
callback(entityId, typeId, stream) — passes remaining stream to entity handler
```

The function calls `(**(code **)(*param_1 + 4))(4)` to read 4 bytes (entityId), then `(**(code **)(*param_1 + 4))(2)` to read 2 bytes (typeId). The typeId is used as a `u16` but only the low byte matters (the high byte is 0x00 = propertyCount).

Debug string: `"ServerConnection::createBasePlayer: id %d\n"` at `0x019d00e0`.

### C++ Server Source

```cpp
// client_handler.cpp:596-600
bundle.beginMessage(BASEMSG_CREATE_BASE_PLAYER);
bundle << entity_->entityId();                    // u32
bundle << (uint8_t)entity_->classDef()->index();  // u8 classId
bundle << (uint8_t)0;                             // u8 propertyCount
bundle.endMessage();
```

### Wire Format

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 2 | u16 | wordLength | Byte count of payload (always 6) |
| 2 | 4 | u32 | entityId | Entity ID allocated by EntityManager |
| 6 | 1 | u8 | classId | Entity type index (0x07 = Account, 0x02 = SGWPlayer) |
| 7 | 1 | u8 | propertyCount | Number of serialized Python properties (always 0) |

**Total wire size**: 1 (msg_id) + 2 (wordLength) + 6 (payload) = **9 bytes on wire**

**Note on typeId reading**: The client reads 2 bytes for typeId. The first byte is classId and the second byte is propertyCount. The client internally uses both, but since propertyCount is always 0 for SGW, no additional property data follows.

### Entity Class IDs

From entity definitions (`entities.xml` ordering):

| classId | Entity Type | Usage |
|---------|------------|-------|
| 0x00 | SGWEntity | Base game entity |
| 0x01 | SGWSpawnableEntity | Visual entity |
| 0x02 | SGWPlayer | Player character (Phase 5 world entry) |
| 0x03 | SGWBeing | Combat-capable entity |
| 0x04 | SGWMob | NPC |
| 0x05 | SGWPet | Player pet |
| 0x06 | SGWDuelMarker | Duel zone marker |
| 0x07 | Account | Login/character management (Phase 4) |

### Buffering Behavior

The client's `createBasePlayer` handler checks for a buffered `createCellPlayer` message (at `this+0xfe0`). If one was received before the base player was created, it plays it back immediately:

```
Debug: "ServerConnection::createBasePlayer: Playing buffered createCellPlayer message\n"
```

This means `CREATE_CELL_PLAYER` can arrive before `CREATE_BASE_PLAYER` and the client handles it gracefully by buffering. However, the server should send them in the correct order.

---

## 3. CREATE_CELL_PLAYER (0x06)

**Handler**: `ServerConnection_createCellPlayer` @ `0x00dda2e0`
**Length type**: WORD_LENGTH (u16 prefix)
**Purpose**: Creates the cell (spatial/positional) part of the player entity on the client. Sent during world entry (Phase 5b) after `CREATE_BASE_PLAYER`.

### Client Decompilation Analysis

From `ServerConnection_createCellPlayer`:
```
check: if this+0x16c == 0, buffer the message (base player not yet created)
read(4) -> (discarded)      — spaceId? (consumed but result unused in local struct init)
read(4) -> vehicleId (u32)  — stored, used for entity chain
read(12) -> position (3xf32) — via FUN_015846a0 which reads 3 consecutive u32s
FUN_015846a0(stream) -> reads 3 additional floats into a separate buffer (rotation)
```

At `0x00dda3cc-0x00dda416` in the disassembly:
```asm
PUSH 0x4       ; read 4 bytes
CALL EDX       ; -> spaceId (consumed, result discarded from decompiler's view)
PUSH 0x4       ; read 4 bytes
CALL EAX       ; -> vehicleId (stored in EBP)
PUSH 0xc       ; read 12 bytes
CALL EAX       ; -> position[3] (8 bytes via MOVQ + 4 bytes via MOV)
```

Then `FUN_015846a0` is called which reads 3 more u32s from the stream — this is the rotation.

After reading, at `0x00dda4d0-0x00dda50a`, the handler calls the callback with:
```
callback(entityId, vehicleId(?), spaceId(?), &position, rot[2], rot[1], rot[0], stream)
```

Debug strings:
- `"ServerConnection::createCellPlayer: id %d\n"` at `0x019d024c`
- `"ServerConnection::createCellPlayer: Got createCellPlayer before createBasePlayer. Buffering message\n"` at `0x019d0160`
- `"ASSERTION FAILED: createCellPlayerMsg_.remainingLength() == 0\n"` — verifies buffer was empty before storing

### C++ Server Source

```cpp
// client_handler.cpp:398-401
bundle.beginMessage(BASEMSG_CREATE_CELL_PLAYER);
bundle << spaceId << (uint32_t)0 <<
    x << y << z << rotX << rotZ << rotY;
bundle.endMessage();
```

**CRITICAL**: The rotation order on the wire is `rotX, rotZ, rotY` — the Y and Z components are SWAPPED relative to their logical meaning. This matches the `entity->createCellPlayer(spaceId, x, y, z, yaw, roll, pitch)` call in `cell_handler.cpp:510` where `roll` and `pitch` are swapped in the parameter list.

### Wire Format

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 2 | u16 | wordLength | Byte count of payload (always 32) |
| 2 | 4 | u32 | spaceId | World space identifier (e.g., 65552 for CombatSim) |
| 6 | 4 | u32 | vehicleId | Vehicle entity ID (0 = not in vehicle) |
| 10 | 4 | f32 | posX | World position X |
| 14 | 4 | f32 | posY | World position Y |
| 18 | 4 | f32 | posZ | World position Z |
| 22 | 4 | f32 | rotX | Rotation X (pitch) in radians |
| 26 | 4 | f32 | rotZ | Rotation Z (roll) — **SWAPPED with Y** |
| 30 | 4 | f32 | rotY | Rotation Y (yaw) — **SWAPPED with Z** |

**Total wire size**: 1 (msg_id) + 2 (wordLength) + 32 (payload) = **35 bytes on wire**

### Rotation Swap Explanation

The wire order is `X, Z, Y` (not the logical `X, Y, Z`). This is a deliberate BigWorld convention visible in the C++ source:
- `client_handler.cpp:400`: `<< rotX << rotZ << rotY`
- `client_handler.cpp:411`: `<< rotX << rotZ << rotY` (in forcedPosition during createCellPlayer)

The client expects this order and stores the values accordingly.

---

## 4. SPACE_VIEWPORT_INFO (0x08)

**Handler**: `ServerConnection_spaceViewportInfo` @ `0x00dda6c0`
**Length type**: CONSTANT_LENGTH = 13
**Purpose**: Sets up a viewport linking the player entity to a world space. Must be sent before `CREATE_CELL_PLAYER` so the client knows which space the cell entity belongs to.

### Client Decompilation Analysis

From `ServerConnection_spaceViewportInfo`, the struct accessed at param_1 (pre-parsed by Mercury):
```
[EDI+0x00] = field0 (u32) — gate/unknown (stored at puVar5+0)
[EDI+0x04] = entityId (u32) — stored at puVar5+4; compared with this+0x16c
[EDI+0x08] = spaceId (u32) — stored at puVar5+8 (EBP)
[EDI+0x0C] = viewportId (u8) — used as byte index (EBX)
```

Debug string: `"ServerConnection::spaceViewportInfo: space %d svid %d\n"` at `0x019d02fc`.

The handler manages a viewport table and associates entity IDs with space IDs. When `entityId2 == 0`, it closes the viewport. When nonzero, it opens/updates the viewport.

### C++ Server Source

```cpp
// client_handler.cpp:391-393
bundle.beginMessage(BASEMSG_SPACE_VIEWPORT_INFO);
bundle << entityId << entityId << spaceId << (uint8_t)0;
bundle.endMessage();
```

Both entityId fields are the same value (the player entity ID).

### Wire Format

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 4 | u32 | entityId | Player entity ID (controlling entity) |
| 4 | 4 | u32 | entityId2 | Same as entityId (viewport entity); 0 = close viewport |
| 8 | 4 | u32 | spaceId | World space ID; 0 = close viewport |
| 12 | 1 | u8 | viewportId | Viewport index (always 0 for SGW) |

**Total wire size**: 13 bytes (no length prefix, constant-length message)

### Viewport Operations

- **Open viewport**: All fields nonzero. Creates mapping: viewportId -> (entityId, spaceId).
- **Close viewport**: entityId2 = 0, spaceId = 0. Removes the mapping.
- **Re-use viewport**: Updating an existing viewport with different spaceId triggers a warning: `"ServerConnection::spaceViewportInfo: Server wants us to re-use space viewport %d changing space from %d to %d!\n"`

---

## 5. FORCED_POSITION (0x31)

**Handler**: `ServerConnection_forcedPosition` @ `0x00dd9ee0`
**Length type**: CONSTANT_LENGTH = 49
**Purpose**: Authoritatively sets position, velocity, rotation, and physics mode for a client-controlled entity. Cannot be used for non-controlled (ghost) entities. Sent during world entry and when the server needs to teleport the player.

### Client Decompilation Analysis

From `ServerConnection_forcedPosition`, the struct is pre-parsed by Mercury (CONSTANT_LENGTH = 49). The handler accesses it via direct struct offsets:

```asm
[ESI+0x00] = entityId    (u32)  — first param to addMove
[ESI+0x04] = spaceId     (u32)
[ESI+0x08] = vehicleId   (u32)
[ESI+0x0C] = posX        (f32)  — start of position Vec3 (param_4 to addMove)
[ESI+0x10] = posY        (f32)
[ESI+0x14] = posZ        (f32)
[ESI+0x18] = velX        (f32)  — start of velocity Vec3 (param_5 to addMove)
[ESI+0x1C] = velY        (f32)
[ESI+0x20] = velZ        (f32)
[ESI+0x24] = rotX        (f32)  — yaw/pitch/roll as full floats
[ESI+0x28] = rotY        (f32)  — note: NOT swapped here (unlike createCellPlayer)
[ESI+0x2C] = rotZ        (f32)
[ESI+0x30] = physics     (u8)   — physics/movement mode flags
```

The total is 4+4+4+12+12+12+1 = **49 bytes**, matching CONSTANT_LENGTH.

Debug string: `"ServerConnection::forcedPosition: Received forced position for entity %d that we do not control\n"` at `0x019cfe88`.

The handler calls `ServerConnection_addMove` (at `0x00dd9330`) with all extracted fields, then updates the physics mode in the `sentPhysics_` map with an assertion: `"ASSERTION FAILED: sentPhysics_[ args.id ] == args.physics"`.

### C++ Server Source

There are two call sites:

**1. During createCellPlayer (world entry):**
```cpp
// client_handler.cpp:407-413
bundle.beginMessage(BASEMSG_FORCED_POSITION);
bundle << entityId << spaceId << (uint32_t)0 <<
    x << y << z <<
    0.0f << 0.0f << 0.0f <<
    rotX << rotZ << rotY <<     // <-- SWAPPED rotation
    (uint8_t)0x01;
bundle.endMessage();
```

**2. Standalone forcedPosition method:**
```cpp
// client_handler.cpp:566-572
bundle.beginMessage(BASEMSG_FORCED_POSITION);
bundle << entityId << spaceId << (uint32_t)0 <<
    position.x << position.y << position.z <<
    velocity.x << velocity.y << velocity.z <<
    rotation.x << rotation.y << rotation.z <<  // <-- NOT swapped (caller's responsibility)
    flags;
bundle.endMessage();
```

### Wire Format

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 4 | u32 | entityId | Controlled entity ID |
| 4 | 4 | u32 | spaceId | World space ID |
| 8 | 4 | u32 | vehicleId | Vehicle entity ID (0 = not in vehicle) |
| 12 | 4 | f32 | posX | World position X |
| 16 | 4 | f32 | posY | World position Y |
| 20 | 4 | f32 | posZ | World position Z |
| 24 | 4 | f32 | velX | Velocity X (0 during world entry) |
| 28 | 4 | f32 | velY | Velocity Y (0 during world entry) |
| 32 | 4 | f32 | velZ | Velocity Z (0 during world entry) |
| 36 | 4 | f32 | rotX | Rotation X |
| 40 | 4 | f32 | rotY | Rotation Y — **may be swapped depending on call site** |
| 44 | 4 | f32 | rotZ | Rotation Z — **may be swapped depending on call site** |
| 48 | 1 | u8 | flags | Physics mode (0x01 = standard) |

**Total wire size**: 49 bytes (no length prefix, constant-length message)

### Rotation Swap Note for FORCED_POSITION

During world entry (`createCellPlayer` call site), the C++ server writes `rotX, rotZ, rotY` (swapped). The standalone `forcedPosition()` method receives rotation already in wire order from the caller. The client reads the fields positionally -- it always interprets offset 36 as rotX, offset 40 as the next rotation component, offset 44 as the third.

The key insight from `ServerConnection_addMove` at `0x00dd9330` is that the callback at `this+0x168` receives the rotation in the order `(param_6, param_7, param_8)` = offsets `(0x24, 0x28, 0x2C)` = `(rotX, rotY, rotZ)` as stored in the struct. The C++ writes `rotX, rotZ, rotY` to the wire, so the client reads them as `rotX=rotX, rotY=rotZ_from_server, rotZ=rotY_from_server`. This is the BigWorld Y/Z swap convention.

---

## 6. CREATE_ENTITY (0x09)

**Length type**: WORD_LENGTH (u16 prefix)
**Purpose**: Creates a ghost (non-player) entity that entered the client's Area of Interest. Used for NPCs, other players, objects, etc.

### C++ Server Source

```cpp
// client_handler.cpp:497-499
bundle.beginMessage(BASEMSG_CREATE_ENTITY);
bundle << entityId << (uint8_t)0xff << classId << (uint8_t)0x00 << (uint8_t)0x00;
bundle.endMessage();
```

After creating the entity, the server immediately sends a position update via `moveEntity()`:
```cpp
// client_handler.cpp:503-505
Vec3 velocity;
moveEntity(entityId, position, rotation, velocity, 0x01);
```

### Wire Format

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 2 | u16 | wordLength | Byte count of payload (always 5) |
| 2 | 4 | u32 | entityId | Entity ID |
| 6 | 1 | u8 | idAlias | ID alias for compression (0xFF = no alias) |
| 7 | 1 | u8 | classId | Entity type index (from entities.xml) |
| 8 | 1 | u8 | unknown1 | Always 0x00 |
| 9 | 1 | u8 | unknown2 | Always 0x00 |

**Total wire size**: 1 (msg_id) + 2 (wordLength) + 5 (payload) = **8 bytes on wire**

### Ghost Entity Position

Ghost entities receive their position via `UPDATE_AVATAR_NO_ALIAS_FULL_POS_YAW_PITCH_ROLL` (0x10) immediately after creation:

```cpp
// client_handler.cpp:548-556
bundle.beginMessage(BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YAW_PITCH_ROLL);
bundle << entityId << position.x << position.y << position.z;
packXYZ(bundle, velocity);      // 5 bytes: u32 packed1 + u8 packed2
bundle << (uint8_t)flags <<
    (uint8_t)(rotation.y / 0.024543693f) <<  // yaw
    (uint8_t)(rotation.x / 0.024543693f) <<  // pitch
    (uint8_t)(rotation.z / 0.024543693f);    // roll
bundle.endMessage();
```

---

## 7. UPDATE_AVATAR_NO_ALIAS_FULL_POS_YAW_PITCH_ROLL (0x10)

**Length type**: CONSTANT_LENGTH = 25
**Purpose**: Movement update for ghost (non-player) entities. Does NOT work on client-controlled entities (use FORCED_POSITION for those).

### Wire Format

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 4 | u32 | entityId | Entity whose position is updated |
| 4 | 4 | f32 | posX | World position X |
| 8 | 4 | f32 | posY | World position Y |
| 12 | 4 | f32 | posZ | World position Z |
| 16 | 5 | packed | velocityXYZ | Compressed velocity (see below) |
| 21 | 1 | u8 | flags | Movement flags (0x01 = standard) |
| 22 | 1 | u8 | yaw | `(u8)(rotation.y / 0.024543693)` — 0-255 maps to 0-2pi rad |
| 23 | 1 | u8 | pitch | `(u8)(rotation.x / 0.024543693)` — 0-255 maps to 0-2pi rad |
| 24 | 1 | u8 | roll | `(u8)(rotation.z / 0.024543693)` — 0-255 maps to 0-2pi rad |

**Rotation constant**: `0.024543693 = 2*PI/256`. Each byte encodes a full rotation in 256 steps.

**Total wire size**: 25 bytes (no length prefix, constant-length message)

### Velocity Compression (`packXYZ`)

The velocity vector is packed into 5 bytes using a custom format (from `client_handler.cpp:647-686`):

```
Byte 0-3 (u32 packed1):
  Bits 31-24: Y delta high byte
  Bit  23:    X sign (1 = negative)
  Bits 22-12: X mantissa (11 bits from IEEE float bits 14-4)
  Bit  11:    Z sign (1 = negative)
  Bits 10-0:  Z mantissa (11 bits from IEEE float bits 26-16)

Byte 4 (u8 packed2):
  Bit  7:     Y sign (1 = negative)
  Bits 6-0:   Y delta low 7 bits
```

The encoding adds 2.0f to each absolute component before extracting mantissa bits, providing a bias that avoids zero-encoding issues. To decode, reverse the bit extraction and subtract the 2.0f bias.

---

## 8. TICK_SYNC (0x0D)

**Length type**: CONSTANT_LENGTH = 8
**Purpose**: Heartbeat sent at 10 Hz (every 100ms). Keeps the client's game clock synchronized and the connection alive. Also triggers client-side tick processing.

### C++ Server Source

```cpp
// client_handler.cpp:486-488
bundle.beginMessage(BASEMSG_TICK_SYNC);
bundle << (uint32_t)time << (uint32_t)CellManager::instance().tickRate();
bundle.endMessage();
```

### Wire Format

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 4 | u32 | gameTime | Current game tick counter (increments each tick) |
| 4 | 4 | u32 | tickRate | Tick interval in milliseconds (100 = 10 Hz) |

**Total wire size**: 8 bytes (no length prefix, constant-length message)

---

## 9. ENTITY_INVISIBLE (0x0B)

**Length type**: CONSTANT_LENGTH = 5
**Purpose**: Marks an entity as invisible before removing it from the client. Always sent before LEAVE_AOI.

### C++ Server Source

```cpp
// client_handler.cpp:525-528
bundle.beginMessage(BASEMSG_ENTITY_INVISIBLE);
bundle << entityId << (uint8_t)0xff;
bundle.endMessage();
```

### Wire Format

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 4 | u32 | entityId | Entity to mark invisible |
| 4 | 1 | u8 | idAlias | ID alias (0xFF = no alias) |

**Total wire size**: 5 bytes

---

## 10. LEAVE_AOI (0x0C)

**Length type**: WORD_LENGTH (u16 prefix)
**Purpose**: Removes an entity from the client's Area of Interest. Sent after ENTITY_INVISIBLE.

### C++ Server Source

```cpp
// client_handler.cpp:535-538
bundle.beginMessage(BASEMSG_LEAVE_AOI);
bundle << entityId << (uint32_t)0;  // CacheStamp 0
```

### Wire Format

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 2 | u16 | wordLength | Byte count of payload (8) |
| 2 | 4 | u32 | entityId | Entity to remove |
| 6 | 4 | u32 | cacheStamp | Cache version stamp (always 0) |

**Total wire size**: 1 (msg_id) + 2 (wordLength) + 8 (payload) = **11 bytes on wire**

---

## Complete World Entry Sequence (Phase 5)

The full server-to-client sequence for world entry, as implemented in the C++ server:

### Phase 5a: Entity System Reset

Triggered by `ClientHandler::switchEntity()` -> `disconnectEntity(false)`:

```
1. (flush current bundle)
2. RESET_ENTITIES (0x04) — keepBase=0
   (flush — must be its own packet)
```

The client tears down all entities, then sends `ENABLE_ENTITIES` (0x08).

### Phase 5b: World Entry

Triggered by `ClientHandler::enableEntities()` on receiving `ENABLE_ENTITIES`:

```
3. CREATE_BASE_PLAYER (0x05) — entityId=playerEntityId, classId=0x02 (SGWPlayer), propCount=0
   (calls entity->attachedToController(), then requests cell data)
```

The BaseApp asks the CellApp for position data via `sendConnectEntity`. The CellApp responds with `CELL_BASE_CREATE_CELL_PLAYER`, which triggers `ClientHandler::createCellPlayer()`:

```
4. SPACE_VIEWPORT_INFO (0x08) — entityId, entityId, spaceId, viewportId=0
5. CREATE_CELL_PLAYER (0x06) — spaceId, vehicleId=0, pos(x,y,z), rot(X,Z,Y)
6. FORCED_POSITION (0x31) — entityId, spaceId, vehicleId=0, pos(x,y,z), vel(0,0,0), rot(X,Z,Y), flags=0x01
```

Messages 4-6 are written to the same bundle and flushed together.

### Example Wire Dump (Phase 5b, single packet body)

```
05                      ; msg_id: CREATE_BASE_PLAYER
06 00                   ; wordLength: 6
XX XX XX XX             ; entityId (u32 LE)
02                      ; classId: SGWPlayer
00                      ; propertyCount: 0

08                      ; msg_id: SPACE_VIEWPORT_INFO (constant=13)
XX XX XX XX             ; entityId
XX XX XX XX             ; entityId2 (same)
YY YY YY YY            ; spaceId
00                      ; viewportId

06                      ; msg_id: CREATE_CELL_PLAYER
20 00                   ; wordLength: 32
YY YY YY YY            ; spaceId
00 00 00 00             ; vehicleId
PP PP PP PP             ; posX (f32)
PP PP PP PP             ; posY (f32)
PP PP PP PP             ; posZ (f32)
RR RR RR RR            ; rotX (f32)
RR RR RR RR            ; rotZ (f32) — SWAPPED
RR RR RR RR            ; rotY (f32) — SWAPPED

31                      ; msg_id: FORCED_POSITION (constant=49)
XX XX XX XX             ; entityId
YY YY YY YY            ; spaceId
00 00 00 00             ; vehicleId
PP PP PP PP             ; posX
PP PP PP PP             ; posY
PP PP PP PP             ; posZ
00 00 00 00             ; velX (0.0)
00 00 00 00             ; velY (0.0)
00 00 00 00             ; velZ (0.0)
RR RR RR RR            ; rotX
RR RR RR RR            ; rotZ — SWAPPED
RR RR RR RR            ; rotY — SWAPPED
01                      ; flags (0x01)
```

---

## Supplementary Messages

### SPACE_DATA (0x07) — WORD_LENGTH

Not used by SGW in modern builds. The server comment says: "Only used in older builds, newer versions use SGWPlayer.onClientMapLoad."

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 2 | u16 | wordLength | Payload length |
| 2 | 4 | u32 | spaceId | Space identifier |
| 6 | 8 | u64 | spaceEntryId | Space entry ID (two u32s packed) |
| 14 | 2 | u16 | key | Data key |
| 16 | N | string | value | Data value (remaining bytes) |

### LOGGED_OFF (0x37) — CONSTANT_LENGTH = 1

Sent to terminate the session.

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 1 | u8 | reason | 0 = normal logoff |

### CONTROL_ENTITY (0x32) — CONSTANT_LENGTH = 5

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 4 | u32 | entityId | Entity to control/release |
| 4 | 1 | u8 | controlled | 1 = locally controlled, 0 = released |

---

## Cross-Reference: Client Binary Addresses

| Function | Address | Message |
|----------|---------|---------|
| `ServerConnection_createBasePlayer` | `0x00dddca0` | CREATE_BASE_PLAYER (0x05) |
| `ServerConnection_createCellPlayer` | `0x00dda2e0` | CREATE_CELL_PLAYER (0x06) |
| `ServerConnection_spaceViewportInfo` | `0x00dda6c0` | SPACE_VIEWPORT_INFO (0x08) |
| `ServerConnection_forcedPosition` | `0x00dd9ee0` | FORCED_POSITION (0x31) |
| `ServerConnection_addMove` | `0x00dd9330` | (internal movement handler) |
| `ServerConnection_spaceData` | `0x00dda540` | SPACE_DATA (0x07) |
| `EntityManager_enterWorld` | `0x00dd1d00` | (entity enter world callback) |
| `EntityManager_connected` | `0x00dd32a0` | (connection callback) |
| `FUN_015846a0` | `0x015846a0` | (reads 3 consecutive u32s from stream) |

### Key Debug Strings

| Address | String |
|---------|--------|
| `0x019d00e0` | `"ServerConnection::createBasePlayer: id %d\n"` |
| `0x019d0110` | `"ServerConnection::createBasePlayer: Playing buffered createCellPlayer message\n"` |
| `0x019d0160` | `"ServerConnection::createCellPlayer: Got createCellPlayer before createBasePlayer. Buffering message\n"` |
| `0x019d024c` | `"ServerConnection::createCellPlayer: id %d\n"` |
| `0x019d02fc` | `"ServerConnection::spaceViewportInfo: space %d svid %d\n"` |
| `0x019cfe88` | `"ServerConnection::forcedPosition: Received forced position for entity %d that we do not control\n"` |
| `0x019cfef8` | `"ASSERTION FAILED: sentPhysics_[ args.id ] == args.physics\n"` |
