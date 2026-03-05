# Space, Viewport, and Entity Lifecycle Wire Formats

> **Last updated**: 2026-03-05
> **RE Status**: Fully documented from Ghidra decompilation + C++ server source
> **Sources**: Ghidra analysis of `sgw.exe` client binary, `src/baseapp/mercury/sgw/messages.cpp`, `src/baseapp/mercury/sgw/client_handler.cpp`
> **Confidence**: HIGH

---

## Overview

This document covers the server-to-client messages for space/viewport management, entity lifecycle (create, move, leave, invisible), resource delivery, and miscellaneous control messages. These messages use the Mercury reliable or unreliable bundle protocol.

All messages below are **server-to-client**. The message ID is the first byte after the Mercury packet header. Length handling is either `CONSTANT_LENGTH` (no length prefix; the scanner knows the exact byte count) or `WORD_LENGTH` (a `uint16` length prefix follows the message ID).

---

## BANDWIDTH_NOTIFICATION (0x01)

**Handler**: `ClientMessageHandler<bandwidthNotificationArgs>` (RTTI at `0x01e52088`)
**Length type**: CONSTANT_LENGTH(4)
**Sent**: During connection setup, before entity system init. Not used by SGW (no bandwidth mutator).

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 4 | uint32 | bandwidth | Max bandwidth in bps |

**Server source** (`messages.cpp:134`): Single uint32 write.

---

## UPDATE_FREQUENCY_NOTIFICATION (0x02)

**Handler**: `ClientMessageHandler<updateFrequencyNotificationArgs>` (RTTI at `0x01e520e0`)
**Length type**: CONSTANT_LENGTH(1)
**Sent**: First message after connection setup. Sets the server tick resolution.

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 1 | uint8 | resolution | Ticks per second. Usually 10 (= 1000ms / tickRate). |

**Server source** (`client_handler.cpp:46-53`):
```cpp
uint8_t updateFreq = (1000 / CellManager::instance().tickRate());
bundle << updateFreq;
```

---

## SET_GAME_TIME (0x03)

**Handler**: `ClientMessageHandler<setGameTimeArgs>` (RTTI at `0x01e52138`)
**Length type**: CONSTANT_LENGTH(4)
**Sent**: During connection setup, immediately after TICK_SYNC.

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 4 | uint32 | gameTime | Current game time in ticks (resolution set by 0x02) |

**Server source** (`client_handler.cpp:61-63`):
```cpp
bundle << (uint32_t)ticks;
```

---

## RESET_ENTITIES (0x04)

**Handler**: `ClientMessageHandler<resetEntitiesArgs>` (RTTI at `0x01e52180`)
**Length type**: CONSTANT_LENGTH(1)
**Sent**: When tearing down the entity system (e.g., before switching characters).

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 1 | uint8 | keepBaseEntities | 0 = clear all, 1 = keep base entities (always 0 in practice) |

**Server source** (`client_handler.cpp:472-474`):
```cpp
bundle << (uint8_t)0;
```

---

## CREATE_BASE_PLAYER (0x05)

**Handler**: `ServerConnection_createBasePlayer` @ `0x00dddca0`
**Length type**: WORD_LENGTH
**Sent**: On ENABLE_ENTITIES (0x08) from client. Creates the base part of the player entity.

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 4 | uint32 | entityId | Player entity ID |
| 4 | 1 | uint8 | classId | Index into entity class table |
| 5 | 1 | uint8 | propertyCount | Number of Python properties to follow (always 0 for SGW) |
| 6 | var | ... | properties | Optional Python property data (not used by SGW) |

**Client decompilation** (`0x00dddca0`): Reads uint32 (entityId), then uint16 (classId + propertyCount as 2 bytes). Stores entityId at `this+0x16c`. If a buffered createCellPlayer message exists, replays it immediately.

**Server source** (`client_handler.cpp:596-600`):
```cpp
bundle << entity_->entityId();
bundle << (uint8_t)entity_->classDef()->index();
bundle << (uint8_t)0; // Python property count
```

---

## CREATE_CELL_PLAYER (0x06)

**Handler**: `ServerConnection_createCellPlayer` @ `0x00dda2e0`
**Length type**: WORD_LENGTH
**Sent**: After CREATE_BASE_PLAYER, provides cell (world) position data.

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 4 | uint32 | spaceId | World instance ID |
| 4 | 4 | uint32 | vehicleId | Vehicle entity ID (0 = none) |
| 8 | 4 | float | posX | Player X position |
| 12 | 4 | float | posY | Player Y position |
| 16 | 4 | float | posZ | Player Z position |
| 20 | 4 | float | rotX | Rotation X (pitch) |
| 24 | 4 | float | rotZ | Rotation Z (roll) -- **NOTE: Z before Y** |
| 28 | 4 | float | rotY | Rotation Y (yaw) -- **NOTE: Y/Z swapped** |

**Client decompilation** (`0x00dda2e0`): Reads instanceId(4), vehicleId(4), position Vec3(12), then calls `FUN_015846a0` which reads 3 more uint32s (the rotation). The client can buffer this message if it arrives before CREATE_BASE_PLAYER.

**Server source** (`client_handler.cpp:398-401`):
```cpp
bundle << spaceId << (uint32_t)0 <<
    x << y << z << rotX << rotZ << rotY;
```

**Critical**: Rotation is written as `rotX, rotZ, rotY` (Y and Z swapped compared to logical order). This matches the memory note about wire rotation order.

---

## SPACE_DATA (0x07)

**Handler**: `ServerConnection_spaceData` @ `0x00dda540`
**Length type**: WORD_LENGTH
**Sent**: Space metadata updates. Unused in newer SGW builds (replaced by `SGWPlayer.onClientMapLoad`).

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 4 | uint32 | spaceId | Space identifier |
| 4 | 8 | uint64 | spaceEntryId | Space entry ID (u32 + u32) |
| 12 | 2 | uint16 | key | Space data key |
| 14 | var | string | value | Space data value (remaining bytes) |

**Client decompilation** (`0x00dda540`): Reads uint32(4) for spaceId, uint64(8) for entryId, uint16(2) for key, then remaining bytes as a string value. Debug string: `"ServerConnection::spaceData: space %d key %d"`.

**Server source** (`messages.cpp:189-190`): WORD_LENGTH, documented as `SpaceID, SpaceEntryID, Key, Value`.

---

## SPACE_VIEWPORT_INFO (0x08)

**Handler**: `ServerConnection_spaceViewportInfo` @ `0x00dda6c0`
**Length type**: CONSTANT_LENGTH(13)
**Sent**: After CREATE_BASE_PLAYER, before CREATE_CELL_PLAYER. Establishes the viewport for the player's space.

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 4 | uint32 | entityId1 | Controlling entity ID (player entity) |
| 4 | 4 | uint32 | entityId2 | Viewport entity ID (same as entityId1) |
| 8 | 4 | uint32 | spaceId | Space identifier |
| 12 | 1 | uint8 | viewportId | Viewport index (always 0) |

**Operations**:
- **Open viewport**: Set all fields to non-zero values.
- **Close viewport**: Set entityId2=0, spaceId=0 (entityId1 and viewportId still present).
- **Update viewport**: Send with new spaceId (warns if space changes for existing viewport).

**Client decompilation** (`0x00dda6c0`): Reads the viewportId at offset 12 as a byte pointer (`param_1 + 3` where param_1 is a uint32 pointer, so byte 12). Manages a viewport table at `this+0xf84` with entries containing {entityId1, entityId2, spaceRef, isNew}. Also updates a space-to-viewport mapping at `this+0xf90` and a ref-count table at `this+0xf9c`.

**Server source** (`client_handler.cpp:390-393`):
```cpp
bundle << entityId << entityId << spaceId << (uint8_t)0;
```

---

## CREATE_ENTITY (0x09)

**Handler**: (via entity message dispatch)
**Length type**: WORD_LENGTH
**Sent**: When a new entity enters the player's Area of Interest.

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 4 | uint32 | entityId | Entity ID |
| 4 | 1 | uint8 | idAlias | ID alias (0xFF = no alias) |
| 5 | 1 | uint8 | classId | Entity class index |
| 6 | 1 | uint8 | unknown1 | Always 0 |
| 7 | 1 | uint8 | unknown2 | Always 0 |

**Server source** (`client_handler.cpp:498-499`):
```cpp
bundle << entityId << (uint8_t)0xff << classId << (uint8_t)0x00 << (uint8_t)0x00;
```

After CREATE_ENTITY, the server sends a position update via `moveEntity()` which uses UPDATE_AVATAR (0x10).

---

## UPDATE_ENTITY (0x0A)

**Handler**: (via entity property dispatch)
**Length type**: WORD_LENGTH
**Sent**: Updates Python property dictionary of an entity. Not used by SGW (no client-side Python).

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | var | ... | propertyData | Entity property update blob |

---

## ENTITY_INVISIBLE (0x0B)

**Handler**: `ClientMessageHandler<entityInvisibleArgs>` (RTTI at `0x01e52220`), Detail class RTTI at `0x01e91168`
**Length type**: CONSTANT_LENGTH(5)
**Sent**: Before an entity is destroyed from the client's perspective.

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 4 | uint32 | entityId | Entity being hidden |
| 4 | 1 | uint8 | idAlias | ID alias (0xFF = no alias) |

**Server source** (`client_handler.cpp:525-528`):
```cpp
bundle << entityId << (uint8_t)0xff;
```

Usually followed by LEAVE_AOI (0x0C) if the entity is being fully removed.

---

## LEAVE_AOI (0x0C)

**Handler**: `Detail_EntityLeaveMessage__vfunc_0` @ `0x01562110` (destructor); real handler at `FUN_015620b0`
**Length type**: WORD_LENGTH
**Sent**: Entity leaves client Area of Interest. Usually preceded by ENTITY_INVISIBLE (0x0B).

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 4 | uint32 | entityId | Entity leaving AoI |
| 4 | 4 | uint32 | cacheStamp0 | Cache stamp (always 0 in SGW) |

**Server source** (`client_handler.cpp:535-538`):
```cpp
bundle << entityId << (uint32_t)0;
```

The cacheStamp is part of BigWorld's entity caching system. Only stamp index 0 is sent.

---

## TICK_SYNC (0x0D)

**Handler**: `ClientMessageHandler<tickSyncArgs>` (RTTI at `0x01e52270`)
**Length type**: CONSTANT_LENGTH(8)
**Sent**: Every game tick (10 Hz default). Synchronizes game time.

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 4 | uint32 | gameTime | Current game tick counter |
| 4 | 4 | uint32 | tickRate | Tick interval in milliseconds (typically 100) |

**Server source** (`client_handler.cpp:486-488`):
```cpp
bundle << (uint32_t)time << (uint32_t)CellManager::instance().tickRate();
```

Can be sent on the unreliable channel if `unreliable_tick_sync = true`.

---

## SET_SPACE_VIEWPORT (0x0E)

**Handler**: `ClientMessageHandler<setSpaceViewportArgs>` (RTTI at `0x01e522b8`)
**Length type**: CONSTANT_LENGTH(1)
**Sent**: Switch the active viewport. Used when transitioning between spaces.

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 1 | uint8 | viewportId | Viewport index to activate (previously created via SPACE_VIEWPORT_INFO) |

**String references**: `"setSpaceViewport"` at `0x019d0a98`, `"setSpaceViewportAck"` at `0x019d093c`.

---

## SET_VEHICLE (0x0F)

**Handler**: `ClientMessageHandler<setVehicleArgs>` (RTTI at `0x01e52308`)
**Length type**: CONSTANT_LENGTH(4)
**Sent**: When an entity enters or exits a vehicle. Not supported by SGW.

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 4 | uint32 | entityId | Entity entering/exiting vehicle |

**Note**: The C++ server source (`messages.cpp:259`) documents this as 4 bytes total with `entityId` and `vehicleId`, but the constant length is 4, suggesting only one uint32 is read in the SGW implementation. The BigWorld reference may include both entity and vehicle IDs in different versions. The client has `"setVehicleAck"` string at `0x019d0950`, indicating an acknowledgement flow exists.

**String references**: `"setVehicle"` at `0x019d0aac`.

---

## UPDATE_AVATAR variants (0x10 - 0x2F)

**Length type**: CONSTANT_LENGTH (varies by variant)

32 position update message variants for entities in the player's AoI. The primary variant used by the Cimmeria server is:

### UPDATE_AVATAR_NO_ALIAS_FULL_POS_YAW_PITCH_ROLL (0x10)

**Length type**: CONSTANT_LENGTH(25)
**Sent**: For NPC/entity movement broadcasts within AoI.

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 4 | uint32 | entityId | Full entity ID (no alias) |
| 4 | 4 | float | posX | World X position |
| 8 | 4 | float | posY | World Y position |
| 12 | 4 | float | posZ | World Z position |
| 16 | 5 | packed | velocityXZ | PackedXYZ velocity (see below) |
| 21 | 1 | uint8 | flags | Movement flags (0x01 typical) |
| 22 | 1 | uint8 | yaw | Packed yaw: `rotation.y / 0.024543693` |
| 23 | 1 | uint8 | pitch | Packed pitch: `rotation.x / 0.024543693` |
| 24 | 1 | uint8 | roll | Packed roll: `rotation.z / 0.024543693` |

**Velocity packing** (`ClientHandler::packXYZ`): Velocity is compressed into 5 bytes (uint32 + uint8):
- Bits [22:12] of packed1: X magnitude (11 bits, sign in bit 23)
- Bits [10:0] of packed1: Z magnitude (11 bits, sign in bit 11)
- Bits [31:24] of packed1 + bits [6:0] of packed2: Y magnitude (15 bits, sign in bit 7 of packed2)

The packing adds 2.0 to the absolute value, extracts mantissa bits from the IEEE 754 representation, then packs sign+magnitude.

**Direction quantization**: Each angle is quantized to 256 steps over 2*pi radians. The constant `0.024543693` equals `2*pi/256`. To encode: `byte = angle / 0.024543693`. To decode: `angle = byte * 0.024543693`.

**Direction order in wire**: `yaw, pitch, roll` but encoded as `rotation.y, rotation.x, rotation.z`.

**Server source** (`client_handler.cpp:548-555`):
```cpp
bundle << entityId << position.x << position.y << position.z;
packXYZ(bundle, velocity);
bundle << (uint8_t)flags <<
    (uint8_t)(rotation.y / 0.024543693f) <<  // yaw
    (uint8_t)(rotation.x / 0.024543693f) <<  // pitch
    (uint8_t)(rotation.z / 0.024543693f);    // roll
```

Can be sent on the unreliable channel if `unreliable_movement_update = true`.

### All 32 Variant Sizes

| ID Range | Alias | Position | Direction variants (YPR/YP/Y/None) | Sizes |
|----------|-------|----------|-------------------------------------|-------|
| 0x10-0x13 | NoAlias (4B) | FullPos (12B) | +5B velocity + 1B flags + 3/2/1/0 | 25, 24, 23, 22 |
| 0x14-0x17 | NoAlias (4B) | OnChunk | +5B velocity + 1B flags + 3/2/1/0 | 25, 24, 23, 22 |
| 0x18-0x1B | NoAlias (4B) | OnGround | +5B velocity + 1B flags + 3/2/1/0 | 25, 24, 23, 22 |
| 0x1C-0x1F | NoAlias (4B) | NoPos (0B) | +5B velocity + 1B flags + 3/2/1/0 | 13, 12, 11, 10 |
| 0x20-0x23 | Alias (1B) | FullPos (12B) | +5B velocity + 1B flags + 3/2/1/0 | 22, 21, 20, 19 |
| 0x24-0x27 | Alias (1B) | OnChunk | +5B velocity + 1B flags + 3/2/1/0 | 22, 21, 20, 19 |
| 0x28-0x2B | Alias (1B) | OnGround | +5B velocity + 1B flags + 3/2/1/0 | 22, 21, 20, 19 |
| 0x2C-0x2F | Alias (1B) | NoPos (0B) | +5B velocity + 1B flags + 3/2/1/0 | 10, 9, 8, 7 |

---

## DETAILED_POSITION (0x30)

**Length type**: CONSTANT_LENGTH(41)
**Sent**: Full precision position, velocity, and direction update. Does NOT work on client-controlled entities.

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 4 | uint32 | entityId | Target entity |
| 4 | 4 | float | posX | Position X |
| 8 | 4 | float | posY | Position Y |
| 12 | 4 | float | posZ | Position Z |
| 16 | 4 | float | velX | Velocity X |
| 20 | 4 | float | velY | Velocity Y |
| 24 | 4 | float | velZ | Velocity Z |
| 28 | 4 | float | rotX | Rotation X (pitch) |
| 32 | 4 | float | rotY | Rotation Y (yaw) |
| 36 | 4 | float | rotZ | Rotation Z (roll) |
| 40 | 1 | uint8 | flags | Movement flags (0x01) |

---

## FORCED_POSITION (0x31)

**Handler**: `ServerConnection_forcedPosition` @ `0x00dd9ee0`
**Length type**: CONSTANT_LENGTH(49)
**Sent**: Server correction of client-controlled entity position. Overrides client prediction.

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 4 | uint32 | entityId | Entity being corrected |
| 4 | 4 | uint32 | spaceId | Space identifier |
| 8 | 4 | uint32 | vehicleId | Vehicle ID (0 = none) |
| 12 | 4 | float | posX | Position X |
| 16 | 4 | float | posY | Position Y |
| 20 | 4 | float | posZ | Position Z |
| 24 | 4 | float | velX | Velocity X |
| 28 | 4 | float | velY | Velocity Y |
| 32 | 4 | float | velZ | Velocity Z |
| 36 | 4 | float | rotX | Rotation X (pitch) |
| 40 | 4 | float | rotZ | Rotation Z (roll) -- **NOTE: Z before Y** |
| 44 | 4 | float | rotY | Rotation Y (yaw) -- **NOTE: Y/Z swapped** |
| 48 | 1 | uint8 | flags | Physics mode (0x01) |

**Client decompilation** (`0x00dd9ee0`): Calls `ServerConnection_addMove` with arguments mapped from param_1 offsets. The rotation order is confirmed by the argument mapping: `param_1[9]` = rotX, `param_1[10]` = rotZ, `param_1[11]` = rotY (swapped).

**Server source** (`client_handler.cpp:407-413` - createCellPlayer path):
```cpp
bundle << entityId << spaceId << (uint32_t)0 <<
    x << y << z <<
    0.0f << 0.0f << 0.0f <<
    rotX << rotZ << rotY <<
    (uint8_t)0x01;
```

**Server source** (`client_handler.cpp:566-572` - forcedPosition path):
```cpp
bundle << entityId << spaceId << (uint32_t)0 <<
    position.x << position.y << position.z <<
    velocity.x << velocity.y << velocity.z <<
    rotation.x << rotation.y << rotation.z <<
    flags;
```

**Important discrepancy**: The `createCellPlayer` path writes `rotX, rotZ, rotY` (swapped), while the standalone `forcedPosition` path writes `rotation.x, rotation.y, rotation.z` (in order). The client handler reads them at fixed offsets. The `createCellPlayer` path is the one confirmed to match the client's expected order from pcap analysis (Y/Z swapped in wire).

---

## CONTROL_ENTITY (0x32)

**Handler**: `ClientMessageHandler<controlEntityArgs>` (RTTI at `0x01e53000`)
**Length type**: CONSTANT_LENGTH(5)
**Sent**: Tells the client whether an entity is controlled locally. Untested in SGW.

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 4 | uint32 | entityId | Entity whose control state changes |
| 4 | 1 | uint8 | controlled | 1 = locally controlled, 0 = server controlled |

**String reference**: `"controlEntity"` at `0x019d0f24`.

---

## VOICE_DATA (0x33)

**Handler**: `ServerConnection_voiceData` @ `0x00dd68b0`
**Length type**: WORD_LENGTH
**Sent**: Voice chat data. No handler registered for SGW.

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | var | bytes | voiceData | Raw voice data |

**Client decompilation** (`0x00dd68b0`): Simply forwards two parameters to the handler callback at `this+0x168 vtable[0x34]`. If no handler is set, logs warning: `"Got voice data before a handler has been set."`.

---

## RESTORE_CLIENT (0x34)

**Handler**: `ServerConnection_restoreClient` @ `0x00dd8ae0`
**Length type**: WORD_LENGTH
**Sent**: Server wants to restore client to a previous state. Untested.

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 4 | uint32 | entityId | Entity ID |
| 4 | 4 | uint32 | spaceId | Space identifier |
| 8 | 4 | uint32 | vehicleId | Vehicle entity ID |
| 12 | 12 | Vec3 | position | 3x float (X, Y, Z) |
| 24 | 12 | Vec3 | velocity | 3x float (X, Y, Z) |
| 36 | 12 | Vec3 | direction | 3x float (X, Y, Z) |

**Client decompilation** (`0x00dd8ae0`): Reads entityId(4), spaceId(4), vehicleId(4), then two Vec3 reads (position via `FUN_015846a0` and velocity remaining). After processing, sends a RESTORE_CLIENT_ACK back.

---

## RESTORE_BASEAPP (0x35)

**Length type**: WORD_LENGTH
**Sent**: BaseApp restoration notification. Untested.

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | var | bytes | data | Restoration data |

---

## RESOURCE_FRAGMENT (0x36)

**Handler**: `ServerConnection_resourceFragment` @ `0x00dddd80`
**Length type**: WORD_LENGTH
**Sent**: Delivers cooked data resources (items, abilities, missions, etc.) in fragments.

### Header (always present)

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 2 | uint16 | dataId | Unique per-resource transfer ID (increments per sendResource call) |
| 2 | 1 | uint8 | chunkId | Fragment sequence number (0, 1, 2, ...) |
| 3 | 1 | uint8 | flags | Bitfield (see below) |

### Flags byte

| Bit | Mask | Name | Description |
|-----|------|------|-------------|
| 0 | 0x01 | INITIAL_FRAGMENT | First fragment of a resource |
| 1 | 0x02 | FINAL_FRAGMENT | Last fragment of a resource |
| 6 | 0x40 | BASE_FLAG | Always set (purpose unknown, possibly "base app origin") |
| 7 | 0x80 | ERROR_FLAG | Resource error (client sets status 0xFF) |

### Body (first fragment only, after flags)

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 4 | 1 | uint8 | messageType | Always 0 (MESSAGE_CacheData) |
| 5 | 4 | uint32 | categoryId | Resource category index (see table below) |
| 9 | 4 | uint32 | elementId | Resource identifier (e.g., item def ID) |
| 13 | var | bytes | xmlBody | Start of XML document bytes |

### Body (subsequent fragments)

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 4 | var | bytes | xmlBody | Continuation of XML document bytes |

**Fragment size**: Maximum 1000 bytes per fragment body (`FragmentSize` constant).

**Reassembly**: The client maintains a linked list of fragments per `dataId`. When `FINAL_FRAGMENT` is received, all fragments are concatenated in reverse order (last received first) and delivered to the resource handler via `this+0x168 vtable[0x38]`.

**Client decompilation** (`0x00dddd80`): Two code paths based on `BASE_FLAG (0x40)`:
1. **With BASE_FLAG**: Fragment reassembly path. Allocates fragment nodes (11 + bodySize bytes each), chains them in sequence order, triggers reassembly when expected count matches received count.
2. **Without BASE_FLAG**: Direct delivery path. Uses semaphore-based synchronization. Writes fragment data to a FILE handle. Releases semaphore on completion.

### Resource Category IDs

| Index | Category Name | Description |
|-------|--------------|-------------|
| 0 | *(reserved)* | |
| 1 | `kismet_event_sequence` | Kismet event sequences |
| 2 | `ability` | Combat abilities |
| 3 | `mission` | Quest definitions |
| 4 | `item` | Item definitions |
| 5 | `dialog` | Dialog trees |
| 6 | `kismet_event_set` | Kismet event sets |
| 7 | `char_creation` | Character creation data |
| 8 | `interaction_set_map` | Interaction menus |
| 9 | `effect` | Effects |
| 10 | `text` | Localized text |
| 11 | `error_text` | Error strings |
| 12 | `world_info` | Zone data |
| 13 | `stargate` | Stargates |
| 14 | `container` | Inventory containers |
| 15 | `blueprint` | Crafting blueprints |
| 16 | `applied_science` | Applied sciences |
| 17 | `discipline` | Crafting disciplines |
| 18 | `racial_paradigm` | Racial paradigms |
| 19 | `special_words` | Chat filter words |
| 20 | `interaction` | Interactions |

---

## LOGGED_OFF (0x37)

**Handler**: `ServerConnection_loggedOff` @ `0x00dd8c20`
**Length type**: CONSTANT_LENGTH(1)
**Sent**: Server forcibly disconnects the client.

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 1 | uint8 | reason | Disconnect reason code |

**Client decompilation** (`0x00dd8c20`): Logs `"The server has disconnected us. reason = %d"` and calls the disconnect handler.

**Server source** (`client_handler.cpp:461-463`):
```cpp
bundle << (uint8_t)0;
```

---

## ENTITY_MESSAGE (0x38+)

**Handler**: `ServerConnection_startEntityMessage` @ `0x00dd6a60` (for entity method calls)
**Length type**: WORD_LENGTH (for messages >= 0x80)
**Sent**: Entity RPC method calls from server to client.

For entity messages, the msg_id has bit 7 set (0x80). The format is:

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | 4 | uint32 | entityId | Target entity |
| 4 | var | bytes | methodArgs | Serialized method arguments |

The msg_id byte itself encodes the method index with 0x80 OR'd in.

**Client decompilation** (`0x00dd6a60`): Sets the message ID with `0x80` OR'd in, writes the entity ID as a uint32 payload.

---

## REPLY_MESSAGE (0xFF)

**Length type**: WORD_LENGTH
**Sent**: Reply to BaseApp connection requests (Mercury-level, not application-level).

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0 | var | bytes | replyData | Connection reply payload |

This is a Mercury protocol-level message used during the initial connection handshake, not during normal gameplay.

---

## Complete Server Message Table

For reference, the complete message ID to format mapping:

| ID | Name | Length Type | Size | Notes |
|----|------|------------|------|-------|
| 0x00 | AUTHENTICATE | DWORD_LENGTH | var | Never observed |
| 0x01 | BANDWIDTH_NOTIFICATION | CONSTANT | 4 | Not used by SGW |
| 0x02 | UPDATE_FREQUENCY_NOTIFICATION | CONSTANT | 1 | |
| 0x03 | SET_GAME_TIME | CONSTANT | 4 | |
| 0x04 | RESET_ENTITIES | CONSTANT | 1 | |
| 0x05 | CREATE_BASE_PLAYER | WORD | var | |
| 0x06 | CREATE_CELL_PLAYER | WORD | var | |
| 0x07 | SPACE_DATA | WORD | var | Unused |
| 0x08 | SPACE_VIEWPORT_INFO | CONSTANT | 13 | |
| 0x09 | CREATE_ENTITY | WORD | var | |
| 0x0A | UPDATE_ENTITY | WORD | var | Not used by SGW |
| 0x0B | ENTITY_INVISIBLE | CONSTANT | 5 | |
| 0x0C | LEAVE_AOI | WORD | var | |
| 0x0D | TICK_SYNC | CONSTANT | 8 | |
| 0x0E | SET_SPACE_VIEWPORT | CONSTANT | 1 | |
| 0x0F | SET_VEHICLE | CONSTANT | 4 | Untested |
| 0x10-0x2F | UPDATE_AVATAR (32 variants) | CONSTANT | 7-25 | See table above |
| 0x30 | DETAILED_POSITION | CONSTANT | 41 | |
| 0x31 | FORCED_POSITION | CONSTANT | 49 | |
| 0x32 | CONTROL_ENTITY | CONSTANT | 5 | Untested |
| 0x33 | VOICE_DATA | WORD | var | Unused |
| 0x34 | RESTORE_CLIENT | WORD | var | Untested |
| 0x35 | RESTORE_BASEAPP | WORD | var | Untested |
| 0x36 | RESOURCE_FRAGMENT | WORD | var | |
| 0x37 | LOGGED_OFF | CONSTANT | 1 | |
| 0x38 | CLIENT_MESSAGE (entity RPC) | WORD | var | |
| 0xFF | REPLY_MESSAGE | WORD | var | Mercury handshake |

---

## Typical Message Sequences

### World Entry (after character select)

```
Server -> Client: RESET_ENTITIES(0x04)              [flush]
Client -> Server: ENABLE_ENTITIES(0x08)
Server -> Client: CREATE_BASE_PLAYER(0x05)           entityId, classId=Account, props=0
Server -> Client: SPACE_VIEWPORT_INFO(0x08)          entityId, entityId, spaceId, viewportId=0
Server -> Client: CREATE_CELL_PLAYER(0x06)           spaceId, vehicleId=0, pos, rot(X,Z,Y)
Server -> Client: FORCED_POSITION(0x31)              entityId, spaceId, vehicleId=0, pos, vel=0, rot(X,Z,Y), flags=1
```

### Entity Enter AoI

```
Server -> Client: CREATE_ENTITY(0x09)                entityId, alias=0xFF, classId, 0, 0
Server -> Client: UPDATE_AVATAR(0x10)                entityId, pos, packedVel, flags, yaw, pitch, roll
Server -> Client: entity RPC: onVisible(0x08)        entityId, bool=true  (SGW-specific)
```

### Entity Leave AoI

```
Server -> Client: ENTITY_INVISIBLE(0x0B)             entityId, alias=0xFF
Server -> Client: LEAVE_AOI(0x0C)                    entityId, cacheStamp=0
```

### Resource Delivery (single fragment, small resource)

```
Server -> Client: RESOURCE_FRAGMENT(0x36)            dataId, chunkId=0, flags=0x43, msgType=0, catId, elemId, xmlBody
```
(flags 0x43 = BASE_FLAG | INITIAL_FRAGMENT | FINAL_FRAGMENT)

### Resource Delivery (multi-fragment)

```
Server -> Client: RESOURCE_FRAGMENT(0x36)            dataId, chunkId=0, flags=0x41, msgType=0, catId, elemId, xmlBody[0..999]
Server -> Client: RESOURCE_FRAGMENT(0x36)            dataId, chunkId=1, flags=0x40, xmlBody[1000..1999]
Server -> Client: RESOURCE_FRAGMENT(0x36)            dataId, chunkId=2, flags=0x42, xmlBody[2000..end]
```
(flags: 0x41=BASE+INITIAL, 0x40=BASE only, 0x42=BASE+FINAL)

---

## Related Documents

- [Login Handshake](../../protocol/login-handshake.md) -- Connection setup
- [Position Updates](../../protocol/position-updates.md) -- Movement details, filter architecture
- [Entity Property Sync](entity-property-sync.md) -- Property sync wire formats
- [Entity Types](entity-types-wire-formats.md) -- Account, SGWEntity types
- [Cooked Data Pipeline](../../engine/cooked-data-pipeline.md) -- Resource delivery details
- [Space Management](../../engine/space-management.md) -- AoI, grid, viewport system
