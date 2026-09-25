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
4. Detects "use current" sentinel (`-13000.0f` at `DAT_019d1a44`; an earlier revision said `FLT_MAX`, corrected 2026-09-24) — if a component equals the sentinel, the entity's current value is preserved.
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

> **Superseded (2026-09-25, NA10): this section is not an NPC animation state machine.** `0x00deb660` is registered through `CallbackImpl<Event_NetIn_onShowPath>` on `GameProxyPlayer`. It is the GM path visualiser for `SGWGmPlayer.onShowPath(aEntityId, aMovementType, aPath)`. The movement type only picks the debug text and colour. See [§11](#11-correction-2026-09-25-the-client-has-no-movement-type-receiver). The case table below is still correct as a decode of `EMobMovementType`.

The client renders AI movement state received from the server via the `onRemoteEntityMove` CME event signal. The AI movement FSM has **7 states**, dispatched by a jump table at `0x00dec018` inside `MovementTypeSwitch` (`FUN_00deb660`):

| Index | State Name | Description |
|-------|-----------|-------------|
| 0 | CoverAdvance | Moving toward a cover node (see [`cover-system.md`](cover-system.md)) |
| 1 | CombatAdvance | Moving toward a combat target |
| 2 | Patrol | Following a defined patrol route |
| 3 | Follow | Following a player or squad leader |
| 4 | Wander | Random idle wandering |
| 5 | Leash | Returning to spawn point after target left leash radius |
| 6 | Avoid | Obstacle / collision avoidance maneuver |

> **Correction (2026-09-24):** an earlier revision of this table put Leash at 2 and shifted Patrol, Follow and Wander up by one, apparently from the order of the debug strings in `.rdata`. The jump table says otherwise. `MovementTypeSwitch` switches on the raw `aMovementType` byte (`MOVZX EBP, byte ...` at `0x00deb844`, `CMP EBP, 6` at `0x00deba58`, `JMP [EBP*4 + 0x00dec018]` at `0x00deba69`). The table entries are case 0 → `0x00deba70` ("is moving to cover"), 1 → `0x00debaa4` ("is making a combat advance"), 2 → `0x00debb04` (the patrol string, wide, at `0x019d2d5c`), 3 → `0x00debb38` ("is following"), 4 → `0x00debb64` ("is wandering"), **5 → `0x00debad0` ("is leashing", `0x019d2d2c`)**, 6 → `0x00debb95` ("is avoiding"). This matches `EMobMovementType` in `entities/defs/enumerations.xml` (`MOB_MOVEMENT_Leash = 5`, `MOB_MOVEMENT_Patrol = 2`) and the server's `MobMovementType::Leash` wire byte.

**Evidence**: String cross-references in `FUN_00deb660` (`0x00deb660`); jump table at `0x00dec018`; strings "is moving to cover", "is making a combat advance", "is leashing" found in the function body. Function is 610 instructions (body `0x00deb660`–`0x00dec015`); decompilation times out and was examined via `read_memory` + `get_assembly_context`.

### Registered Callbacks

| Function | Address | Registration |
|----------|---------|-------------|
| `TickUpdate` | `0x00dedf30` | Called every game tick — advances entity along waypath |
| `onPositionUpdate` | `0x00deaaf0` | BigWorld position update signal |
| `MovementTypeSwitch` | `0x00deb660` | **Corrected 2026-09-25:** `Event_NetIn_onShowPath` (GM path debug), not a movement-type receiver |
| `PathDestroy` | `0x00dec040` | **Corrected 2026-09-25:** `Event_NetIn_onDisableShowPath` (GM path debug) |
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
| Position sentinel | `DAT_019d1a44` | `-13000.0f` (bytes `00 20 4b c6`) | "preserve current component" |

Axis swap (confirmed in `EntityManager::onEntityMoveWithError` `0x00dd1650`):
```
UE3_X = BW_Z × 100.0
UE3_Y = BW_X × 100.0
UE3_Z = BW_Y × 100.0
```

---

## 6. Leash-Back-to-Spawn Mechanics

> **Superseded (2026-09-25):** no `movementType` reaches the client for a leash or any other NPC state. See [§11](#11-correction-2026-09-25-the-client-has-no-movement-type-receiver). A walk home is shown by position and velocity alone.

When an NPC's target exits `LEASH_DISTANCE` from the NPC's spawn point, the server transitions the NPC to `AiState::Leashing`.

### What the binary expects (confirmed from client callback registration and string evidence)

The client expects `movementType = 5` (Leash state, corrected 2026-09-24 from 2; see the §3 correction) on the `onRemoteEntityMove` CME signal, followed by a waypath back to spawn. `MovementTypeSwitch` case 5 ("is leashing") would trigger the Leash animation/path-following on the client side.

### What Cimmeria currently sends

`npc_ai_leash()` in `crates/services/src/cell/service/npc_ai.rs`:
1. Snaps the NPC to spawn position **instantly** (direct field write, no pathfinding).
2. Restores health to max.
3. Resets `ai_state` to `AiState::Idle`.
4. Sends `onStatUpdate` (method 20) and `onStateFieldUpdate` (method 19).
5. Does **not** send `movementType=5` or a waypath.

**Gap**: The client never sees the leash animation. From the player's perspective, the NPC teleports to its spawn point. The correct behavior is:
1. Send `onRemoteEntityMove` with `movementType=5` + waypath from current NPC position to spawn.
2. Move the NPC along that path over time (with `npc_movement_tick`).
3. When spawn position is reached, send `movementType` reset and health restore.

---

## 7. Combat Advance — Server-Side Movement Emission Gap

> **Superseded (2026-09-25):** there is no `movementType=1 + waypath` payload to send; see [§11](#11-correction-2026-09-25-the-client-has-no-movement-type-receiver).

`npc_movement_tick()` in `crates/services/src/cell/service/ticks/npc_movement.rs`:
- Moves NPCs along `nav_path` at `move_speed` per tick (100ms tick; velocity = `move_speed × 10.0` for per-second scaling).
- Calls `space_mgr.update_entity_position()` which propagates position to witnesses via AoI `EntityMoved`.
- Sets NPC yaw via `atan2(dx, dz)`.
- Does **not** send `onRemoteEntityMove` with `movementType=1` (CombatAdvance) and the computed waypath.

**Gap**: Witnesses receive raw position updates (avatarUpdate wire) but the client never receives the `movementType=1 + waypath` payload that would trigger the `CombatAdvance` AI animation state. NPCs move to the right place but may display idle animation rather than a combat approach animation.

---

## 8. Open Questions

1. **onRemoteEntityMove payload structure**: Does it carry the full waypath array or only the next waypoint? Wire capture needed to confirm.
2. **movementType=5 (Leash) wire payload**: What exact CME fields accompany the Leash signal? Field names (destination, waypointCount, etc.) are not yet recovered from the binary.
3. **Path actor to waypoint correspondence**: Do path visualization actors created in `onPositionUpdate` map 1:1 to server `nav_path` waypoints?
4. **Cover system intersection**: How do `CoverAdvance` (state 0) and `CombatAdvance` (state 1) transitions interact? Cross-reference `W-cover` findings in [`cover-system.md`](cover-system.md).
5. **EntityInterpolatorUpdate multi-tick**: Does the physics interpolator at `entity+0x1d0` smooth over a configurable number of frames, or is it a fixed BigWorld 200ms window?

---

## 9. Implementation Recommendations for Cimmeria

> **Superseded (2026-09-25):** the `onRemoteEntityMove` / `movementType` recommendations below rest on the §3 misreading. See [§11](#11-correction-2026-09-25-the-client-has-no-movement-type-receiver).

### Short-term (behavior correctness)
- **Fix leash**: Instead of instant snap, pathfind from current NPC position to spawn, emit `onRemoteEntityMove` with `movementType=5` (Leash; `2` is Patrol, see the §3 correction) + waypath, then move along path. Health restore fires on arrival.
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
| `0x00deb660` | `GameProxyPlayer` `onShowPath` handler (was "MovementTypeSwitch") | GM path debug; text/colour per `aMovementType`, jump table at `0x00dec018` |
| `0x00dec018` | `onShowPath` label jump table | Cases 0–6 for CoverAdvance…Avoid |
| `0x00deaaf0` | `GameProxyPlayer` `onShowCommandWaypoints` handler (was "onPositionUpdate") | Creates UE3 path-visualization actors |
| `0x00dec040` | `GameProxyPlayer` `onDisableShowPath` handler (was "PathDestroy") | Destroys path actors by wcsicmp name |
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
| `0x019d1a44` | Position sentinel | `-13000.0f` = "use current component" |

---

## 11. Correction (2026-09-25): the client has no movement-type receiver

Found by NA10 of the NPC AI restoration while choosing what to send when an NPC stops. It supersedes §3, §6, §7 and §9 where they talk about a server-sent movement type.

**The handler at `0x00deb660` is the GM path visualiser.**

- It is registered through `FUN_00dfaf20`, which constructs `CME::EventSignal::MemberCallback<..., GameProxyPlayer, ..., Event_NetIn_onShowPath>` (`FUN_00df7690`).
- Its arguments are read by name: `aPath`, `aMovementType` and `aEntityId` (ASCII at `0x019d2c70`, `0x019d2c78` and `0x019d2c88`). Those are the three arguments of `SGWGmPlayer.onShowPath` in `entities/defs/SGWGmPlayer.def`.
- Each jump-table case only picks a wide label ("Entity: %d is moving to cover", "... is making a combat advance", and so on) and a debug colour. It then spawns and positions path-marker actors.
- The two sibling registrations are the same kind: `0x00deaaf0` is `Event_NetIn_onShowCommandWaypoints` (`FUN_00df7610`) and `0x00dec040` is `Event_NetIn_onDisableShowPath` (`FUN_00df7710`). All three are `GameProxyPlayer` member callbacks. The registrars `0x00df3ab0` and `0x00df3cc0` were previously named `SGWBeing_/SGWMob_RegisterCallbacks`.

**`setMovementType` only goes from client to server.** It is an `<Exposed/>` cell method in `SGWBeing.def`. The binary has `Event_NetOut_SetMovementType` and no `Event_NetIn_*` twin. No `<ClientMethods>` entry anywhere carries a movement type.

**What Cimmeria was actually sending.** `broadcast_movement_type` sent `WitnessEntityMethod { method_index: 1, args: [kind] }`. For a witness, client method 1 of every NPC entity type is `onSequence` (`SGWSpawnableEntity`, `client_methods::spawnable_entity::ON_SEQUENCE`). That is the same method attack animations use. So every Fighting, Patrol, Leash or Follow entry sent each witness a one-byte, truncated `onSequence`. NA10 removed the send. The server keeps the value as a cache only, logged as `movement.movement_type outcome=suppressed`.

**What the client animates from.** `EntityManager::onEntityMoveWithError` (`0x00dd1650`) scales the wire velocity by 100 and swaps its axes. `GameEntityBase::ApplyTransform` (`0x00e68a30`) writes it into the UE3 actor next to `Location` and `Rotation`. That velocity is the only per-NPC "moving" signal on the wire. An NPC is shown standing by broadcasting a zero velocity. Cimmeria does this with `npc_ai::movement_stop` (NA10).

**Consequences for other work**

- A leash walk home is shown by position and velocity. There is no Leash type to send.
- A cover pose cannot come from movement type 0. It has to come from something else, such as a stance, a state flag or an ability animation (NA20 / NA22).
- Showing an NPC's movement mode on a client is a GM-only feature through `onShowPath`, if it is ever wanted.
