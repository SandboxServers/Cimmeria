# Position & Movement Wire Formats

> **Last updated**: 2026-03-05
> **RE Status**: COMPLETE — field-level wire format verified from Ghidra decompilation of handler functions
> **Sources**: Ghidra analysis of sgw.exe client binary, vtable tracing of `ClientMessageHandler<>` template instantiations, decompiled handler functions at 0x00dda000-0x00dddc00

---

## Overview

Position and movement messages in the BigWorld/Mercury protocol use a set of optimized encodings for server-to-client entity position updates. There are 34 total position-related messages:

- **32 avatarUpdate variants** (msg_id 0x10-0x2F): Compact entity position updates for AoI entities
- **detailedPosition** (msg_id 0x30): Full-precision position for non-player entities
- **forcedPosition** (msg_id 0x31): Server-authoritative position correction

All are CONSTANT_LENGTH messages (no u16 length prefix on the wire).

---

## forcedPosition (msg_id 0x31, 49 bytes)

Server forces an entity to a specific position. Used for teleports, spawn positions, and server corrections. This is the most complete position message, containing all fields at full precision.

**Handler**: `ServerConnection_forcedPosition` at `0x00dd9ee0`

### Wire Format

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 4 | int32 | entityID | Target entity |
| 4 | 4 | int32 | spaceID | Space/viewport ID |
| 8 | 4 | int32 | vehicleID | Vehicle entity ID (0 if none) |
| 12 | 4 | float32 | posX | Position X (world coords) |
| 16 | 4 | float32 | posY | Position Y (height) |
| 20 | 4 | float32 | posZ | Position Z (world coords) |
| 24 | 4 | float32 | velX | Velocity X |
| 28 | 4 | float32 | velY | Velocity Y |
| 32 | 4 | float32 | velZ | Velocity Z |
| 36 | 4 | float32 | roll | Roll angle (radians) |
| 40 | 4 | float32 | pitch | Pitch angle (radians) |
| 44 | 4 | float32 | yaw | Yaw angle (radians) |
| 48 | 1 | uint8 | physics | Physics/movement mode |

**Total: 49 bytes**

### Field Notes

- **Rotation order on wire**: roll (offset 36), pitch (offset 40), yaw (offset 44). This is Y-Z swapped from the logical (yaw, pitch, roll) order. The handler passes them to `addMove` as `(yaw=param[11], pitch=param[10], roll=param[9])`.
- **spaceID**: References the Space Viewport ID system. Identifies which space/viewport the entity belongs to.
- **vehicleID**: If the entity is mounted on a vehicle, this is the vehicle's entity ID. Zero when not mounted.
- **physics**: Encodes the current physics mode (walking, flying, swimming, etc.). Stored per-entity in `sentPhysics_[]`.

### Handler Behavior

1. Looks up entity in entity map at `this+0xfac`
2. Calls `ServerConnection::addMove` with `isForced=true` (echoes position back to server)
3. Invokes callback at `this+0x168` to notify game layer
4. Asserts `sentPhysics_[id] == args.physics`

---

## detailedPosition (msg_id 0x30, 41 bytes)

Full-precision position update without space/vehicle context. Used when the server needs to send complete position data for an entity without changing its space or vehicle assignment.

**Handler**: `FUN_00dd9e00` at `0x00dd9e00`

### Wire Format

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 4 | int32 | entityID | Target entity |
| 4 | 4 | float32 | posX | Position X |
| 8 | 4 | float32 | posY | Position Y |
| 12 | 4 | float32 | posZ | Position Z |
| 16 | 4 | float32 | velX | Velocity X |
| 20 | 4 | float32 | velY | Velocity Y |
| 24 | 4 | float32 | velZ | Velocity Z |
| 28 | 4 | float32 | roll | Roll angle (radians) |
| 32 | 4 | float32 | pitch | Pitch angle (radians) |
| 36 | 4 | float32 | yaw | Yaw angle (radians) |
| 40 | 1 | uint8 | physics | Physics/movement mode |

**Total: 41 bytes**

### Difference from forcedPosition

`detailedPosition` omits `spaceID` (4 bytes) and `vehicleID` (4 bytes) compared to `forcedPosition`. The entity's current space and vehicle context are preserved unchanged.

### Handler Behavior

1. Resolves entity via `FUN_00dd9d20` (SVID follow logic)
2. If the position is for the entity we control, stores position in the entity record at offset +0x10 (12 bytes)
3. Invokes the callback with full position/velocity/rotation data

---

## avatarUpdate Messages (msg_id 0x10-0x2F)

### Encoding Dimensions

The 32 variants are a 2x4x4 matrix combining three encoding parameters:

| Dimension | Options | Wire Impact |
|-----------|---------|-------------|
| **Entity ID** | NoAlias (4 bytes), Alias (1 byte) | 3-byte difference |
| **Position** | FullPos, OnChunk, OnGround, NoPos | 12 or 0 bytes |
| **Direction** | YawPitchRoll, YawPitch, Yaw, NoDir | 0-3 bytes |

### Compressed Field Encodings

Unlike `forcedPosition` and `detailedPosition` which use full float32 for all fields, the avatarUpdate messages use compact encodings:

#### Position: 3 x float32 (12 bytes) or absent

When present (FullPos/OnChunk/OnGround variants), position is encoded as three float32 values (12 bytes total), the same as in forcedPosition. The position type name (FullPos/OnChunk/OnGround) determines how the CLIENT INTERPRETS the Y value:

- **FullPos**: All three components (X, Y, Z) used directly
- **OnChunk**: X and Z used; Y replaced with sentinel (client derives Y from chunk height)
- **OnGround**: X and Z used; Y replaced with sentinel (client derives Y from terrain)

**Critical finding**: All three position types use the SAME wire format (3 x float32 = 12 bytes). The difference is purely in the handler function: FullPos handlers read `posY = data[offset+4]`, while OnChunk and OnGround handlers set `posY = SENTINEL` (a constant at `DAT_019d1a44`, likely `FLT_MAX`). The 4 bytes at the Y position offset are present in all variants but ignored by OnChunk/OnGround handlers. Confirmed by decompiling:
- `FUN_00ddb0c0` (NoAliasFullPosNoDir): `local_8 = param_1[2]` (uses Y)
- `FUN_00ddb220` (NoAliasOnChunkYPR): `local_8 = DAT_019d1a44` (ignores Y)
- `FUN_00ddb830` (NoAliasOnGroundYPR): `local_8 = DAT_019d1a44` (ignores Y)

When absent (NoPos variants), no position data is sent. The client uses the entity's previous position.

#### Velocity: 5 bytes (always present)

Velocity is ALWAYS present in all 32 variants, encoded in a compressed 5-byte format:

| Component | Encoding | Size |
|-----------|----------|------|
| Velocity XZ | Packed 3-byte format | 3 bytes |
| Velocity Y | Packed half-float (uint16) | 2 bytes |

**Velocity XZ packing** (decoded by `FUN_00de1850` at `0x00de1850`):

The 3 bytes encode two float values (velX and velZ) using a custom bit-packing scheme:
```
Byte layout: [b0] [b1] [b2]
b0 = first byte (8 bits)
b1_b2 = second and third bytes (uint16 LE)

velX mantissa bits: (b0 & 0x7FF000) << 3  (11 bits from upper portion of b0)
velZ mantissa bits: (b0 & 0x7FF) << 15    (11 bits from lower portion of b0)
velX sign: (b0 & 0xFF800000) << 8
velZ sign: (b1_b2 & 0xFFFFF8) << 28

Both values have an implicit exponent of 2.0 (IEEE 754 exponent field = 0x40000000)
and are offset by -DAT_01816a84 (likely 3.0 or similar bias constant)
```

**Velocity Y half-float packing** (decoded inline):
```c
uint16 raw = *(uint16*)(data + velY_offset);
float velY_sign    = (raw & 0x8000) << 16;          // sign bit
float velY_mantissa = ((raw & 0x7FFF) | 0x40000) << 12;  // implicit exponent
float velY = velY_sign | (velY_mantissa - BIAS);     // subtract bias constant
```

This is a custom 16-bit float with 1 sign bit, and 15 mantissa bits that are biased and shifted into IEEE 754 format.

#### Direction: 0-3 bytes packed int8

Each direction component is a single signed byte (int8), encoding an angle quantized to 256 steps over the full circle:

| Component | Encoding | Scale (unpack) |
|-----------|----------|----------------|
| Yaw | int8 | `value * DAT_019d12a4` (approx PI/128 for 0-2PI range) |
| Pitch | int8 | `value * DAT_019d12a8` (approx PI/128 for -PI to PI range) |
| Roll | int8 | `value * DAT_019d12a8` (same as pitch) |

Precision: ~1.4 degrees per step (360/256). Order on wire: yaw first, then pitch, then roll.

The packing function `FUN_00de1720` at `0x00de1720` performs the inverse: float angles to 3 packed bytes.

#### Physics Mode: 1 byte (always present)

Single uint8 byte encoding the current physics/movement mode. Always the last field before direction bytes.

### Complete Wire Layout Tables

#### NoAlias Variants (entity ID = 4 bytes, msg_id 0x10-0x1F)

##### FullPos / OnChunk / OnGround (with position)

These three position types share IDENTICAL wire layouts. The handler interprets the Y value differently.

**NoAliasFullPosYawPitchRoll** (msg_id 0x10, 25 bytes)
**NoAliasOnChunkYawPitchRoll** (msg_id 0x14, 25 bytes)
**NoAliasOnGroundYawPitchRoll** (msg_id 0x18, 25 bytes)

| Offset | Size | Type | Field |
|--------|------|------|-------|
| 0 | 4 | int32 | entityID |
| 4 | 4 | float32 | posX |
| 8 | 4 | float32 | posY (ignored by OnChunk/OnGround) |
| 12 | 4 | float32 | posZ |
| 16 | 3 | packed | velocityXZ |
| 19 | 2 | half16 | velocityY |
| 21 | 1 | uint8 | physics |
| 22 | 1 | int8 | yaw |
| 23 | 1 | int8 | pitch |
| 24 | 1 | int8 | roll |

**NoAliasFullPosYawPitch** (msg_id 0x11, 24 bytes)
**NoAliasOnChunkYawPitch** (msg_id 0x15, 24 bytes)
**NoAliasOnGroundYawPitch** (msg_id 0x19, 24 bytes)

| Offset | Size | Type | Field |
|--------|------|------|-------|
| 0 | 4 | int32 | entityID |
| 4 | 4 | float32 | posX |
| 8 | 4 | float32 | posY |
| 12 | 4 | float32 | posZ |
| 16 | 3 | packed | velocityXZ |
| 19 | 2 | half16 | velocityY |
| 21 | 1 | uint8 | physics |
| 22 | 1 | int8 | yaw |
| 23 | 1 | int8 | pitch |

**NoAliasFullPosYaw** (msg_id 0x12, 23 bytes)
**NoAliasOnChunkYaw** (msg_id 0x16, 23 bytes)
**NoAliasOnGroundYaw** (msg_id 0x1A, 23 bytes)

| Offset | Size | Type | Field |
|--------|------|------|-------|
| 0 | 4 | int32 | entityID |
| 4 | 4 | float32 | posX |
| 8 | 4 | float32 | posY |
| 12 | 4 | float32 | posZ |
| 16 | 3 | packed | velocityXZ |
| 19 | 2 | half16 | velocityY |
| 21 | 1 | uint8 | physics |
| 22 | 1 | int8 | yaw |

**NoAliasFullPosNoDir** (msg_id 0x13, 22 bytes)
**NoAliasOnChunkNoDir** (msg_id 0x17, 22 bytes)
**NoAliasOnGroundNoDir** (msg_id 0x1B, 22 bytes)

| Offset | Size | Type | Field |
|--------|------|------|-------|
| 0 | 4 | int32 | entityID |
| 4 | 4 | float32 | posX |
| 8 | 4 | float32 | posY |
| 12 | 4 | float32 | posZ |
| 16 | 3 | packed | velocityXZ |
| 19 | 2 | half16 | velocityY |
| 21 | 1 | uint8 | physics |

##### NoPos (without position)

**NoAliasNoPosYawPitchRoll** (msg_id 0x1C, 13 bytes)

| Offset | Size | Type | Field |
|--------|------|------|-------|
| 0 | 4 | int32 | entityID |
| 4 | 3 | packed | velocityXZ |
| 7 | 2 | half16 | velocityY |
| 9 | 1 | uint8 | physics |
| 10 | 1 | int8 | yaw |
| 11 | 1 | int8 | pitch |
| 12 | 1 | int8 | roll |

**NoAliasNoPosYawPitch** (msg_id 0x1D, 12 bytes)

| Offset | Size | Type | Field |
|--------|------|------|-------|
| 0 | 4 | int32 | entityID |
| 4 | 3 | packed | velocityXZ |
| 7 | 2 | half16 | velocityY |
| 9 | 1 | uint8 | physics |
| 10 | 1 | int8 | yaw |
| 11 | 1 | int8 | pitch |

**NoAliasNoPosYaw** (msg_id 0x1E, 11 bytes)

| Offset | Size | Type | Field |
|--------|------|------|-------|
| 0 | 4 | int32 | entityID |
| 4 | 3 | packed | velocityXZ |
| 7 | 2 | half16 | velocityY |
| 9 | 1 | uint8 | physics |
| 10 | 1 | int8 | yaw |

**NoAliasNoPosNoDir** (msg_id 0x1F, 10 bytes)

| Offset | Size | Type | Field |
|--------|------|------|-------|
| 0 | 4 | int32 | entityID |
| 4 | 3 | packed | velocityXZ |
| 7 | 2 | half16 | velocityY |
| 9 | 1 | uint8 | physics |

#### Alias Variants (entity alias = 1 byte, msg_id 0x20-0x2F)

Alias variants are identical to their NoAlias counterparts except the 4-byte `entityID` is replaced with a 1-byte `alias`. The alias is an index into the client's viewport alias table at `this + 0xb7c`, which resolves to a full entity ID via `entityID = *(int32*)(this + alias * 4 + 0xb7c)`.

##### FullPos / OnChunk / OnGround (with position)

**AliasFullPosYawPitchRoll** (msg_id 0x20, 22 bytes)
**AliasOnChunkYawPitchRoll** (msg_id 0x24, 22 bytes)
**AliasOnGroundYawPitchRoll** (msg_id 0x28, 22 bytes)

| Offset | Size | Type | Field |
|--------|------|------|-------|
| 0 | 1 | uint8 | alias |
| 1 | 4 | float32 | posX |
| 5 | 4 | float32 | posY (ignored by OnChunk/OnGround) |
| 9 | 4 | float32 | posZ |
| 13 | 3 | packed | velocityXZ |
| 16 | 2 | half16 | velocityY |
| 18 | 1 | uint8 | physics |
| 19 | 1 | int8 | yaw |
| 20 | 1 | int8 | pitch |
| 21 | 1 | int8 | roll |

**AliasFullPosYawPitch** (msg_id 0x21, 21 bytes)
**AliasOnChunkYawPitch** (msg_id 0x25, 21 bytes)
**AliasOnGroundYawPitch** (msg_id 0x29, 21 bytes)

| Offset | Size | Type | Field |
|--------|------|------|-------|
| 0 | 1 | uint8 | alias |
| 1 | 4 | float32 | posX |
| 5 | 4 | float32 | posY |
| 9 | 4 | float32 | posZ |
| 13 | 3 | packed | velocityXZ |
| 16 | 2 | half16 | velocityY |
| 18 | 1 | uint8 | physics |
| 19 | 1 | int8 | yaw |
| 20 | 1 | int8 | pitch |

**AliasFullPosYaw** (msg_id 0x22, 20 bytes)
**AliasOnChunkYaw** (msg_id 0x26, 20 bytes)
**AliasOnGroundYaw** (msg_id 0x2A, 20 bytes)

| Offset | Size | Type | Field |
|--------|------|------|-------|
| 0 | 1 | uint8 | alias |
| 1 | 4 | float32 | posX |
| 5 | 4 | float32 | posY |
| 9 | 4 | float32 | posZ |
| 13 | 3 | packed | velocityXZ |
| 16 | 2 | half16 | velocityY |
| 18 | 1 | uint8 | physics |
| 19 | 1 | int8 | yaw |

**AliasFullPosNoDir** (msg_id 0x23, 19 bytes)
**AliasOnChunkNoDir** (msg_id 0x27, 19 bytes)
**AliasOnGroundNoDir** (msg_id 0x2B, 19 bytes)

| Offset | Size | Type | Field |
|--------|------|------|-------|
| 0 | 1 | uint8 | alias |
| 1 | 4 | float32 | posX |
| 5 | 4 | float32 | posY |
| 9 | 4 | float32 | posZ |
| 13 | 3 | packed | velocityXZ |
| 16 | 2 | half16 | velocityY |
| 18 | 1 | uint8 | physics |

##### NoPos (without position)

**AliasNoPosYawPitchRoll** (msg_id 0x2C, 10 bytes)

| Offset | Size | Type | Field |
|--------|------|------|-------|
| 0 | 1 | uint8 | alias |
| 1 | 3 | packed | velocityXZ |
| 4 | 2 | half16 | velocityY |
| 6 | 1 | uint8 | physics |
| 7 | 1 | int8 | yaw |
| 8 | 1 | int8 | pitch |
| 9 | 1 | int8 | roll |

**AliasNoPosYawPitch** (msg_id 0x2D, 9 bytes)

| Offset | Size | Type | Field |
|--------|------|------|-------|
| 0 | 1 | uint8 | alias |
| 1 | 3 | packed | velocityXZ |
| 4 | 2 | half16 | velocityY |
| 6 | 1 | uint8 | physics |
| 7 | 1 | int8 | yaw |
| 8 | 1 | int8 | pitch |

**AliasNoPosYaw** (msg_id 0x2E, 8 bytes)

| Offset | Size | Type | Field |
|--------|------|------|-------|
| 0 | 1 | uint8 | alias |
| 1 | 3 | packed | velocityXZ |
| 4 | 2 | half16 | velocityY |
| 6 | 1 | uint8 | physics |
| 7 | 1 | int8 | yaw |

**AliasNoPosNoDir** (msg_id 0x2F, 7 bytes)

| Offset | Size | Type | Field |
|--------|------|------|-------|
| 0 | 1 | uint8 | alias |
| 1 | 3 | packed | velocityXZ |
| 4 | 2 | half16 | velocityY |
| 6 | 1 | uint8 | physics |

---

## Size Summary Table

| Msg ID | Variant | Size |
|--------|---------|------|
| 0x10 | NoAliasFullPosYawPitchRoll | 25 |
| 0x11 | NoAliasFullPosYawPitch | 24 |
| 0x12 | NoAliasFullPosYaw | 23 |
| 0x13 | NoAliasFullPosNoDir | 22 |
| 0x14 | NoAliasOnChunkYawPitchRoll | 25 |
| 0x15 | NoAliasOnChunkYawPitch | 24 |
| 0x16 | NoAliasOnChunkYaw | 23 |
| 0x17 | NoAliasOnChunkNoDir | 22 |
| 0x18 | NoAliasOnGroundYawPitchRoll | 25 |
| 0x19 | NoAliasOnGroundYawPitch | 24 |
| 0x1A | NoAliasOnGroundYaw | 23 |
| 0x1B | NoAliasOnGroundNoDir | 22 |
| 0x1C | NoAliasNoPosYawPitchRoll | 13 |
| 0x1D | NoAliasNoPosYawPitch | 12 |
| 0x1E | NoAliasNoPosYaw | 11 |
| 0x1F | NoAliasNoPosNoDir | 10 |
| 0x20 | AliasFullPosYawPitchRoll | 22 |
| 0x21 | AliasFullPosYawPitch | 21 |
| 0x22 | AliasFullPosYaw | 20 |
| 0x23 | AliasFullPosNoDir | 19 |
| 0x24 | AliasOnChunkYawPitchRoll | 22 |
| 0x25 | AliasOnChunkYawPitch | 21 |
| 0x26 | AliasOnChunkYaw | 20 |
| 0x27 | AliasOnChunkNoDir | 19 |
| 0x28 | AliasOnGroundYawPitchRoll | 22 |
| 0x29 | AliasOnGroundYawPitch | 21 |
| 0x2A | AliasOnGroundYaw | 20 |
| 0x2B | AliasOnGroundNoDir | 19 |
| 0x2C | AliasNoPosYawPitchRoll | 10 |
| 0x2D | AliasNoPosYawPitch | 9 |
| 0x2E | AliasNoPosYaw | 8 |
| 0x2F | AliasNoPosNoDir | 7 |
| 0x30 | detailedPosition | 41 |
| 0x31 | forcedPosition | 49 |

---

## Field Size Formula

For any avatarUpdate variant, the total size can be computed as:

```
size = entity_size + position_size + velocity_size + physics_size + direction_size

where:
  entity_size    = 4 (NoAlias) or 1 (Alias)
  position_size  = 12 (FullPos/OnChunk/OnGround) or 0 (NoPos)
  velocity_size  = 5 (always: 3 bytes XZ + 2 bytes Y)
  physics_size   = 1 (always)
  direction_size = 3 (YPR) or 2 (YP) or 1 (Yaw) or 0 (NoDir)
```

Examples:
- NoAliasFullPosYPR: 4 + 12 + 5 + 1 + 3 = **25**
- AliasNoPosNoDir: 1 + 0 + 5 + 1 + 0 = **7**
- AliasFullPosYaw: 1 + 12 + 5 + 1 + 1 = **20**

---

## Handler Function Addresses

### avatarUpdate Handlers (Decompiled)

All handlers share the same pattern: resolve entity ID (from alias or direct), decode position/velocity/direction fields, invoke callback.

| Variant | Handler | Address | Verified |
|---------|---------|---------|----------|
| NoAliasFullPosNoDir | `FUN_00ddb0c0` | 0x00ddb0c0 | YES (decompiled) |
| NoAliasOnChunkYPR | `FUN_00ddb220` | 0x00ddb220 | YES (decompiled, identical to OnGround) |
| NoAliasOnGroundYPR | `FUN_00ddb830` | 0x00ddb830 | YES (decompiled) |
| NoAliasNoPosYPR | `FUN_00ddbe40` | 0x00ddbe40 | YES (decompiled) |
| AliasFullPosNoDir | `FUN_00ddc8e0` | 0x00ddc8e0 | YES (decompiled) |
| AliasFullPosYPR | `FUN_00ddc420` | 0x00ddc420 | YES (decompiled) |

### handleMessage Wrappers (vtable vfunc_1)

These template-generated wrappers read N bytes from the stream and dispatch to the handler:

| Function | Read Size | Used By |
|----------|-----------|---------|
| `FUN_00de1a20` | 25 (0x19) | NoAlias + Position + YPR |
| `FUN_00de19f0` | 24 (0x18) | NoAlias + Position + YP |
| `FUN_00de19c0` | 23 (0x17) | NoAlias + Position + Yaw |
| `FUN_00de1a50` | 22 (0x16) | NoAlias + Position + NoDir; Alias + Position + YPR |
| `FUN_00de1b40` | 21 (0x15) | Alias + Position + YP |
| `FUN_00de1b70` | 20 (0x14) | Alias + Position + Yaw |
| `FUN_00de1b10` | 19 (0x13) | Alias + Position + NoDir |
| `FUN_00de1a80` | 13 (0x0D) | NoAlias + NoPos + YPR |
| `FUN_00de1ab0` | 12 (0x0C) | NoAlias + NoPos + YP |
| `FUN_00de1ae0` | 11 (0x0B) | NoAlias + NoPos + Yaw |
| `FUN_00de1ba0` | 10 (0x0A) | NoAlias + NoPos + NoDir; Alias + NoPos + YPR |
| `FUN_00de1bd0` | 9 (0x09) | Alias + NoPos + YP |
| `FUN_00de1990` | 8 (0x08) | Alias + NoPos + Yaw |
| `FUN_00de1c00` | 7 (0x07) | Alias + NoPos + NoDir |
| `FUN_00de1c30` | 41 (0x29) | detailedPosition |
| `FUN_00de1c60` | 49 (0x31) | forcedPosition |

### Vtable Layout

All `ClientMessageHandler<>` vtables are at stride 12 bytes in the range `0x019d1410` - `0x019d15a8`:

```
vtable_base - 4: RTTI_Complete_Object_Locator pointer
vtable_base + 0: destructor (shared: 0x00de1cc0 for all variants)
vtable_base + 4: handleMessage wrapper (reads N bytes, dispatches to handler)
```

---

## Velocity Decoder Reference

### FUN_00de1850 — Velocity XZ Decoder (3 bytes to 2 floats)

Address: `0x00de1850`

Input: 3 bytes at `this`
Output: `*param_1` = velX (float), `*param_2` = velZ (float)

```c
void decode_velocity_xz(uint8* data, float* velX, float* velZ) {
    uint32 b0 = data[0];                    // first byte
    uint16 b1_b2 = *(uint16*)(data + 1);   // second + third bytes (LE)

    // Start with implicit exponent 2.0 (0x40000000 in IEEE 754)
    *velX = 2.0f;
    *velZ = 2.0f;

    // Pack mantissa bits
    *velX = float_bits(*velX) | ((b0 & 0x7FF000) << 3);   // 11 bits
    *velZ = float_bits(*velZ) | ((b0 & 0x7FF) << 15);     // 11 bits

    // Subtract bias
    *velX -= BIAS;  // DAT_01816a84
    *velZ -= BIAS;

    // Apply sign bits
    *velX = float_bits(*velX) | ((b0 & 0xFF800000) << 8);
    *velZ = float_bits(*velZ) | ((b1_b2 & 0xFFFFF8) << 28);
}
```

### Velocity Y Decoder (2 bytes to 1 float, inline)

```c
float decode_velocity_y(uint16 raw) {
    uint32 sign = (raw & 0x8000) << 16;            // bit 15 -> bit 31
    float mantissa = ((raw & 0x7FFF) | 0x40000) << 12;  // 15 bits + implicit 2.0 exponent
    return float_bits(sign | uint32(mantissa - BIAS));
}
```

---

## Server Implementation Recommendations

### Simplest Approach

For initial implementation, use only `forcedPosition` (msg_id 0x31, 49 bytes) for all entity position updates. This provides full precision and includes all fields. The client handles it correctly.

### Bandwidth-Optimized Approach

For AoI entity updates, select the minimal variant:
1. Use **Alias** if the entity has a viewport alias assigned via `spaceViewportInfo`
2. Use **FullPos** if position changed (always safe; OnChunk/OnGround provide no bandwidth savings in SGW since all three use 12 bytes)
3. Use **NoPos** if only velocity/direction changed
4. Use **Yaw** for most entities (skip pitch/roll unless entity is flying/swimming)
5. Velocity XZ/Y packing is complex; consider sending zero velocity (5 zero bytes) and letting the client extrapolate from position deltas

### Physics Mode Byte

The physics byte must match what the client expects. Tracked per-entity. The `forcedPosition` handler asserts that `sentPhysics_[id] == args.physics`.

---

## Related Documents

- [Message Dispatch Table](../../protocol/message-dispatch-table.md) -- Message IDs and sizes
- [Position Update Protocol](../../protocol/position-updates.md) -- High-level overview and client-side prediction
- [Space Viewport Wire Formats](space-viewport-wire-formats.md) -- spaceViewportInfo message
- [System Protocol Wire Formats](system-protocol-wire-formats.md) -- FORCED_POSITION in system message context
