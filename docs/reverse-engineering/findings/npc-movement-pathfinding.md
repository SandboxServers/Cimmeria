# NPC Movement and Pathfinding

> **Author**: W-path (Session 4 V5 Campaign)
> **Date**: 2026-05-13
> **Binary**: SGW.exe 32-bit x86 PE (MSVC 8.0 / VC80)
> **Cross-references**: [`npc-ai-state-machine.md`](npc-ai-state-machine.md), [`position-movement-wire-formats.md`](position-movement-wire-formats.md), [`cover-system.md`](cover-system.md)

---

## Overview

NPC movement in Stargate Worlds is split across three layers: the **wire transport** (BigWorld Mercury avatarUpdate), the **CME event bus** (onRemoteEntityMove signals), and the **UE3 rendering layer** (GameEntityBase::ApplyTransform + path visualization actors). The server drives movement; the client receives position updates and renders them with optional interpolation. This document covers the client-side receive path, the 7-state AI movement FSM, coordinate conversion, client-side interpolation, and the leash-back-to-spawn mechanism. Implementation gaps on the Cimmeria server side are flagged throughout.

---

## 1. Wire Transport Layer

NPC movement does **not** use dedicated CME emitter functions. No `EmitNetOut_*Move*` or `EmitNetOut_*Goto*` functions exist in the binary. Instead, the server streams positions through BigWorld's standard Mercury protocol:

- **avatarUpdate variants** (msg IDs `0x10`–`0x2F`): 32 variants encoding position + velocity + rotation at various precision levels. Documented in full in [`position-movement-wire-formats.md`](position-movement-wire-formats.md).
- **detailedPosition** (`0x30`): full-precision 12-byte position update.
- **forcedPosition** (`0x31`): authoritative position override (used for teleport / leash snap).

The entry point on the client is `EntityManager::onEntityMoveWithError` (`0x00dd1650`), which:
1. Reads raw BigWorld coordinates (meters, BW axis order).
2. Multiplies all position and velocity components by `BW_TO_UE3_SCALE = 100.0f` (`DAT_018cad90`).
3. Swaps axes: `UE3_X = BW_Z × 100`, `UE3_Y = BW_X × 100`, `UE3_Z = BW_Y × 100`.
4. Detects "use current" sentinel (`FLT_MAX / infinity` at `DAT_019d1a44`) — if a component equals the sentinel, the entity's current value is preserved.
5. Delegates to `GameEntityBase::ApplyTransform` (`0x00e68a30`).

---

## 2. Client-Side Position Application and Interpolation

`GameEntityBase::ApplyTransform` (`0x00e68a30`) routes incoming positions through one of three paths:

### Path A — Direct Write (forced/teleport)
When `param_8 != 0` (force flag set — used by `forcedPosition` `0x31` and leash snaps):
- Calls a pre-hook via actor vtable `+0x170`.
- Writes `Location` and `Rotation` directly to the UE3 `ABigWorldEntity` actor fields.
- Calls a post-hook via vtable `+0x174`.
- No interpolation; position is instant.

### Path B — Vehicle Interpolator
When `entity+0xe4 != 0` (vehicle interpolator object is present):
- Calls the vehicle interpolator via vtable slot `+0x10c` with a normalized time parameter.
- Used for mounted / vehicle entities; not relevant for NPCs on foot.

### Path C — Physics Interpolator (smooth NPC movement)
When entity field `+0x1d0` holds a physics interpolator object:
- `ApplyTransform` calls `EntityInterpolatorUpdate` (`0x00e69690`).
- `EntityInterpolatorUpdate` calls `FUN_0049ffb0` then dispatches via `(*interpolator + 0xe8)`.
- This provides frame-rate-independent smooth interpolation between received positions.
- The interpolator is the standard BigWorld 1.9.1 entity interpolator (smooths movement over multiple game frames between 200ms BW update packets).

---

## 3. AI Movement State Machine

The client renders AI movement state received from the server via the `onRemoteEntityMove` CME event signal. The AI movement FSM has **7 states**, dispatched by a jump table at `0x00dec018` inside `MovementTypeSwitch` (`FUN_00deb660`):

| Index | State Name | Description |
|-------|-----------|-------------|
| 0 | CoverAdvance | Moving toward a cover node (see [`cover-system.md`](cover-system.md)) |
| 1 | CombatAdvance | Moving toward a combat target |
| 2 | Leash | Returning to spawn point after target left leash radius |
| 3 | Patrol | Following a defined patrol route |
| 4 | Follow | Following a player or squad leader |
| 5 | Wander | Random idle wandering |
| 6 | Avoid | Obstacle / collision avoidance maneuver |

**Evidence**: String cross-references in `FUN_00deb660` (`0x00deb660`); jump table at `0x00dec018`; strings "is moving to cover", "is making a combat advance", "is leashing" found in the function body. Function is 610 instructions (body `0x00deb660`–`0x00dec015`); decompilation times out and was examined via `read_memory` + `get_assembly_context`.

### Registered Callbacks

| Function | Address | Registration |
|----------|---------|-------------|
| `TickUpdate` | `0x00dedf30` | Called every game tick — advances entity along waypath |
| `onPositionUpdate` | `0x00deaaf0` | BigWorld position update signal |
| `MovementTypeSwitch` | `0x00deb660` | Fires when server sends new movementType |
| `PathDestroy` | `0x00dec040` | Fires when server ends a waypath |
| `RegionUpdate` | `0x00df3550` | BigWorld space/region change |

Both `SGWBeing_RegisterCallbacks` (`0x00df3ab0`) and `SGWMob_RegisterCallbacks` (`0x00df3cc0`) register the **identical set** of these callbacks — confirmed by decompiling both. SGWBeing and SGWMob share a common movement implementation.

---

## 4. Path Visualization — onPositionUpdate and PathDestroy

### onPositionUpdate (`0x00deaaf0`)

This function does more than update a position. When a BigWorld `onPositionUpdate` event fires with a new waypath:
- It **allocates new UE3 actors** for each waypoint in the path (path visualization nodes).
- Actor positions are set using the same BW→UE3 coordinate conversion.
- Path visualization actors are named by a convention matched by `PathDestroy`.

This is a debug/editor visualization system used by the client to display NPC intended paths. It is not part of the gameplay-visible NPC motion — motion uses ApplyTransform.

### PathDestroy (`0x00dec040`)

When the server signals path completion or cancellation:
- Iterates the entity's path actor list.
- Uses `wcsicmp` (wide-string case-insensitive compare) to match actor names.
- Destroys matching actors.

---

## 5. Coordinate Conversion Reference

| Constant | Address | Value | Purpose |
|----------|---------|-------|---------|
| `BW_TO_UE3_SCALE` | `0x018cad90` | `100.0f` | BW meters → UE3 centimeters |
| `RAD_TO_URU` | `0x018cafcc` | `10430.378f` | Radians → UE3 rotation units (65536/2π) |
| `NEG_RAD_TO_URU` | `0x018cafd0` | `-10430.378f` | Negated (axis handedness) |
| Position sentinel | `DAT_019d1a44` | FLT_MAX / ∞ | "preserve current component" |

Axis swap (confirmed in `EntityManager::onEntityMoveWithError` `0x00dd1650`):
```
UE3_X = BW_Z × 100.0
UE3_Y = BW_X × 100.0
UE3_Z = BW_Y × 100.0
```

---

## 6. Leash-Back-to-Spawn Mechanics

When an NPC's target exits `LEASH_DISTANCE` from the NPC's spawn point, the server transitions the NPC to `AiState::Leashing`.

### What the binary expects (confirmed from client callback registration and string evidence)

The client expects `movementType = 2` (Leash state) on the `onRemoteEntityMove` CME signal, followed by a waypath back to spawn. `MovementTypeSwitch` case 2 ("is leashing") would trigger the Leash animation/path-following on the client side.

### What Cimmeria currently sends

`npc_ai_leash()` in `crates/services/src/cell/service/npc_ai.rs`:
1. Snaps the NPC to spawn position **instantly** (direct field write, no pathfinding).
2. Restores health to max.
3. Resets `ai_state` to `AiState::Idle`.
4. Sends `onStatUpdate` (method 20) and `onStateFieldUpdate` (method 19).
5. Does **not** send `movementType=2` or a waypath.

**Gap**: The client never sees the leash animation. From the player's perspective, the NPC teleports to its spawn point. The correct behavior is:
1. Send `onRemoteEntityMove` with `movementType=2` + waypath from current NPC position to spawn.
2. Move the NPC along that path over time (with `npc_movement_tick`).
3. When spawn position is reached, send `movementType` reset and health restore.

---

## 7. Combat Advance — Server-Side Movement Emission Gap

`npc_movement_tick()` in `crates/services/src/cell/service/ticks/npc_movement.rs`:
- Moves NPCs along `nav_path` at `move_speed` per tick (100ms tick; velocity = `move_speed × 10.0` for per-second scaling).
- Calls `space_mgr.update_entity_position()` which propagates position to witnesses via AoI `EntityMoved`.
- Sets NPC yaw via `atan2(dx, dz)`.
- Does **not** send `onRemoteEntityMove` with `movementType=1` (CombatAdvance) and the computed waypath.

**Gap**: Witnesses receive raw position updates (avatarUpdate wire) but the client never receives the `movementType=1 + waypath` payload that would trigger the `CombatAdvance` AI animation state. NPCs move to the right place but may display idle animation rather than a combat approach animation.

---

## 8. Open Questions

1. **onRemoteEntityMove payload structure**: Does it carry the full waypath array or only the next waypoint? Wire capture needed to confirm.
2. **movementType=2 wire payload**: What exact CME fields accompany the Leash signal? Field names (destination, waypointCount, etc.) are not yet recovered from the binary.
3. **Path actor to waypoint correspondence**: Do path visualization actors created in `onPositionUpdate` map 1:1 to server `nav_path` waypoints?
4. **Cover system intersection**: How do `CoverAdvance` (state 0) and `CombatAdvance` (state 1) transitions interact? Cross-reference `W-cover` findings in [`cover-system.md`](cover-system.md).
5. **EntityInterpolatorUpdate multi-tick**: Does the physics interpolator at `entity+0x1d0` smooth over a configurable number of frames, or is it a fixed BigWorld 200ms window?

---

## 9. Implementation Recommendations for Cimmeria

### Short-term (behavior correctness)
- **Fix leash**: Instead of instant snap, pathfind from current NPC position to spawn, emit `onRemoteEntityMove` with `movementType=2` + waypath, then move along path. Health restore fires on arrival.
- **Fix combat advance**: After `space_mgr.find_path()` succeeds in `npc_ai_fight()`, emit `onRemoteEntityMove` with `movementType=1` + path waypoints.

### Longer-term (fidelity)
- Recover the exact `onRemoteEntityMove` CME payload schema (field names, types) from the binary — likely in the SGWBeing or SGWMob `.def` entity definition.
- Implement `PathDestroy` equivalent: when an NPC's path is cleared server-side, notify clients.

---

## 10. Address Reference (quick lookup)

| Address | Name | Notes |
|---------|------|-------|
| `0x00dd1650` | `EntityManager::onEntityMoveWithError` | Wire → UE3 conversion entry point |
| `0x00dd19e0` | `GameEntityManager_UpdateControlledEntityTransform` | Player-controlled entity transform |
| `0x00deb660` | `MovementTypeSwitch` | 7-state AI FSM; jump table at `0x00dec018` |
| `0x00dec018` | AI movement jump table | Cases 0–6 for CoverAdvance…Avoid |
| `0x00deaaf0` | `onPositionUpdate` | Creates UE3 path-visualization actors |
| `0x00dec040` | `PathDestroy` | Destroys path actors by wcsicmp name |
| `0x00dec6d0` | `onSquadList` | Squad-member path receiver |
| `0x00dec9e0` | `onBigWorldTimeComplete` | BigWorld time-sync callback |
| `0x00dedf30` | `TickUpdate` | Per-tick movement advance |
| `0x00def320` | `ApplyTargetChange` | Target acquisition / heading |
| `0x00df08c0` | `TargetIDReceiver` | CME NetIn target-id event |
| `0x00df3550` | `RegionUpdate` | BW space/region change |
| `0x00df3ab0` | `SGWBeing_RegisterCallbacks` | Movement + AI callback registrar |
| `0x00df3cc0` | `SGWMob_RegisterCallbacks` | Identical to SGWBeing variant |
| `0x00e68a30` | `GameEntityBase::ApplyTransform` | Position → UE3 actor write + interpolation |
| `0x00e688c0` | `EntityVisibilityManager` | Distance-cull / LOD |
| `0x00e69690` | `EntityInterpolatorUpdate` | Physics interpolator dispatch |
| `0x018cad90` | `BW_TO_UE3_SCALE` | `100.0f` constant |
| `0x019d1a44` | Position sentinel | FLT_MAX = "use current component" |
