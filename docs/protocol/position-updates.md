# Position Update Protocol

> **Last updated**: 2026-03-01
> **RE Status**: Substantially documented from BigWorld source + Cimmeria movement code + Ghidra RE of sgw.exe
> **Sources**: `external/engines/BigWorld-Engine-2.0.1/src/lib/connection/common_client_interface.hpp`, `src/cellapp/entity/movement.hpp`, `src/mercury/base_cell_messages.hpp`, Ghidra analysis of `sgw.exe` client binary

---

## Overview

Position updates are the most frequently sent messages in the game. BigWorld uses an optimized system with multiple message variants that trade precision for bandwidth. The Cimmeria server has its own movement controller system that processes client updates and broadcasts them to other players via the AoI system.

## Client-to-Server: Movement Updates

### BigWorld avatarUpdate Messages

BigWorld defines 32 variants of the `avatarUpdate` message by combining three parameters:

| Parameter | Options | Description |
|-----------|---------|-------------|
| ID type | `NoAlias`, `Alias` | Full EntityID (4 bytes) or alias (1 byte) |
| Position | `FullPos`, `OnChunk`, `OnGround`, `NoPos` | Position encoding |
| Direction | `YawPitchRoll`, `YawPitch`, `Yaw`, `NoDir` | Direction encoding |

This creates a 2 x 4 x 4 = 32 message matrix. Only the variant with sufficient precision for the current update is sent, minimizing bandwidth.

### Position Encodings

**UPDATED (2026-03-05)**: Ghidra RE confirmed that FullPos, OnChunk, and OnGround all use the SAME wire size (3 x float32 = 12 bytes). The difference is in handler interpretation: OnChunk and OnGround ignore the Y float and derive it from terrain/chunk height. See [Position Movement Wire Formats](../reverse-engineering/findings/position-movement-wire-formats.md) for field-level details.

| Type | Format | Size | Description |
|------|--------|------|-------------|
| `FullPos` | 3 x float32 | 12 bytes | Full 3D position (X, Y, Z all used) |
| `OnChunk` | 3 x float32 | 12 bytes | X, Z used; Y IGNORED (client uses chunk height) |
| `OnGround` | 3 x float32 | 12 bytes | X, Z used; Y IGNORED (client uses terrain height) |
| `NoPos` | (none) | 0 bytes | No position change |

### Direction Encodings

| Type | Format | Size | Description |
|------|--------|------|-------------|
| `YawPitchRoll` | 3 compressed angles | 3 bytes | Full 3D rotation |
| `YawPitch` | 2 compressed angles | 2 bytes | Yaw + pitch only |
| `Yaw` | 1 compressed angle | 1 byte | Yaw only (most common) |
| `NoDir` | (none) | 0 bytes | No direction change |

### Relative Position Reference

BigWorld uses a reference position to send subsequent positions as deltas:

```
relativePositionReference:
  uint8    sequenceNumber    -- Which client-sent position to use as reference

relativePosition:
  Vector3  position          -- New reference position (absolute)
```

This allows bandwidth savings when entities move small distances.

### All avatarUpdate Messages

All messages are `UNRELIABLE` -- lost position updates are acceptable because new ones arrive frequently.

```
avatarUpdateNoAliasFullPosYawPitchRoll    (largest: ~16 bytes)
avatarUpdateNoAliasFullPosYawPitch
avatarUpdateNoAliasFullPosYaw
avatarUpdateNoAliasFullPosNoDir
avatarUpdateNoAliasOnChunkYawPitchRoll
...
avatarUpdateAliasNoPosNoDir               (smallest: ~1 byte)
```

## Server-to-Client: Entity Position Updates

### Cimmeria Movement Broadcast

The CellApp detects movement and notifies the BaseApp, which forwards to clients. From `base_cell_messages.hpp`:

```
CELL_BASE_MOVE (0x09):
  uint32    Space ID
  uint32    Entity ID
  Vec3      Location     (3 x float = 12 bytes)
  Vec3      Rotation     (3 x float = 12 bytes)
  Vec3      Velocity     (3 x float = 12 bytes)
  uint8     Flags
  uint8     ServerFlags
```

The BaseApp translates this into client-facing entity update messages.

### Forced Position

Used to correct client prediction or set initial position. See [Position Movement Wire Formats](../reverse-engineering/findings/position-movement-wire-formats.md) for complete field-level wire format.

```
forcedPosition (msg_id 0x31, CONSTANT_LENGTH = 49 bytes):
  int32     entityID       (4 bytes)
  int32     spaceID        (4 bytes)
  int32     vehicleID      (4 bytes)
  float32   posX           (4 bytes)
  float32   posY           (4 bytes)
  float32   posZ           (4 bytes)
  float32   velX           (4 bytes)
  float32   velY           (4 bytes)
  float32   velZ           (4 bytes)
  float32   roll           (4 bytes)
  float32   pitch          (4 bytes)
  float32   yaw            (4 bytes)
  uint8     physics        (1 byte)
```

This overrides any client-side prediction and forces the entity to the specified location.

## Movement Controllers

Cimmeria implements three movement controller types (from `src/cellapp/entity/movement.hpp`):

### PlayerController

Handles player movement from client input:
- Receives `playerUpdate()` calls with position, rotation, velocity, and flags
- Processes updates each server tick
- Broadcasts position changes to AoI witnesses

### WaypointController

Handles NPC pathfinding movement:
- Uses Recast/Detour navmesh for pathfinding
- Moves along waypoints with navigation paths
- Recalculates rotation to face movement direction
- Triggers callbacks when waypoints are reached

### DebugController

Used for testing entity movement:
- Simple movement pattern for development purposes

### Movement Tick Results

Each controller tick returns a result indicating the type of movement:

| Result | Value | Description |
|--------|-------|-------------|
| `NoMovement` | 0 | Entity did not move |
| `LinearMovement` | 1 | Predictable movement (velocity unchanged) |
| `NonlinearMovement` | 2 | Direction or velocity changed |
| `ForcedMovement` | 3 | Server overrides client position |

These results determine what updates to broadcast:
- `NoMovement`: No update sent (unless idle update timer fires)
- `LinearMovement`: Clients can predict, no update needed unless `idle_update_ticks` elapsed
- `NonlinearMovement`: Full update sent to all AoI witnesses
- `ForcedMovement`: Forced position sent to owning client

## Volatile Properties

BigWorld's `VolatileInfo` system (from `volatile_info.hpp`) controls how frequently position and direction are sent for non-player entities:

```xml
<!-- In entity .def file -->
<Volatile>
    <position>2.0</position>    <!-- priority for position updates -->
    <yaw>2.0</yaw>              <!-- priority for yaw updates -->
    <pitch>1.0</pitch>          <!-- priority for pitch updates -->
    <roll>0</roll>              <!-- 0 = never send roll -->
</Volatile>
```

Higher priority values mean more frequent updates. A value of 0 means never sent. The `ALWAYS` constant means every tick.

SGW's `SGWPlayer.def` comments out the `<Volatile/>` element, suggesting players use default volatile behavior (always send position and yaw).

## Configuration

From `config/BaseService.config`:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `unreliable_movement_update` | false | Send movement on unreliable channel |
| `tick_rate` | 100 ms | Server tick interval (10 Hz) |
| `idle_update_ticks` | 10 | Ticks between idle movement broadcasts |
| `grid_chunk_size` | 50 | World grid chunk size (meters) |
| `grid_vision_distance` | 3 | Visibility range in chunks |

Setting `unreliable_movement_update = true` reduces reliable channel congestion at the cost of occasional lost updates. For position data, this is usually acceptable because updates are frequent.

## AoI Integration

Position updates trigger the AoI (Area of Interest) system:

1. Entity moves to new position
2. `WorldGrid::move()` checks if entity crossed a chunk boundary
3. If crossed, scan for entering/leaving entities based on `grid_vision_distance`
4. Hysteresis (`grid_hysteresis = 1` chunk) prevents rapid enter/leave oscillation
5. Enter AoI -> send `CELL_BASE_ENTER_AOI` with initial position
6. Leave AoI -> send `CELL_BASE_LEAVE_AOI`
7. Movement within AoI -> send `CELL_BASE_MOVE`

See [Space Management](../engine/space-management.md) for details on the grid system.

## Ghidra RE: avatarUpdate Variants Used by SGW (sgw.exe)

### All 32 Standard Variants Are Registered

The SGW client registers all 32 standard BigWorld `avatarUpdate` message handlers. Each has a `ClientMessageHandler<>` RTTI type and a corresponding string name in the binary:

```
Addresses 019d0ab8-019d0ee4: All 32 message name strings
Addresses 01e52350-01e52f08: All 32 RTTI ClientMessageHandler<> type descriptors

NoAlias variants (16):  avatarUpdateNoAlias{FullPos,OnChunk,OnGround,NoPos}{YawPitchRoll,YawPitch,Yaw,NoDir}
Alias variants (16):    avatarUpdateAlias{FullPos,OnChunk,OnGround,NoPos}{YawPitchRoll,YawPitch,Yaw,NoDir}
```

### SGW-Specific Update Messages (Client-to-Server)

In addition to the 32 standard receive-side handlers, `ServerConnection::addMove` (at `0x00dd9330`) sends position updates using **4 SGW-specific message types**, selected based on two criteria:

| Condition | Implicit variant | Explicit variant |
|-----------|-----------------|------------------|
| Own entity (`entityID == playerID`) | `avatarUpdateImplicit` | `avatarUpdateExplicit` |
| Other entity (ward/vehicle) | `avatarUpdateWardImplicit` | `avatarUpdateWardExplicit` |

**Implicit** = no change in space/vehicle/physics mode (smaller message, omits spaceID and physics flag).
**Explicit** = space, vehicle, or physics mode changed (includes spaceID and physics flag byte).

Message selection logic in `addMove` (0x00dd9330):
```
if (entityID == playerID):
    if (no change needed):
        send avatarUpdateImplicit    (msg ID at DAT_01ef24e8, starter at 0x00de2a90)
    else:
        send avatarUpdateExplicit    (msg ID at DAT_01ef24ec, starter at 0x00de2ae0)
else:
    if (no change needed):
        send avatarUpdateWardImplicit  (msg ID at DAT_01ef24f0, starter at 0x00de2b30)
    else:
        send avatarUpdateWardExplicit  (msg ID at DAT_01ef24f4, starter at 0x00de2b80)
```

### avatarUpdateImplicit Message Format

The implicit variant (most common) sends:
```
avatarUpdateImplicit:
  uint32   spaceViewportID    -- Viewport (implicit = same space)
  Vec3     position           (3 x float = 12 bytes)
  Vec3     velocity           (3 x float = 12 bytes)
  YPR      direction          (3 x int8 = 3 bytes, packed angles)
  4 bytes  auxiliary           (timestamp/sequence data)
```

### avatarUpdateExplicit Message Format

The explicit variant (sent on space/physics change) adds:
```
avatarUpdateExplicit:
  uint32   entityID
  uint32   spaceViewportID
  Vec3     position           (3 x float = 12 bytes)
  Vec3     velocity           (3 x float = 12 bytes)
  YPR      direction          (3 x int8 = 3 bytes, packed angles)
  uint8    physics             (physics/movement mode)
  4 bytes  auxiliary
```

### Server Implementation Note

The Cimmeria server does NOT need to implement all 32 receive-side variants. The client sends only the 4 SGW-specific types (`Implicit`, `Explicit`, `WardImplicit`, `WardExplicit`). The 32 standard handlers exist on the client to **receive** position updates from the server for other entities in the AoI. The server selects which of the 32 variants to send based on the BigWorld volatile info and bandwidth optimization logic.

## Ghidra RE: Packed Position and Direction Formats

### Direction Packing (Confirmed)

The function at `0x00de1720` packs three float angles (yaw, pitch, roll) into 3 bytes. The algorithm converts each angle to a single `int8` value:

```c
// Pseudocode reconstructed from FUN_00de1720 at 0x00de1720
void packDirection(uint8* out, float yaw, float pitch, float roll) {
    // yaw uses a different scale constant than pitch/roll
    // DAT_01851a30 ≈ 256/(2*pi) for yaw, DAT_01847294 ≈ 256/(2*pi) for pitch/roll
    // DAT_019002fc = scaling factor, DAT_01816a8c = rounding bias
    out[0] = (int8)(yaw   * SCALE_YAW   * INV_2PI + 0.5);   // yaw   -> [-128, 127]
    out[1] = (int8)(pitch * SCALE_PITCH  * INV_2PI + 0.5);   // pitch -> [-128, 127]
    out[2] = (int8)(roll  * SCALE_PITCH  * INV_2PI + 0.5);   // roll  -> [-128, 127]
}
```

Each angle is quantized to 256 steps over the full circle (2*pi radians), giving ~1.4 degree precision per byte. The function uses IEEE 754 rounding tricks (add bias, mask mantissa) to achieve fast float-to-int rounding.

### Position Format (Confirmed)

The SGW client sends positions as **raw 3x float32** (12 bytes) for the client-to-server path. There is no evidence of PackedXYZ/PackedXZ/PackedXHZ compression in the client-to-server send path (`addMove`). The position is written directly as 3 consecutive 32-bit floats.

For server-to-client avatar updates, all three position types (FullPos/OnChunk/OnGround) use the SAME 3x float32 encoding (12 bytes). The "OnChunk" and "OnGround" handlers simply ignore the Y float and derive height from the terrain/chunk system. This was confirmed by decompiling the handler functions (see [Position Movement Wire Formats](../reverse-engineering/findings/position-movement-wire-formats.md)).

### Velocity Packing in addMove

Velocity is also sent as **raw 3x float32** (12 bytes) in the client-to-server messages.

### Angle Delta Packing (Auxiliary Data)

The `addMove` function also computes angle deltas in the auxiliary 4-byte field:
```c
// At end of addMove (0x00dd9930 area):
aux[0] = (int8)(round(yaw_raw)   - round(yaw_packed));    // yaw residual
aux[1] = (int8)(round(pitch_raw) - round(pitch_packed));  // pitch residual
aux[2] = (int8)(round(roll_raw)  - round(roll_packed));   // roll residual
aux[3] = this->sequenceCounter;                             // move sequence number
```

## Ghidra RE: Entity ID Aliasing

### Aliasing Is Fully Supported

The SGW client **fully supports** entity ID aliasing. Evidence:

1. **All 16 Alias-variant message handlers are registered** -- RTTI types for all `avatarUpdateAlias*` variants exist (addresses `01e52968`-`01e52f08`), with corresponding handler code.

2. **Space Viewport ID (SVID) system** -- The `ServerConnection::svidFollow` function (at `0x00dd82d0`) traverses the vehicle chain using a `byte` viewport ID, not a full 32-bit entity ID. The function signature and debug string confirm this:
   ```
   "ServerConnection::svidFollow: Viewport for entity %d (immed vehicle %d, top vehicle %d) is unknown"
   ```

3. **spaceViewportInfo message** -- `ServerConnection::spaceViewportInfo` (at `0x00dda6c0`) manages a viewport table at `this+0xf84`. Each viewport entry is 16 bytes containing: spaceID, entityID, spaceRef, and a boolean flag. The viewport ID is a single byte (`param_1+12` is accessed as `byte*`), mapping a 1-byte alias to a full entity/space context.

4. **addMove uses SVID** -- When constructing movement updates, `addMove` calls `svidFollow` to get the viewport byte for the entity, then uses this 1-byte viewport ID in the message header instead of the full 4-byte entity ID.

5. **Alias in avatarUpdate receive path** -- The 16 `avatarUpdateAlias*` handlers allow the server to send position updates using 1-byte entity aliases instead of 4-byte entity IDs, saving 3 bytes per update for entities in the player's AoI cache.

### Implementation Requirement

The Cimmeria server must implement the SVID (Space Viewport ID) system to use aliased updates. When a client enters a space, the server sends `spaceViewportInfo` to assign a 1-byte SVID. Subsequent position updates for entities in that viewport can use `avatarUpdateAlias*` variants with the 1-byte SVID instead of the full 4-byte entity ID.

## Ghidra RE: Client-Side Prediction and Movement Filtering

### Movement Filter Architecture

SGW uses a filter-based movement system implemented in `BWFilter.cpp`. The class hierarchy is:

```
UBWFilter (.\Src\BWFilter.cpp)
  |-- UBWAvatarFilter        (base BW interpolation filter)
        |-- USGWAvatarFilter  (SGW-customized filter with velocity damping)
```

All filters are UE3 UObjects with `execInput`, `execOutput`, `execReset`, and `execGetLastInput` UnrealScript-callable methods. The filter is stored at entity offset `+0x394`.

### Ring Buffer Frame Storage

Both filter types maintain a ring buffer of position frames. Each frame is **0x38 bytes (56 bytes)**:

```
Frame structure (0x38 bytes):
  +0x00  float    timestamp
  +0x04  float    spaceViewportX     (x component of space reference)
  +0x08  float    spaceViewportY     (y component)
  +0x0C  float[3] position           (world position x, y, z)
  +0x18  float    speed              (scalar speed)
  +0x1C  float[3] velocity           (velocity vector x, y, z)
  +0x28  int      yaw                (packed yaw angle, int representation)
  +0x30  int      pitch              (packed pitch, int representation)
  +0x34  uint8    onGround           (on-ground flag)
```

The ring buffer size is stored at `DAT_01e69c8c` with a minimum of 5 frames (asserted: `"FrameCount > 4"` at BWFilter.cpp line 0x1ac for SGW, line 0x35 for BW). The current write index is at filter offset `+0x200`.

### BWAvatarFilter Output: Weighted Multi-Frame Interpolation

`BWAvatarFilter::Output` (at `0x00e824f0`) implements a sophisticated interpolation scheme:

1. **Adaptive time offset**: Maintains a smooth time offset (`+0x204`) that gradually converges to the target offset (`+0x208`) using a rate constant (`DAT_01e69c80`). This prevents jitter from network timing variations.

2. **Multi-sample blending**: Samples the frame buffer at multiple time offsets (up to `DAT_01e69c8c` samples), weighted equally (`1/numSamples`). Each sample calls `FUN_00e81430` which performs linear interpolation between the two frames bracketing the requested time:
   ```
   t = (requestedTime - frameA.time) / (frameB.time - frameA.time)
   position = frameA.position + (frameB.position - frameA.position) * t
   velocity = frameA.velocity + (frameB.velocity - frameA.velocity) * t
   direction = frameA.direction + (frameB.direction - frameA.direction) * (int)t
   ```

3. **Extrapolation beyond latest frame**: If the requested time is past the latest frame, extrapolates using velocity (multiplied by 0.0, effectively holding last position -- a conservative approach that avoids overshoot).

4. **Velocity normalization**: After computing the blended position, applies inverse-square-root normalization to the velocity vector to maintain unit-length direction.

### SGWAvatarFilter: Velocity-Damped Linear Extrapolation

`USGWAvatarFilter::Input` (at `0x00e81970`) modifies the base filter behavior:

1. **Teleport detection**: If the position delta per unit time exceeds a threshold (`DAT_01e69c90`), the filter performs a **hard snap** -- it copies the new position directly to all buffer frames (indices at `DAT_01e69c94`, `DAT_01e69c98`, `DAT_01ef26f0`) and clears the reset flag. This handles teleportation and large corrections.

2. **Velocity smoothing**: For normal movement, it **does not directly use the server's position**. Instead, it:
   - Computes a position delta between the current and previous frame
   - Scales this delta by `DAT_01816a84` (a damping factor < 1.0) multiplied by the inverse time delta
   - Further scales by `DAT_01903cd4` (another damping constant)
   - This produces smoothed velocity that is written to the interpolation frame
   - The effect is a low-pass filter on velocity changes, preventing jerky movement

3. **Angular velocity estimation**: Computes yaw and pitch angular velocities from the deltas between the new and stored angles:
   ```
   // Angle wrapping at +/- 0x4001 (16385 angle units)
   yawDelta = clamp(newYaw - storedYaw, -0x4001, 0x4001)
   angularYawVelocity = (yawDelta * 2 * inverseDt + prevAngularVel) * dampingFactor
   ```

`USGWAvatarFilter::Output` (at `0x00e81dc0`) performs **linear extrapolation**:
```
if (timeSinceLastFrame > 0):
    position = lastPosition + velocity * timeSinceLastFrame
    yaw = lastYaw + yawVelocity * timeSinceLastFrame
    pitch = lastPitch + pitchVelocity * timeSinceLastFrame
```

### Forced Position Handling

`ServerConnection::forcedPosition` (at `0x00dd9ee0`) handles server corrections:

1. Looks up the entity in the entity map at `this+0xfac`
2. Verifies the entity is controlled by this client (assert: `sentPhysics_[args.id] == args.physics`)
3. Calls `addMove` with `forced=true` to send the corrected position back to the server
4. If a callback handler exists (at `this+0x168`), invokes it to notify the game layer
5. The physics mode byte is stored per-entity to track the current movement mode

The forced position includes: entityID, spaceViewportID, position (Vec3), velocity (Vec3), yaw, pitch, roll, physics mode, and is treated as authoritative (bypasses prediction).

### Summary: Prediction Model

| Aspect | Mechanism |
|--------|-----------|
| **Own player** | Client-authoritative; sends position to server via `avatarUpdateImplicit/Explicit` |
| **Other entities** | Server sends positions; client interpolates between buffered frames |
| **Interpolation** | BWAvatarFilter: weighted multi-frame lerp with adaptive time offset |
| **Extrapolation** | SGWAvatarFilter: velocity-damped linear extrapolation beyond last known frame |
| **Teleport detection** | Position delta threshold triggers hard snap to new position |
| **Server correction** | `forcedPosition` message overrides client position; `addMove` echoes it back |
| **Buffer depth** | Minimum 5 frames; configurable via static data |
| **Direction smoothing** | Angular velocity computed from frame deltas with low-pass damping |

## Related Documents

- [Mercury Wire Format](mercury-wire-format.md) -- Packet-level protocol
- [Entity Property Sync](entity-property-sync.md) -- Property updates
- [Space Management](../engine/space-management.md) -- World grid and AoI
- [BigWorld Architecture](../engine/bigworld-architecture.md) -- Entity model

## TODO

- [x] Reverse engineer which of the 32 avatarUpdate variants SGW actually uses (Ghidra) -- All 32 standard handlers registered for receiving; client sends 4 SGW-specific types (Implicit/Explicit/WardImplicit/WardExplicit)
- [x] Determine the packed position/direction formats used by SGW (PackedXYZ, PackedXZ, etc.) -- Client-to-server uses raw float32 positions + int8-packed angles (256 steps/circle); server-to-client supports compressed formats via the 32 handler variants
- [ ] Document the vehicle/mount position system (setVehicle message)
- [x] Verify if SGW uses entity ID aliasing (BW EntityCache supports IDAlias -- see `engine/space-management.md` EntityCache section) -- Yes, fully supported via SVID (Space Viewport ID) system; 1-byte viewport aliases mapped via spaceViewportInfo message
- [x] Document client-side prediction and server reconciliation algorithm -- BWAvatarFilter (weighted multi-frame interpolation) and SGWAvatarFilter (velocity-damped linear extrapolation); forced position for server corrections
- [ ] Measure actual bandwidth usage of position updates in a typical session
