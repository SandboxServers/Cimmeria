# Post-Authentication Entity Creation Sequence

Complete ordered sequence of messages the BaseApp must send after a successful `baseAppLogin`, with wire formats. Derived from analysis of BigWorld 1.9.1/2.0.1 source code cross-referenced against Cimmeria's current implementation and SGW client reverse engineering.

## Key Type Definitions

From BigWorld `basictypes.hpp`, with SGW modifications noted:

| Type | C++ Type | Size | Notes |
|------|----------|------|-------|
| `EntityID` | `int32` | 4 bytes | Signed |
| `SpaceID` | `int32` | 4 bytes | Signed |
| `EntityTypeID` | `uint16` (BW) / `uint8` (SGW) | 2/1 byte | SGW uses 1-byte class ID |
| `SessionKey` | `uint32` | 4 bytes | |
| `TimeStamp` | `uint32` | 4 bytes | Game tick count |
| `Position3D` | 3 x `float` | 12 bytes | x, y, z |
| `Direction3D` | 3 x `float` | 12 bytes | roll, pitch, yaw |
| `IDAlias` | `uint8` | 1 byte | 0xFF = no alias |

## Phase 0: Channel Establishment (baseAppLogin)

Already implemented in `connect_handler.cpp`.

### Client Sends: `baseAppLogin` (message 0x00 in BaseAppExtInterface)

```
Wire format (client -> server):
  uint32    sessionKey      (from LoginApp reply)
  int32     attempt         (attempt number, 0-based)
```

### Server Replies (Mercury request reply)

```
Standard BigWorld:
  uint32    sessionKey      (new session key for this proxy)

SGW (our implementation):
  uint8     ticketLength
  char[]    ticket          (echoed back to client)
```

### Client Behavior

`BaseAppLoginRequest::onSuccess()` extracts the sessionKey, calls `LoginHandler::onBaseAppReply()` which sets `ServerConnection::sessionKey_` and transfers the channel. The client's `logOnComplete()` then calls `primeBundle()` which starts including `authenticate` messages on every outgoing bundle, and calls `enableEntities()`.

## Phase 1: Initial Server Messages

Sent immediately after channel establishment, before `enableEntities` arrives from the client. These three messages are sent as a group and **flushed immediately**.

### Message 1: `updateFrequencyNotification` (ID 0x02)

Tells the client how frequently the server sends updates.

```
Wire format:
  uint8     hertz           (typically 10)
```

Total: 1 byte constant length.

Client behavior: Sets `s_updateFrequency_` for time synchronization math.

### Message 2: `tickSync` (ID 0x0D) -- SGW CUSTOM FORMAT

```
Standard BigWorld format:
  uint8     tickByte        (1 byte sequence counter)

SGW format:
  uint32    gameTime        (current tick count)
  uint32    tickRate        (ms per tick, e.g. 100)
```

Total: 8 bytes (SGW) vs 1 byte (standard BW).

### Message 3: `setGameTime` (ID 0x03)

Sets absolute game time baseline. Sent with FLUSH flag.

```
Wire format:
  uint32    gameTime        (TimeStamp)
```

Total: 4 bytes constant length.

**CRITICAL**: After this message, the bundle is flushed (`channel_->flushBundle(false)`). These three messages go in their own packet, BEFORE the entity creation flow. The comment in our code says: "We need to flush here, otherwise the client will crash when processing CREATE_BASE_ENTITY".

## Phase 2: Internal Entity Creation

After flushing Phase 1, the server creates the Account entity internally via Python:

```cpp
BaseEntityManager<BaseEntity>::instance().create("Account", accountId_)
entity_->setupClient(channel_)
```

The server then **waits** for the client to send `enableEntities`.

## Phase 3: Client Sends `enableEntities`

```
Standard BigWorld:
  uint8     dummy           (always 0)

SGW (may differ):
  uint64    dummy           (8 bytes)
```

Message ID 0x08 in BaseAppExtInterface. This is the client's signal that it is ready to receive entity data.

## Phase 4: createBasePlayer (ID 0x05)

Sent in response to `enableEntities`. Creates the base part of the player entity on the client.

### Standard BigWorld Wire Format (variable length)

```
  EntityID     playerID        (int32, 4 bytes)
  EntityTypeID playerType      (uint16, 2 bytes)
  [base+client property data]  (variable, serialized properties)
```

### SGW Wire Format (our code)

```
  uint32    entityID            (4 bytes)
  uint8     classID             (1 byte -- index into entity type table)
  uint8     propertyCount       (1 byte -- currently always 0)
```

Total: 6 bytes.

### Client Behavior

From `ServerConnection::createBasePlayer` (servconn.cpp line 2731):

1. Reads `playerID` (EntityID) and sets `id_ = playerID`
2. Reads `playerType` (EntityTypeID)
3. Calls `pHandler_->onBasePlayerCreate(id_, playerType, stream)` which creates the base player entity
4. If a `createCellPlayer` was already buffered (arrived before createBasePlayer), replays it now

**IMPORTANT**: In standard BW, `EntityTypeID` is `uint16`. SGW uses `uint8` for the class ID. This is a CME modification.

## Phase 5: Cell Player Creation

After `createBasePlayer`, the server calls `entity_->attachedToController(this)` which invokes the Python Account script. The script initiates space loading, which triggers the following three messages:

### Message 5: `spaceViewportInfo` (ID 0x08) -- SGW CUSTOM

**Does not exist in standard BigWorld.** This is a CME addition for SGW's multi-space viewport system, likely needed for Stargate zone transitions.

```
Wire format:
  uint32    entityID        (player's entity ID)
  uint32    entityID2       (same as entityID; possibly viewport entity)
  uint32    spaceID         (space identifier)
  uint8     viewportID      (always 0 for primary viewport)
```

Total: 13 bytes constant length.

**MUST arrive before any `spaceData` for that space.** Client error check: `"ServerConnection::spaceData: Received space data before any space viewport info"`.

### Message 6: `createCellPlayer` (ID 0x06)

Creates the cell part of the player entity. Places the player in the world with position and rotation.

### Standard BigWorld Wire Format (variable length)

```
  SpaceID     spaceID         (int32, 4 bytes)
  EntityID    vehicleID       (int32, 4 bytes)
  Position3D  position        (3 x float, 12 bytes)
  Direction3D direction       (3 x float, 12 bytes) -- roll, pitch, yaw
  [cell+client property data] (variable, serialized properties)
```

### SGW Wire Format (our code)

```
  uint32    spaceID             (4 bytes)
  uint32    vehicleID           (4 bytes, always 0)
  float     x, y, z             (12 bytes -- position)
  float     rotX, rotZ, rotY    (12 bytes -- NOTE: Z and Y swapped!)
```

Total: 32 bytes.

### Client Behavior

From `ServerConnection::createCellPlayer` (servconn.cpp line 2766):

1. If `id_ == 0` (no base player yet), **buffers** the entire message for later replay
2. Otherwise: reads spaceID, vehicleID, pos, dir
3. Adds player ID to `controlledEntities_`
4. Calls `pHandler_->onCellPlayerCreate(...)` with remaining stream as cell properties
5. Sets channel to regular mode (`isIrregular(false)` in 1.9.1, `isLocalRegular(true)` and `isRemoteRegular(true)` in 2.0.1)

### Message 7: `forcedPosition` (ID 0x31)

Sets the initial authoritative position for the client-controlled player entity.

### Standard BigWorld Wire Format (36 bytes)

```
  EntityID    id              (int32, 4 bytes)
  SpaceID     spaceID         (int32, 4 bytes)
  EntityID    vehicleID       (int32, 4 bytes)
  Position3D  position        (12 bytes)
  Direction3D direction       (12 bytes)
```

### SGW Wire Format (49 bytes -- adds velocity + flags)

```
  uint32    entityID            (4 bytes)
  uint32    spaceID             (4 bytes)
  uint32    vehicleID           (4 bytes, always 0)
  float     x, y, z             (12 bytes -- position)
  float     velX, velY, velZ    (12 bytes -- velocity, all 0.0f)
  float     rotX, rotZ, rotY    (12 bytes -- rotation, Z/Y swapped)
  uint8     flags               (1 byte, 0x01)
```

## Phase 6: Space Data (Optional)

### `spaceData` (ID 0x07)

```
Standard BigWorld wire format (variable length):
  SpaceID      spaceID         (int32, 4 bytes)
  SpaceEntryID spaceEntryID    (8 bytes -- ip:4 + port:2 + salt:2)
  uint16       key             (2 bytes)
  char[]       value           (remaining bytes -- the data string)
```

Key 0 = geometry/map path. Key 1 = weather data.

**SGW note**: Per code comment: "Only used in older builds, newer versions use SGWPlayer.onClientMapLoad". SGW may use RPC-based map loading instead of the standard spaceData mechanism.

## Complete Sequence Diagram

```
Client                                          BaseApp Server
  |                                                   |
  |--- baseAppLogin (sessionKey, attempt) ----------->|
  |<-- reply (ticket echo) ----------------------------|  Phase 0
  |                                                   |
  |   (channel established, encryption active)        |
  |                                                   |
  |<-- updateFrequencyNotification (hertz=10) --------|
  |<-- tickSync (gameTime, tickRate) ------[SGW]------|  Phase 1
  |<-- setGameTime (gameTime) ----[FLUSH]-------------|
  |                                                   |
  |   (server creates Account entity via Python)      |  Phase 2
  |                                                   |
  |--- authenticate (sessionKey) --[on every bundle]->|
  |--- enableEntities (dummy=0) --------------------->|  Phase 3
  |                                                   |
  |<-- createBasePlayer (entityID, classID, 0) -------|  Phase 4
  |                                                   |
  |   (Python script runs, initiates space loading)   |
  |                                                   |
  |<-- spaceViewportInfo (eID, eID, spaceID, 0) [SGW]-|
  |<-- createCellPlayer (spaceID, 0, pos, rot) -------|  Phase 5
  |<-- forcedPosition (eID, spaceID, 0, pos, vel, rot)|
  |                                                   |
  |<-- [optional: spaceData / resource fragments] ----|  Phase 6
  |<-- [optional: createEntity for nearby NPCs] ------|
  |<-- [regular tickSync at 10Hz] --------------------|
  |                                                   |
  |--- avatarUpdateExplicit (pos, vel, dir) --------->|  Game loop
```

## Critical Ordering Rules

1. **Phase 1 messages MUST be flushed** before Phase 4 begins. Our code does this with `channel_->flushBundle(false)` after `setGameTime`.

2. **`createBasePlayer` MUST arrive before `createCellPlayer`**. BigWorld 1.9.1 has a buffer for out-of-order delivery; 2.0.1 asserts. SGW client likely follows 1.9.1 behavior, but we should always send them in order.

3. **`spaceViewportInfo` MUST arrive before any `spaceData`** for that space. The SGW client checks this and logs an error if violated.

4. **`enableEntities` is client-initiated**. The server must NOT send `createBasePlayer` until it receives `enableEntities` from the client.

5. **`forcedPosition` should follow `createCellPlayer`** to set the initial authoritative position.

6. The channel starts as **irregular** (no regular send timer) until `createCellPlayer` is received by the client, at which point it becomes **regular**.

## SGW vs Standard BigWorld Differences

| Aspect | Standard BigWorld 1.9.1 | SGW (Cimmeria) |
|--------|------------------------|----------------|
| `tickSync` format | 1 byte (`uint8 tickByte`) | 8 bytes (`uint32 time + uint32 tickRate`) |
| `EntityTypeID` in createBasePlayer | `uint16` (2 bytes) | `uint8` (1 byte) as `classID` |
| `spaceViewportInfo` | Does not exist | 13-byte message (ID 0x08), must precede spaceData |
| `entityInvisible` | Does not exist | 5-byte message (ID 0x0B), sent before leaveAoI |
| `forcedPosition` | 36 bytes (no velocity) | 49 bytes (adds velocity + flags) |
| `enableEntities` payload | `uint8` (1 byte) | Possibly `uint64` (8 bytes) |
| Auth reply | `uint32 sessionKey` | Ticket echo (variable length) |
| `createCellPlayer` properties | Variable-length property stream | Fixed 32 bytes, no trailing properties |

## Reference Sources

- BigWorld 1.9.1: `bigworld/src/common/servconn.cpp` (client-side receiver)
- BigWorld 2.0.1: `src/lib/connection/server_connection.cpp` (refactored)
- BigWorld 1.9.1: `bigworld/src/server/baseapp/proxy.hpp` (server-side proxy)
- BigWorld 1.9.1/2.0.1: `client_interface.hpp`, `baseapp_ext_interface.hpp`
- Cimmeria: `src/baseapp/mercury/sgw/connect_handler.cpp`, `client_handler.cpp`, `messages.cpp`
- SGW binary strings: `docs/bigworld-version-analysis.md` (ServerConnection debug strings)
