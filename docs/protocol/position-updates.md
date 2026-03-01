# Position Update Protocol

> **Last updated**: 2026-03-01
> **RE Status**: Partially documented from BigWorld source + Cimmeria movement code
> **Sources**: `external/engines/BigWorld-Engine-2.0.1/src/lib/connection/common_client_interface.hpp`, `src/cellapp/entity/movement.hpp`, `src/mercury/base_cell_messages.hpp`

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

| Type | Format | Size | Description |
|------|--------|------|-------------|
| `FullPos` | `PackedXYZ` | 3 floats (12 bytes) | Full 3D position |
| `OnChunk` | `PackedXHZ` | 3 values (compressed) | X, height-on-chunk, Z |
| `OnGround` | `PackedXZ` | 2 values (compressed) | X, Z only (Y from terrain) |
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

Used to correct client prediction or set initial position:

```
forcedPosition:
  uint32    Entity ID
  Vec3      Position
  Vec3      Velocity
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

## Related Documents

- [Mercury Wire Format](mercury-wire-format.md) -- Packet-level protocol
- [Entity Property Sync](entity-property-sync.md) -- Property updates
- [Space Management](../engine/space-management.md) -- World grid and AoI
- [BigWorld Architecture](../engine/bigworld-architecture.md) -- Entity model

## TODO

- [ ] Reverse engineer which of the 32 avatarUpdate variants SGW actually uses (Ghidra)
- [ ] Determine the packed position/direction formats used by SGW (PackedXYZ, PackedXZ, etc.)
- [ ] Document the vehicle/mount position system (setVehicle message)
- [ ] Verify if SGW uses entity ID aliasing (BW EntityCache supports IDAlias — see `engine/space-management.md` EntityCache section)
- [ ] Document client-side prediction and server reconciliation algorithm
- [ ] Measure actual bandwidth usage of position updates in a typical session
