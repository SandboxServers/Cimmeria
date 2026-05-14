---
title: Position Update Wire Formats
chapter_id: spec.protocol.position-updates
status: draft
last_verified: 2026-05-14
verified_by: automated-agent
confidence:
  re: high
  client: n/a
  deprecated: n/a
  rust_expected: n/a
  rust_actual: n/a
evidence_refs:
  re:
    - docs/reverse-engineering/findings/position-movement-wire-formats.md
    - docs/reverse-engineering/findings/space-viewport-wire-formats.md
    - docs/reverse-engineering/findings/entity-creation-wire-formats.md
    - docs/reverse-engineering/findings/system-protocol-wire-formats.md
    - ghidra://SGW.exe@0x00dd9ee0
    - ghidra://SGW.exe@0x00dd9e00
    - ghidra://SGW.exe@0x00de1850
    - ghidra://SGW.exe@0x00de1720
    - ghidra://SGW.exe@0x00ddb0c0
    - ghidra://SGW.exe@0x00ddb220
    - ghidra://SGW.exe@0x00ddb830
    - ghidra://SGW.exe@0x00ddbe40
    - ghidra://SGW.exe@0x00ddc420
    - ghidra://SGW.exe@0x00ddc8e0
  client: []
  deprecated: []
  rust: []
related_chapters:
  - spec.protocol.mercury-wire-format
  - spec.protocol.message-catalog
  - spec.world.world-entry
disputed_by: []
supersedes: []
---

# Position Update Wire Formats

The position-update plane carries every per-entity location message the server pushes to the client. Three families share the role — the 32 `UPDATE_AVATAR` variants (`msg_ids 0x10–0x2F`) for compressed AoI broadcasts, `detailedPosition` (`msg_id 0x30`) for full-precision non-controlled-entity snaps, and `forcedPosition` (`msg_id 0x31`) for authoritative client-position snaps. This chapter canonizes the byte-by-byte layout of every variant in that range; the Mercury envelope that wraps them is in `spec.protocol.mercury-wire-format`.

The position plane is the densest part of the message catalog. Thirty-four message IDs cover one logical operation ("here is this entity's new position") encoded thirty-four different ways, trading bytes for precision and trading precision for axes-of-freedom. The 32-variant compression scheme exists because position spam dominates AoI bandwidth — collapsing 49 bytes to 7 bytes for a stationary entity update is the difference between a playable AoI and a saturated link.

---

## Section 1 — RE findings

Every wire layout below is V5-anchored against `position-movement-wire-formats.md` (the field-level table) and `space-viewport-wire-formats.md` (the message-catalog table). Where the two V5 docs overlap, the field tables in this chapter follow `position-movement-wire-formats.md` because that doc is decompile-grounded against handler functions in the `0x00ddb???` and `0x00ddc???` ranges; `space-viewport-wire-formats.md` is the cross-check.

### 1.1 The 34-message position plane

Position-update messages occupy `msg_ids 0x10` through `0x31` plus the C→S `avatarUpdate` family that mirrors them. All are `CONSTANT_LENGTH` — no `u16` length prefix on the wire. Each variant's size is registered in the `InterfaceElement` table at static-init time and the receiver reads exactly that many bytes from the bundle stream per message.

| Range | Family | Length type | Wire shape |
|---|---|---|---|
| `0x10 – 0x2F` | `UPDATE_AVATAR` (32 variants) | `CONSTANT_LENGTH` (per variant; 7–25 bytes) | Compressed AoI position broadcast |
| `0x30` | `detailedPosition` | `CONSTANT_LENGTH = 41` | Full-precision non-controlled-entity snap |
| `0x31` | `forcedPosition` | `CONSTANT_LENGTH = 49` | Authoritative client-position snap |

Three invariants apply across the whole plane:

- **Constant length, no length prefix.** Every message in this range has a fixed payload size that the client decoder knows from the `InterfaceElement` descriptor.
- **No spaceId / vehicleId in `UPDATE_AVATAR` or `detailedPosition`.** Only `forcedPosition` carries those fields. The other two preserve the entity's existing space and vehicle binding.
- **Physics-mode trailing byte.** Every message in this plane ends with a 1-byte physics-mode field — the same per-entity mutable state used as the `sentPhysics_[]` assertion key in the `forcedPosition` handler.

### 1.2 `UPDATE_AVATAR` — the 32-variant compressed AoI broadcast

The compressed AoI position update. Each of the 32 variants encodes a position update for one ghost entity (an entity in the client's Area of Interest, server-authoritative, client-side-rendered). The variant index is a 5-bit field encoded into the `msg_id` byte itself; the 5 bits select which subset of `(idAlias, position, direction)` fields are present on the wire.

**Does not work on client-controlled entities.** Per `position-movement-wire-formats.md` §"avatarUpdate Messages" and the symmetric callout at §"forcedPosition" / §"detailedPosition", the avatarUpdate handlers reject any update targeted at a locally controlled entity. Use `forcedPosition` (§1.4) for the client's own player.

#### 1.2.1 Variant taxonomy — the 2×4×4 matrix

The 32 variants map a 5-bit index onto a 2×4×4 matrix:

| Dimension | Options | Wire-byte impact |
|---|---|---|
| Entity ID width | `NoAlias` (4-byte `u32`) or `Alias` (1-byte `u8`) | Saves 3 bytes when an alias has been assigned |
| Position width | `FullPos`, `OnChunk`, `OnGround` (each 12 bytes), or `NoPos` (0 bytes) | Saves 12 bytes when omitted |
| Direction width | `YawPitchRoll` (3 B), `YawPitch` (2 B), `Yaw` (1 B), `NoDir` (0 B) | Saves 0–3 bytes |

The `msg_id` byte itself selects the combination. The mapping is structural: the low 2 bits select direction, bits 2-3 select position type, bit 4 selects alias vs no-alias.

| `msg_id` range | Alias | Position | Direction (YPR / YP / Y / NoDir) | Sizes |
|---|---|---|---|---|
| `0x10 – 0x13` | NoAlias (4 B) | FullPos | YPR / YP / Y / NoDir | 25 / 24 / 23 / 22 |
| `0x14 – 0x17` | NoAlias (4 B) | OnChunk | YPR / YP / Y / NoDir | 25 / 24 / 23 / 22 |
| `0x18 – 0x1B` | NoAlias (4 B) | OnGround | YPR / YP / Y / NoDir | 25 / 24 / 23 / 22 |
| `0x1C – 0x1F` | NoAlias (4 B) | NoPos | YPR / YP / Y / NoDir | 13 / 12 / 11 / 10 |
| `0x20 – 0x23` | Alias (1 B) | FullPos | YPR / YP / Y / NoDir | 22 / 21 / 20 / 19 |
| `0x24 – 0x27` | Alias (1 B) | OnChunk | YPR / YP / Y / NoDir | 22 / 21 / 20 / 19 |
| `0x28 – 0x2B` | Alias (1 B) | OnGround | YPR / YP / Y / NoDir | 22 / 21 / 20 / 19 |
| `0x2C – 0x2F` | Alias (1 B) | NoPos | YPR / YP / Y / NoDir | 10 / 9 / 8 / 7 |

The byte count is `entity_size + position_size + velocity_size + physics_size + direction_size`:

```text
size = entity + position + 5 (velocity, always) + 1 (physics, always) + direction

  entity    = 4 (NoAlias) or 1 (Alias)
  position  = 12 (FullPos/OnChunk/OnGround) or 0 (NoPos)
  velocity  = 5 (always: 3 bytes XZ + 2 bytes Y)
  physics   = 1 (always)
  direction = 3 (YPR), 2 (YP), 1 (Yaw), or 0 (NoDir)
```

Sanity checks:

- `NoAliasFullPosYPR (0x10)`: `4 + 12 + 5 + 1 + 3 = 25`
- `AliasNoPosNoDir (0x2F)`: `1 + 0 + 5 + 1 + 0 = 7`
- `AliasFullPosYaw (0x22)`: `1 + 12 + 5 + 1 + 1 = 20`

#### 1.2.2 Position-type semantics

Even though the three "with position" variants (`FullPos`, `OnChunk`, `OnGround`) all carry 12 wire bytes of position, the per-variant handler differs in how it interprets the Y component:

- `FullPos` handlers (e.g. `FUN_00ddb0c0` at `ghidra://SGW.exe@0x00ddb0c0`): read all three floats as-is (`local_8 = param_1[2]`).
- `OnChunk` handlers (e.g. `FUN_00ddb220` at `ghidra://SGW.exe@0x00ddb220`): discard the wire Y and substitute the sentinel at `DAT_019d1a44` (likely `FLT_MAX`); the client derives Y from the chunk's height map.
- `OnGround` handlers (e.g. `FUN_00ddb830` at `ghidra://SGW.exe@0x00ddb830`): discard the wire Y and substitute the same sentinel; the client derives Y from terrain ray-cast.

The 4 wire bytes at the Y offset are still present in every variant — the difference is purely how the handler consumes them. A reimplementation can always emit the same 12 position bytes regardless of variant; the variant choice is the server's signal to the *client* about how to interpret Y, not a wire-format change.

#### 1.2.3 Velocity — `packXYZ` 5-byte compression

Velocity is always present in all 32 variants, encoded in 5 bytes: a packed `u32` plus a tail `u8`. The encoding extracts mantissa bits from each component's IEEE 754 representation, adds a bias of `2.0` to the absolute value (avoiding zero-encoding), and concatenates sign/magnitude fields.

**Bit layout** per `position-movement-wire-formats.md` §"Velocity Compression":

```text
packed1 (u32 LE):
  bits [31:24]:  Y delta high byte
  bit  [23]:     X sign (1 = negative)
  bits [22:12]:  X mantissa (11 bits)
  bit  [11]:     Z sign (1 = negative)
  bits [10:0]:   Z mantissa (11 bits)

packed2 (u8):
  bit  [7]:      Y sign (1 = negative)
  bits [6:0]:    Y delta low 7 bits
```

The XZ decoder is `FUN_00de1850` at `ghidra://SGW.exe@0x00de1850`; the Y decoder is inlined into each variant's handler. The encode counterpart is `FUN_00de1720` at `ghidra://SGW.exe@0x00de1720`.

A reimplementation must replicate the bias-then-extract pipeline exactly. Emitting raw IEEE 754 bytes will produce a client-side velocity off by ~`2.0` in each axis.

#### 1.2.4 Direction — quantized `u8` angles

Each direction angle is a `u8` (decoded as `int8` per `position-movement-wire-formats.md` §"Direction") encoding 256 evenly-spaced steps over `2π` radians. The encode constant `0.024543693 = 2π / 256` is anchored at `DAT_01816a84`. To decode: `angle_rad = byte * 0.024543693`. To encode: `byte = angle_rad / 0.024543693`.

The wire order is `yaw, pitch, roll` (in that order, when present). The encoded *source* axes are `rotation.y, rotation.x, rotation.z` respectively — the SGW server source explicitly writes `(rotation.y / k), (rotation.x / k), (rotation.z / k)` per `client_handler.cpp:548-555`:

```cpp
bundle << entityId << position.x << position.y << position.z;
packXYZ(bundle, velocity);
bundle << (uint8_t)flags <<
    (uint8_t)(rotation.y / 0.024543693f) <<  // yaw
    (uint8_t)(rotation.x / 0.024543693f) <<  // pitch
    (uint8_t)(rotation.z / 0.024543693f);    // roll
```

Precision is ~1.4 degrees per step (`360 / 256`).

#### 1.2.5 Physics mode — the trailing `u8`

A single `u8` byte encoding the current physics/movement mode (walking, flying, swimming, etc.). Always present, always positioned immediately before the direction bytes (or at the end of the message when `NoDir`). The same per-entity value used as the `sentPhysics_[id] == args.physics` assertion key in the `forcedPosition` handler at `0x00dd9ee0`.

The byte's value is `0x01` for the typical world-entry path per `client_handler.cpp:407-413`; observed traffic carries different values for different movement modes.

#### 1.2.6 Canonical variant — `NoAliasFullPosYawPitchRoll` (`msg_id 0x10`, 25 bytes)

The variant the SGW server defaults to in `client_handler.cpp:548-556` for most observed AoI traffic. The other 31 variants are byte-shorter compressions of the same field set.

| Offset | Size | Type | Field |
|---:|---:|---|---|
| 0 | 4 | `u32 LE` | `entityId` — full entity ID (NoAlias variant) |
| 4 | 4 | `f32 LE` | `posX` — world X position |
| 8 | 4 | `f32 LE` | `posY` — world Y position (vertical) |
| 12 | 4 | `f32 LE` | `posZ` — world Z position |
| 16 | 3 | packed | `velocityXZ` — see §1.2.3 |
| 19 | 2 | `u16 LE` packed | `velocityY` — see §1.2.3 |
| 21 | 1 | `u8` | `physics` — movement mode (`0x01` typical) |
| 22 | 1 | `i8` | `yaw` |
| 23 | 1 | `i8` | `pitch` |
| 24 | 1 | `i8` | `roll` |

Wire-byte view:

```text
[0x10]                        msg_id
[entityId:  u32 LE]           4 bytes
[posX:      f32 LE]           4 bytes
[posY:      f32 LE]           4 bytes
[posZ:      f32 LE]           4 bytes
[velocityXZ: 3 bytes packed]  3 bytes
[velocityY:  2 bytes packed]  2 bytes
[physics:   u8]               1 byte
[yaw:       i8]               1 byte
[pitch:     i8]               1 byte
[roll:      i8]               1 byte
```

Total: `1 + 4 + 12 + 5 + 1 + 3 = 26 bytes on the wire including msg_id`; payload is 25 bytes.

#### 1.2.7 NoAlias variants with position — `0x10 – 0x1B`

The four "with position" rows of the NoAlias block. All three position types (`FullPos`, `OnChunk`, `OnGround`) share an identical wire layout; the variant choice signals the *client* how to interpret the Y bytes (see §1.2.2).

**`msg_ids 0x10 / 0x14 / 0x18` — `NoAlias{FullPos,OnChunk,OnGround}YawPitchRoll`, 25 bytes:** layout in §1.2.6.

**`msg_ids 0x11 / 0x15 / 0x19` — `NoAlias{...}YawPitch`, 24 bytes:**

| Offset | Size | Type | Field |
|---:|---:|---|---|
| 0 | 4 | `u32 LE` | `entityId` |
| 4 | 4 | `f32 LE` | `posX` |
| 8 | 4 | `f32 LE` | `posY` |
| 12 | 4 | `f32 LE` | `posZ` |
| 16 | 3 | packed | `velocityXZ` |
| 19 | 2 | packed | `velocityY` |
| 21 | 1 | `u8` | `physics` |
| 22 | 1 | `i8` | `yaw` |
| 23 | 1 | `i8` | `pitch` |

**`msg_ids 0x12 / 0x16 / 0x1A` — `NoAlias{...}Yaw`, 23 bytes:**

| Offset | Size | Type | Field |
|---:|---:|---|---|
| 0 | 4 | `u32 LE` | `entityId` |
| 4 | 4 | `f32 LE` | `posX` |
| 8 | 4 | `f32 LE` | `posY` |
| 12 | 4 | `f32 LE` | `posZ` |
| 16 | 3 | packed | `velocityXZ` |
| 19 | 2 | packed | `velocityY` |
| 21 | 1 | `u8` | `physics` |
| 22 | 1 | `i8` | `yaw` |

**`msg_ids 0x13 / 0x17 / 0x1B` — `NoAlias{...}NoDir`, 22 bytes:**

| Offset | Size | Type | Field |
|---:|---:|---|---|
| 0 | 4 | `u32 LE` | `entityId` |
| 4 | 4 | `f32 LE` | `posX` |
| 8 | 4 | `f32 LE` | `posY` |
| 12 | 4 | `f32 LE` | `posZ` |
| 16 | 3 | packed | `velocityXZ` |
| 19 | 2 | packed | `velocityY` |
| 21 | 1 | `u8` | `physics` |

#### 1.2.8 NoAlias variants without position — `0x1C – 0x1F`

When the entity hasn't moved (or position is being skipped to save bytes), the 12 position bytes vanish.

**`msg_id 0x1C` — `NoAliasNoPosYawPitchRoll`, 13 bytes:**

| Offset | Size | Type | Field |
|---:|---:|---|---|
| 0 | 4 | `u32 LE` | `entityId` |
| 4 | 3 | packed | `velocityXZ` |
| 7 | 2 | packed | `velocityY` |
| 9 | 1 | `u8` | `physics` |
| 10 | 1 | `i8` | `yaw` |
| 11 | 1 | `i8` | `pitch` |
| 12 | 1 | `i8` | `roll` |

**`msg_id 0x1D` — `NoAliasNoPosYawPitch`, 12 bytes:** drops `roll`.

**`msg_id 0x1E` — `NoAliasNoPosYaw`, 11 bytes:** drops `roll` and `pitch`.

**`msg_id 0x1F` — `NoAliasNoPosNoDir`, 10 bytes:** drops all direction bytes.

| Offset | Size | Type | Field |
|---:|---:|---|---|
| 0 | 4 | `u32 LE` | `entityId` |
| 4 | 3 | packed | `velocityXZ` |
| 7 | 2 | packed | `velocityY` |
| 9 | 1 | `u8` | `physics` |

#### 1.2.9 Alias variants with position — `0x20 – 0x2B`

The 4-byte `entityId` is replaced with a 1-byte `alias`. The alias is an index into the client's viewport alias table at `ServerConnection + 0xb7c`, which resolves to a full entity ID via `entityID = *(int32*)(this + alias * 4 + 0xb7c)`. The 0xFF value reserved by `CREATE_ENTITY` (§see Mercury chapter §1.10.5) means "no alias assigned" and forces the wire format to a NoAlias variant for that entity.

**`msg_ids 0x20 / 0x24 / 0x28` — `Alias{FullPos,OnChunk,OnGround}YawPitchRoll`, 22 bytes:**

| Offset | Size | Type | Field |
|---:|---:|---|---|
| 0 | 1 | `u8` | `alias` |
| 1 | 4 | `f32 LE` | `posX` |
| 5 | 4 | `f32 LE` | `posY` |
| 9 | 4 | `f32 LE` | `posZ` |
| 13 | 3 | packed | `velocityXZ` |
| 16 | 2 | packed | `velocityY` |
| 18 | 1 | `u8` | `physics` |
| 19 | 1 | `i8` | `yaw` |
| 20 | 1 | `i8` | `pitch` |
| 21 | 1 | `i8` | `roll` |

**`msg_ids 0x21 / 0x25 / 0x29` — `Alias{...}YawPitch`, 21 bytes:** drops `roll`.

**`msg_ids 0x22 / 0x26 / 0x2A` — `Alias{...}Yaw`, 20 bytes:** drops `roll` and `pitch`.

**`msg_ids 0x23 / 0x27 / 0x2B` — `Alias{...}NoDir`, 19 bytes:** drops all direction bytes.

| Offset | Size | Type | Field |
|---:|---:|---|---|
| 0 | 1 | `u8` | `alias` |
| 1 | 4 | `f32 LE` | `posX` |
| 5 | 4 | `f32 LE` | `posY` |
| 9 | 4 | `f32 LE` | `posZ` |
| 13 | 3 | packed | `velocityXZ` |
| 16 | 2 | packed | `velocityY` |
| 18 | 1 | `u8` | `physics` |

#### 1.2.10 Alias variants without position — `0x2C – 0x2F`

The smallest variants in the family. `0x2F` at 7 bytes is the floor of the compression scheme.

**`msg_id 0x2C` — `AliasNoPosYawPitchRoll`, 10 bytes:**

| Offset | Size | Type | Field |
|---:|---:|---|---|
| 0 | 1 | `u8` | `alias` |
| 1 | 3 | packed | `velocityXZ` |
| 4 | 2 | packed | `velocityY` |
| 6 | 1 | `u8` | `physics` |
| 7 | 1 | `i8` | `yaw` |
| 8 | 1 | `i8` | `pitch` |
| 9 | 1 | `i8` | `roll` |

**`msg_id 0x2D` — `AliasNoPosYawPitch`, 9 bytes:** drops `roll`.

**`msg_id 0x2E` — `AliasNoPosYaw`, 8 bytes:** drops `roll` and `pitch`.

**`msg_id 0x2F` — `AliasNoPosNoDir`, 7 bytes:** the smallest variant.

| Offset | Size | Type | Field |
|---:|---:|---|---|
| 0 | 1 | `u8` | `alias` |
| 1 | 3 | packed | `velocityXZ` |
| 4 | 2 | packed | `velocityY` |
| 6 | 1 | `u8` | `physics` |

#### 1.2.11 Direction option ordering invariant

Within each `(alias, position)` pair, the four direction variants always appear in the order `YPR / YP / Y / NoDir` as the `msg_id` low 2 bits increase. So `0x10` is `NoAlias + FullPos + YPR`, `0x11` is `NoAlias + FullPos + YP`, `0x12` is `NoAlias + FullPos + Y`, `0x13` is `NoAlias + FullPos + NoDir`. This ordering is consistent across all 8 `(alias, position)` rows.

#### 1.2.12 Channel choice

Per `space-viewport-wire-formats.md` §"UPDATE_AVATAR variants" and the canonical-variant decompile evidence, the server's `unreliable_movement_update` config flag controls whether `UPDATE_AVATAR` is emitted on the reliable Mercury channel (default) or the unreliable channel. In SGW the flag is typically true for AoI position spam to avoid retransmission overhead. This is a server-side configuration, not a wire-format property — the bytes are identical either way.

### 1.3 `detailedPosition` — full-precision non-controlled-entity snap

The full-precision sibling to `forcedPosition`. Carries `entityId`, position, velocity, and rotation as full `f32` values plus a 1-byte physics-mode field — but unlike `forcedPosition`, it does *not* carry `spaceId` or `vehicleId`. The omitted fields are preserved from the entity's current state.

**Does not work on client-controlled entities.** Per `position-movement-wire-formats.md` §"detailedPosition / Handler Behavior" the message is rejected if the entity is client-controlled — use `forcedPosition` (§1.4) for those.

| Property | Value |
|---|---|
| Message ID | `0x30` |
| Length type | `CONSTANT_LENGTH = 41` |
| Payload size | 41 bytes |
| Handler | `FUN_00dd9e00` at `ghidra://SGW.exe@0x00dd9e00` |

**Wire layout** per `position-movement-wire-formats.md` §"detailedPosition (msg_id 0x30, 41 bytes)":

| Offset | Size | Type | Field |
|---:|---:|---|---|
| 0 | 4 | `u32 LE` | `entityId` |
| 4 | 4 | `f32 LE` | `posX` |
| 8 | 4 | `f32 LE` | `posY` (vertical) |
| 12 | 4 | `f32 LE` | `posZ` |
| 16 | 4 | `f32 LE` | `velX` |
| 20 | 4 | `f32 LE` | `velY` |
| 24 | 4 | `f32 LE` | `velZ` |
| 28 | 4 | `f32 LE` | `roll` (radians; rotation about Z) |
| 32 | 4 | `f32 LE` | `pitch` (radians; rotation about X) |
| 36 | 4 | `f32 LE` | `yaw` (radians; rotation about Y) |
| 40 | 1 | `u8` | `physics` |

**Rotation order — `roll, pitch, yaw` on the wire.** Unlike `forcedPosition`'s offsets-36/40/44 swap (see §1.4), `detailedPosition` writes its rotation triplet in the conventional `roll, pitch, yaw` order. A reimplementation must use the message-specific rotation order — there is no protocol-wide convention.

**Handler behavior** per `position-movement-wire-formats.md` §"detailedPosition / Handler Behavior":

1. Resolves the entity via `FUN_00dd9d20` (SVID follow logic; rejects client-controlled entities).
2. If the position is for the entity we control, stores position in the entity record at offset `+0x10` (12 bytes).
3. Invokes the callback with full position/velocity/rotation data.

### 1.4 `forcedPosition` — authoritative client-position snap

The authoritative "you are here" message. Sent by the server when the client's position must be hard-set (world entry, gate travel, anti-cheat correction, teleport). Carries position, a previous-position reference vector (see §1.4.2 — *not* velocity, despite long-standing source-doc labels), full-precision rotation, and a physics-mode byte. Unlike `UPDATE_AVATAR` or `detailedPosition`, `forcedPosition` works on client-controlled entities — it is the only position message that does.

| Property | Value |
|---|---|
| Message ID | `0x31` |
| Length type | `CONSTANT_LENGTH = 49` |
| Payload size | 49 bytes |
| Handler | `ServerConnection_forcedPosition` at `ghidra://SGW.exe@0x00dd9ee0` |

#### 1.4.1 Wire layout

| Offset | Size | Type | Field |
|---:|---:|---|---|
| 0 | 4 | `u32 LE` | `entityId` |
| 4 | 4 | `u32 LE` | `spaceId` |
| 8 | 4 | `u32 LE` | `vehicleId` (0 = none) |
| 12 | 4 | `f32 LE` | `posX` |
| 16 | 4 | `f32 LE` | `posY` |
| 20 | 4 | `f32 LE` | `posZ` |
| 24 | 4 | `f32 LE` | `prevPosX` — previous-position reference (see §1.4.2) |
| 28 | 4 | `f32 LE` | `prevPosY` |
| 32 | 4 | `f32 LE` | `prevPosZ` |
| 36 | 4 | `f32 LE` | rotation slot A (see §1.4.3) |
| 40 | 4 | `f32 LE` | rotation slot B |
| 44 | 4 | `f32 LE` | rotation slot C |
| 48 | 1 | `u8` | `physics` (per-entity movement mode; `0x01` at world entry) |

#### 1.4.2 The 12 bytes at offsets 24-35 are a previous-position reference, not velocity

> [!NOTE] **Source-doc override: V5 docs label the 12 bytes at offsets 24-35 as `velocity` — that label is wrong.** Three V5 docs carry the mislabel (`position-movement-wire-formats.md` §"forcedPosition (msg_id 0x31, 49 bytes)", `entity-creation-wire-formats.md` §"5. FORCED_POSITION (0x31)", `space-viewport-wire-formats.md` §"FORCED_POSITION (0x31)"); this chapter overrides all three. The C++ server source comment "velocity (zeros at world entry)" inherits the same mislabel — the zeros at world entry exist because there is no prior position to delta from, not because the field semantically represents velocity.

**Evidence** (game-archaeology Ghidra pass, 2026-05-14):

- `ProcessForcedEntityPosition` at `ghidra://SGW.exe@0x00dd9ee0` executes `LEA EAX, [ESI+0x18]` (struct offset 24, wire offset 25) and **passes the 12-byte block as a pointer** to the internal `PackageAndSendEntityMove` helper as its `pOrientation` argument. This is the pointer-pass pattern used for positional data, not the individual-float load pattern used for orientation angles.
- The rotation block at struct offsets `+0x24/+0x28/+0x2c` (wire offsets 36/40/44) is loaded as three individual floats via `FLD` and passed as separate `flYaw, flPitch, flRoll` scalar arguments — a distinct memory-access pattern from the pointer-pass at offset 24.
- `PackageAndSendEntityMove` then executes:

  ```c
  local_18 = *(float *)pOrientation;
  fStack_14 = *(float *)((int)pOrientation+4);
  local_10 = *(float *)((int)pOrientation+8);
  *(undefined8 *)pPrevPos = *(undefined8 *)pOrientation;
  pPrevPos[2] = local_10;
  ```

  The `pOrientation` block is copied verbatim into `pPrevPos`, which aliases `pPosition == &(ESI+0xc)`. This is the BigWorld client's snapshot of the previous position, used by the client's delta-compression path when it retransmits the resulting move as an `addMove` for server reconciliation.

**Semantic clarification.** The 12 bytes at offsets 24-35 are the previous-position reference vector — the value the client uses as the "previous position" when re-emitting the move. At world entry the values are zeros because there is no prior position to delta from; after world entry they typically equal the entity's last-known position.

The exact semantic (`previous position used as delta-reference` vs `an orientation expressed as a local-frame direction vector`) is inferred from the `PackageAndSendEntityMove` pointer-pass + `pPrevPos` aliasing pattern. The previous-position-reference reading fits the BigWorld `addMove` delta-compression mechanism cleanly; the orientation-as-direction-vector reading does not have a corresponding consumer in the decompile. Confidence: high on the role (previous-position reference); the alternative interpretation is documented in case future evidence reframes it.

A reimplementation should emit zero bytes at offsets 24-35 for world-entry `forcedPosition`. For non-world-entry snaps (gate travel, teleport, anti-cheat), the conservative choice is to emit the entity's last-known position; emitting zeros is also safe because the client will re-emit an `addMove` with that delta as the previous position regardless of whether the value is meaningful.

#### 1.4.3 Rotation order is per call site, not protocol-wide

The handler at `0x00dd9ee0` reads the three floats at offsets 36/40/44 positionally — offset 36 → `param_1[9]`, offset 40 → `param_1[10]`, offset 44 → `param_1[11]` — and shuffles them as `addMove(yaw = param[11], pitch = param[10], roll = param[9])` (per `system-protocol-wire-formats.md` §"FORCED_POSITION (0x31) -- Rotation Order Evidence").

That positional read works correctly *only when the caller writes Y/Z swapped on the wire* — which the world-entry path does, but the standalone path does not by default. SGW emits `forcedPosition` from two distinct C++ call sites:

| Call site | C++ rotation emit | Wire byte order at offsets 36–47 |
|---|---|---|
| `client_handler.cpp:407-413` (world-entry, during `createCellPlayer`) | `rotX << rotZ << rotY` | `rotX, rotZ, rotY` (Y/Z swapped) |
| `client_handler.cpp:566-572` (standalone `ServerConnection::forcedPosition()`) | `rotation.x << rotation.y << rotation.z` | caller's responsibility |

The wire-byte names in §1.4.1 (`rotation slot A/B/C`) are deliberately neutral — they avoid both the misleading `rotX/rotY/rotZ` decompile-struct labels and the protocol-canonical `roll/pitch/yaw` labels, because neither set is true at both call sites.

For the world-entry path the wire bytes are `(rotation.x, rotation.z, rotation.y)`; the standalone path emits whatever the caller's `rotation` argument is in `(x, y, z)` order. A reimplementation must pick the rotation order per call site, not per protocol.

#### 1.4.4 Handler behavior

Per `position-movement-wire-formats.md` §"forcedPosition / Handler Behavior":

1. Looks up the entity in the entity map at `ServerConnection + 0xfac`.
2. Calls `ServerConnection::addMove` with `isForced = true` (echoes the position back to the server for reconciliation).
3. Invokes the callback at `this + 0x168` to notify the game layer.
4. Asserts `sentPhysics_[id] == args.physics`.

### 1.5 C→S counterpart — `avatarUpdate` (not server-emitted)

The C→S `avatarUpdate` family is not catalogued here because the bible chapter scope is server-to-client messages. The client emits its own movement via the entity-method dispatch path documented in `spec.protocol.mercury-wire-format` §1.8, not through the `UPDATE_AVATAR` `msg_id` range. The send-side path:

- `ServerConnection::addMove` at `ghidra://SGW.exe@0x00dd9330` — the entry point for client-side move emission. Called by the `forcedPosition` handler with `isForced = true` (after server snap) and by the client's per-frame movement code with `isForced = false` (steady-state).
- The C→S move format is a method-call (`msg_id | 0x80` or `msg_id | 0xC0`) carrying the move arguments, not a fixed-`msg_id` system message.

If the C→S avatarUpdate side surfaces in future RE work, this section will be expanded; for now, the canonical record for the C→S move flow lives in the entity-method-dispatch portion of `spec.protocol.mercury-wire-format` plus the message-catalog reference in `spec.protocol.message-catalog`.

### 1.6 Wire-format divergences from stock BigWorld 2.0.1

The position plane is mostly inherited from stock BigWorld; divergences are concentrated in the trailing `physics` byte and `forcedPosition`'s payload size.

| Surface | Stock BigWorld 2.0.1 | SGW |
|---|---|---|
| `forcedPosition` payload | 36 bytes (`entityID + spaceID + vehicleID + Position3D + Direction3D`) | 49 bytes — adds 12 bytes of previous-position reference (offsets 24-35) and 1 trailing physics byte (offset 48) |
| `forcedPosition` field at offsets 24-35 | (not present) | Previous-position reference vector (not velocity, despite source-doc labels — see §1.4.2) |
| `forcedPosition` rotation order | `roll, pitch, yaw` (`Direction3D` convention) | Per call site: world-entry writes `rotX, rotZ, rotY` (Y/Z swapped); standalone writes caller-supplied order |
| `detailedPosition` payload | (stock-BW analog full-precision) | 41 bytes — adds trailing `physics` byte (same SGW addition as `forcedPosition`) |
| `detailedPosition` rotation order | `roll, pitch, yaw` | `roll, pitch, yaw` (no swap — distinct from `forcedPosition`) |
| `UPDATE_AVATAR` family | Same 32-variant compression scheme | Same; SGW-specific trailing-byte semantics (physics mode rather than stock-BW reserved flags) |

The most subtle divergence is `forcedPosition`'s rotation order — the world-entry path's Y/Z swap is required for the handler's positional read at offsets 36/40/44 to produce correct yaw/pitch/roll. A reimplementation that copies the stock-BW `Direction3D` ordering will produce client-side rotation that is wrong by a 90-degree rotation about the X axis.

### 1.7 Source-of-truth crosswalk

One row per load-bearing claim. The "Primary V5 source" column is the canonical evidence; the "Secondary cross-check" disambiguates or cross-validates.

**§1.1–§1.2 The `UPDATE_AVATAR` family (`0x10 – 0x2F`):**

| Claim | Primary V5 source | Secondary cross-check |
|---|---|---|
| 32 variants in `msg_id 0x10 – 0x2F`, all `CONSTANT_LENGTH` | `position-movement-wire-formats.md` §"avatarUpdate Messages (msg_id 0x10-0x2F)" | `space-viewport-wire-formats.md` §"UPDATE_AVATAR variants (0x10 - 0x2F)" and §"All 32 Variant Sizes" |
| 2×4×4 variant matrix (alias × position × direction) | `position-movement-wire-formats.md` §"Encoding Dimensions" | `space-viewport-wire-formats.md` §"All 32 Variant Sizes" |
| `FullPos / OnChunk / OnGround` share an identical wire layout; differ in handler Y interpretation | `position-movement-wire-formats.md` §"Position: 3 x float32 (12 bytes) or absent" | Handler decompiles at `0x00ddb0c0`, `0x00ddb220`, `0x00ddb830` |
| `OnChunk` / `OnGround` substitute `DAT_019d1a44` for Y | `position-movement-wire-formats.md` §"avatarUpdate Messages" | Ghidra anchors above |
| Velocity = 5 bytes packed (3 bytes XZ + 2 bytes Y) | `position-movement-wire-formats.md` §"Velocity: 5 bytes (always present)" | `space-viewport-wire-formats.md` §"UPDATE_AVATAR_NO_ALIAS_FULL_POS_YAW_PITCH_ROLL (0x10)" — `packXYZ` |
| `packXYZ` bit layout (X/Z 11-bit mantissa, Y 15-bit packed) | `position-movement-wire-formats.md` §"Velocity Compression" | `FUN_00de1850` at `ghidra://SGW.exe@0x00de1850` |
| Direction quantization (`u8` over 256 steps, constant `0.024543693 = 2π/256`) | `position-movement-wire-formats.md` §"Direction" | `space-viewport-wire-formats.md` §"UPDATE_AVATAR_NO_ALIAS_FULL_POS_YAW_PITCH_ROLL"; `DAT_01816a84` |
| Direction wire order is `yaw, pitch, roll`; encoded sources are `rotation.y, rotation.x, rotation.z` | `space-viewport-wire-formats.md` §"UPDATE_AVATAR_NO_ALIAS_FULL_POS_YAW_PITCH_ROLL" — C++ source `client_handler.cpp:548-555` | — |
| Alias resolves via `entityId = *(int32*)(this + alias*4 + 0xb7c)` | `position-movement-wire-formats.md` §"Alias Variants (entity alias = 1 byte, msg_id 0x20-0x2F)" | — |
| Per-variant byte sizes (7-25) | `position-movement-wire-formats.md` §"Size Summary Table" | `space-viewport-wire-formats.md` §"All 32 Variant Sizes" |
| Unreliable-channel emission via `unreliable_movement_update` | `space-viewport-wire-formats.md` §"UPDATE_AVATAR variants" | — |
| Does not work on client-controlled entities | `position-movement-wire-formats.md` §"forcedPosition" / §"detailedPosition" symmetric callouts | — |

**§1.3 `detailedPosition` (`0x30`):**

| Claim | Primary V5 source | Secondary cross-check |
|---|---|---|
| `CONSTANT_LENGTH = 41` | `position-movement-wire-formats.md` §"detailedPosition (msg_id 0x30, 41 bytes)" | `space-viewport-wire-formats.md` §"DETAILED_POSITION (0x30)" |
| Wire layout (no spaceId / vehicleId; rotation in `roll, pitch, yaw` order) | `position-movement-wire-formats.md` §"detailedPosition" wire table | `space-viewport-wire-formats.md` §"DETAILED_POSITION (0x30)" |
| Rejected on client-controlled entities | `position-movement-wire-formats.md` §"detailedPosition / Handler Behavior" | — |
| Trailing `physics` byte = same per-entity field as `forcedPosition` | `position-movement-wire-formats.md` §"detailedPosition" | — |

**§1.4 `forcedPosition` (`0x31`):**

| Claim | Primary V5 source | Secondary cross-check |
|---|---|---|
| `CONSTANT_LENGTH = 49` | `position-movement-wire-formats.md` §"forcedPosition (msg_id 0x31, 49 bytes)" | `space-viewport-wire-formats.md` §"FORCED_POSITION (0x31)"; `entity-creation-wire-formats.md` §"5. FORCED_POSITION (0x31)" |
| Offsets 24-35 are a previous-position reference vector, **not velocity** | game-archaeology Ghidra pass on `ProcessForcedEntityPosition` at `ghidra://SGW.exe@0x00dd9ee0` (2026-05-14): `LEA EAX, [ESI+0x18]` pointer-pass to `PackageAndSendEntityMove`, copied verbatim into `pPrevPos` aliased to `pPosition` | three V5 docs (`position-movement-wire-formats.md`, `entity-creation-wire-formats.md`, `space-viewport-wire-formats.md`) all carry the legacy "velocity" label; chapter §1.4.2 source-doc-override callout |
| Rotation order is per call site (world-entry: `rotX, rotZ, rotY`; standalone: caller's responsibility) | `entity-creation-wire-formats.md` §"5. FORCED_POSITION (0x31)" (two C++ call sites) | `system-protocol-wire-formats.md` §"FORCED_POSITION (0x31) -- Rotation Order Evidence" (addMove `param_1[9/10/11]` mapping) |
| Trailing `physics` byte (offset 48) = per-entity movement mode, value `0x01` at world entry | `position-movement-wire-formats.md` §"forcedPosition" Field Notes | `entity-creation-wire-formats.md` C++ emit `(uint8_t)0x01`; assertion `sentPhysics_[args.id] == args.physics` in handler |
| Works on client-controlled entities (the only position message that does) | `position-movement-wire-formats.md` §"forcedPosition / Handler Behavior" — `addMove` with `isForced = true` | — |

**§1.6 Divergences from stock BigWorld:**

| Claim | Primary V5 source | Secondary cross-check |
|---|---|---|
| Stock BW `forcedPosition` is 36 bytes (entityID + spaceID + vehicleID + Position3D + Direction3D) | Mercury chapter `spec.protocol.mercury-wire-format` §1.13 | `external/BigWorld-2.0.1/src/lib/connection/baseapp_ext_interface.hpp` |
| Trailing `physics` byte is an SGW addition for both `forcedPosition` and `detailedPosition` | `position-movement-wire-formats.md` §"forcedPosition" / §"detailedPosition" | — |

---

## Section 2 — Client findings

N/A — pending Section 1 review. The client's expectation of the position-update plane is implicit in its handler functions in the `0x00ddb???` / `0x00ddc???` / `0x00de1???` ranges, already cited in Section 1. A future Section 2 will catalogue what the *client-side configuration* (`game/sgw/Working/SGWGame/Config/*.ini`) declares about movement filters, prediction tuning, and interpolation parameters that interact with the wire format.

---

## Section 3 — Deprecated server

N/A — pending Section 1 review. `deprecated/cpp/src/baseapp/mercury/sgw/client_handler.cpp` is the C++ source for the legacy implementation; the two `forcedPosition` call sites (`:407-413` world-entry, `:566-572` standalone) and the `UPDATE_AVATAR` canonical-variant emit (`:548-556`) are cited in Section 1. Section 3 will reconstruct the deprecated server's full per-call-site emit logic when authored.

---

## Section 4 — Expected implementation in Rust

N/A — pending Section 1 review. Will name the Rust symbols that must encode each variant on the server side, using the no-line-numbers rule (`cimmeria-services::cell::move_emit::AvatarUpdate::serialize`, `cimmeria-mercury::wire::position::ForcedPosition::encode`, etc.).

---

## Section 5 — Actual implementation in Rust

N/A — pending Section 1 review.
