# Respawn Lifecycle — RE Findings

**Confidence**: HIGH
**Date**: 2026-05-13
**Worker**: W-respawn-lifecycle (V5 Documentation Campaign)
**Related issues**: #232 (state-field broadcast), #233 (per-player respawner unlock gating)

## Overview

Covers the full death and respawn lifecycle as recovered from SGW.exe: HP→0 through the
Defeat Window, respawner selection, entity teleport, and state reset. Twelve functions
annotated in Ghidra. Key finding: the client binary uses **no separate corpse entity** —
the original entity transitions state in place via the BSF_Dead flag.

---

## 1. Death Sequence

### HP → 0 → BSF_Dead

Damage is entirely server-side (confirmed by `combat-damage-analysis.md`). When the server
determines an entity's HP has reached zero it:

1. Applies BSF_Dead (bit 0 of `bStateField`) via the property-sync wire.
2. The client receives an `onStateFieldUpdate` message (method index confirmed in
   `state-flag-broadcast.md`).
3. `GameBeing_OnStateFieldUpdate` (`0x00e01c90`) XOR-diffs the old and new `bStateField`
   value and dispatches each changed bit to a per-flag handler.
4. For bit 0 (BSF_Dead): dispatch target is `GameBeing_OnDeadStateChanged` (`0x00e6e330`).

### GameBeing_OnDeadStateChanged (0x00e6e330)

Confirmed by decompilation and completeness score 39 (structural ceiling ~77).

- Reads the BSF_Dead transition direction (set or cleared).
- If **newly dead**: changes the entity interaction type from alive→dead/loot, fires
  `Event_Entity_InteractionUpdate` on the CME bus.
- If **cleared** (respawn path): reverses the interaction type change.

Addresses:
- `0x00e6e330` — `GameBeing_OnDeadStateChanged` (plate comment set)
- `0x00e01c90` — `GameBeing_OnStateFieldUpdate` (XOR-delta dispatch, documented W-state)

### GameBeing_GetInteractionConfig (0x00dff610)

Returns a 4-element interaction descriptor array queried by the right-click pipeline.

Gate predicate (at start of function):
```
if ((*(byte *)(pThis + 0x158) & 1) != 0)   // bit 0 = BSF_Dead
    → fill all 4 slots with dead/loot descriptor (DAT_0185d37c)
else
    → switch on pThis+0x13c (entity class tag, values 1-4)
      → fill slots with class-appropriate interaction handles
```

This is the mechanism that transforms right-click from "talk/interact" to "loot corpse"
when BSF_Dead is set. See also `right-click-routing-on-corpse.md` for the full right-click
gate predicate at `0x00e68570`.

Key data addresses:
- `0x0185d37c` — dead/loot interaction descriptor handle
- `0x017f7ea0` — default/talk interaction descriptor handle
- `0x0185d374` — secondary NPC interaction handle (case 2 only)

### Corpse Entity Model — CONFIRMED: In-Place Transition

**No separate corpse entity is spawned.** The original entity's BSF_Dead flag is toggled,
which causes:
- Interaction config to return loot descriptors (via `GameBeing_GetInteractionConfig`)
- Ragdoll physics change via client-side kismet Entity_Death sequence (event 5001)
- Nameplate tint via `GameBeing_ApplyDeadInteraction` (`0x00e791d0`)

Evidence:
- String search for `SGWPlayerCorpse`, `SGWCorpse`, `SGWPlayerRespawner`, `SGWRespawner`
  returned zero results in the client binary.
- `GameBeing_OnDeadStateChanged` operates on `this` (the existing entity), not a new
  entity allocation.
- `death-respawn-system.md` confirms this interpretation from the server side.

### GameBeing_ApplyDeadInteraction (0x00e791d0)

UI-layer only. Looks up the entity's "CharacterName" widget, reads a Color value from
the caller-supplied descriptor, and applies it via vtable slot at offset +0x158 on the
widget node. Does not modify simulation state.

---

## 2. Defeat Window

### onBeginAidWait Wire Format (method 98)

Confirmed from decompilation of `SGWScriptedWindow_ParseBeginAidWaitEvent` (`0x00cc2eb0`).
The function explicitly reads field name strings at:
- `0x019B4670` — `"TimeToAid"`
- `0x019B467C` — `"respawners"`
- `0x019B4688` — `"respawnerID"`
- `0x019B4694` — `"respawnerName"`

Wire layout:
```
onBeginAidWait (cell → client, method 98):
  [INT32  TimeToAid]        // seconds until auto-respawn fires; server sends 30
  [UINT32 array_count]      // number of available respawners
  Per respawner entry:
    [INT32   respawnerID]   // identifies the respawner; passed back in CALL_FOR_AID
    [WSTRING respawnerName] // display name shown in Defeat Window UI
```

Server sends `TimeToAid = 30` (hardcoded in `damage_apply/mod.rs`). If no respawners are
configured for the world, the server sends a synthetic entry: `respawnerID=0, name="Respawn Point"`.

### Defeat Window UI

`SGWGame/Content/UI/Core/PlayerDefeat/PlayerDefeat.lua`

Player presses "Release" → `callForAid(respawnerID)` → `Lua_callForAid` C binding.

### SGWScriptedWindow_ParseBeginAidWaitEvent (0x00cc2eb0)

Plate comment set. Algorithm:
1. Read `TimeToAid` from event object.
2. Get Lua window context from the owning `SGWScriptedWindow`.
3. Iterate the `respawners` array; per entry: read `respawnerID` and `respawnerName`.
4. Push each entry to Lua stack and fire the Lua callback.

Score 28 effective (max achievable 78) — complex function with 51 undefined locals;
structural ceiling. Knowledge fully captured in plate comment.

### SGWScriptedWindow_OnBeginAidWait_Dispatch (0x00cea4a0)

Thin wrapper: reads `pThis+0x8` (count), `pThis+0xc` (list ptr), `pThis+0x10` (Lua ctx),
calls `SGWScriptedWindow_ParseBeginAidWaitEvent`. Named and plated.

---

## 3. Client → Server Respawn Messages

### CALL_FOR_AID (cell method 67) — Player Presses Release

Full wire path, confirmed by decompilation:
```
PlayerDefeat.lua::callForAid(respawnerID)
  → Lua_callForAid (0x00aa1c00)          [Lua C binding, __cdecl, returns int]
  → EmitNetOut_callForAid (0x00aea880)   [Pattern-B NetOut emitter]
  → Event_NetOut_callForAid CME signal
  → SGWNetworkManager::EventHandler<Event_NetOut_callForAid>::HandleEvent (vtable slot 5)
  → RouteOutgoingEntityRpc (0x00c6fc40)
  → wire: entity method call, method index 67 (CALL_FOR_AID)
```

`Lua_callForAid` signature (confirmed):
```c
int __cdecl Lua_callForAid(void* pLuaState);
// Args: Lua stack must have exactly 2 entries (self + respawnerID integer)
// Returns: 0 (success pushed to Lua stack)
```

`EmitNetOut_callForAid` (`0x00aea880`) — Pattern B:
- Field set: `"respawnerID"` → `"aRespawnerMobID"` (inferred from SetField call sequence)
- Event object: 12 bytes (`scalable_malloc(0xc)`)

### RESPAWN (cell method 70) — Auto-Respawn Timer

Client fires method 70 with no args after `TimeToAid` seconds. Server-side: handled by the
same `handle_respawn()` code path as CALL_FOR_AID, but with `respawner_id = -1`, which
falls through to world-default respawner selection.

### UNSTUCK (cell method 71)

Unimplemented on server side as of this session.

---

## 4. Server → Client GiveRespawner Flow

### EmitNetOut_GiveRespawner (0x00c81430)

Pattern B emitter. Called when the server grants the client a respawner entity.

Algorithm:
1. Read `"RespawnerMobId"` field from incoming entity data via CME GetField.
2. `scalable_malloc(0xc)` → `EventNetOut_GiveRespawner_Ctor` (`0x00cb7ee0`).
3. Set `"aRespawnerMobID"` on the event object via `CmeEventSignal_SetFieldHelper`.
4. Dispatch: `FUN_00cace50(pSystem, 0, pEvent, 1)` → SGWNetworkManager routes to client.

### EventNetOut_GiveRespawner_Ctor (0x00cb7ee0)

Fastcall ctor. Double-vtable-overwrite pattern:
- `NetworkEvent_Ctor` (base init)
- `*param_1 = NetworkEvent::vftable` (immediately overwritten)
- `*param_1 = Event_NetOut_GiveRespawner::vftable` (`0x019B37E0`)

Identical structure to `EventNetOut_callForAid_Ctor`.

---

## 5. Respawner Selection — resolve_respawn_target

Implemented in `crates/services/src/cell/cell_methods/player/combat/respawn.rs`.

Priority order:
1. Explicit `respawner_id > 0` from CALL_FOR_AID → find in `space_mgr.respawners` by id.
2. First `RespawnerDef` matching `entity.world_name`.
3. Castle default: world `"Castle_CellBlock"`, position `[-334.231, 73.472, -228.026]`.
4. In-place at current position (warn log; avoids silent cross-world teleport).

`RespawnerDef` structure (from `crates/services/src/cell/spawner/respawners.rs`):
```rust
pub struct RespawnerDef {
    pub respawner_id: i32,
    pub world_name: String,
    pub name: String,
    pub pos: [f32; 3],
}
```
SQL source: `resources.respawners JOIN resources.worlds ON world_id`.

### Issue #233 — Per-Player Respawner Unlock Gating

**No binary evidence** of a per-player unlock mechanism in SGW.exe. String searches for
"SGWPlayerRespawner" and per-player unlock concepts returned zero results.

Current Cimmeria implementation uses a global flat `Vec<RespawnerDef>` filtered only by
`world_name` — all respawners in a world are available to all players. Issue #233 asks
for a per-player `HashSet<i32>` of unlocked respawner IDs (e.g., populated when
`EmitNetOut_GiveRespawner` is received). This is a **server-side design decision** not
recoverable from the binary.

Recommended Rust fix (out of scope for this worker — no Rust edits):
```rust
// In player entity: add field
pub unlocked_respawners: HashSet<i32>,

// In handle_respawn(): filter space_mgr.respawners by world_name AND
// respawner_id in entity.unlocked_respawners (or allow all if set is empty = legacy mode)

// On GiveRespawner method receive: insert respawner_id into unlocked_respawners
```

---

## 6. Respawn Execution

### Same-World Respawn — CellToBaseMsg::ReanchorPlayer

No `RESET_ENTITIES`. In-place state reset:
1. `onEndAidWait` (method 99) — closes Defeat Window.
2. Reset HEALTH + FOCUS stats to max; serialize dirty stats; clear dirty flags.
3. `entity.clear_all_state_flags()` — hard-resets `bStateField` + refcounts.
4. `entity.abilities.clear_all_cooldowns()` — clears ability cooldown state.
5. Update entity position in space manager.
6. Send `onStatUpdate` → refreshes HUD health/focus bars.
7. Send `onStateFieldUpdate(0)` → clears BSF_Dead / BSF_MovementLock / dead-cursor on client.
8. `CellToBaseMsg::ReanchorPlayer` → base sends:
   - `BASEMSG_CREATE_BASE_PLAYER` (pawn recreate, drops ragdoll)
   - `BASEMSG_SPACE_VIEWPORT_INFO` + `BASEMSG_CREATE_CELL_PLAYER` + `BASEMSG_FORCED_POSITION`
   - Cached `BeingAppearance` + `onEntityTint` replay

Why `CREATE_BASE_PLAYER` and not just `CREATE_CELL_PLAYER`: the client's `createCellPlayer`
handler treats a re-issue for an existing player id as a space/viewport update, not a pawn
recreate. The pawn stays ragdolled. `createBasePlayer` destroys the ragdolled pawn actor
and instantiates a fresh one. (Confirmed by iteration — see `respawn.rs` doc comment for
the full rejected-alternatives list.)

### Cross-World Respawn — CellToBaseMsg::GateTravel

Player is genuinely leaving the space. Instance teardown unavoidable. Same code path as
gate travel: flush dirty ammo, `destroy_entity`, send `GateTravel`.

---

## 7. Wire Format Summary Table

| Message | Direction | Method | Wire Layout |
|---------|-----------|--------|-------------|
| onBeginAidWait | server→client | 98 | INT32 TimeToAid, UINT32 count, (INT32 respawnerID, WSTRING name) × count |
| onEndAidWait | server→client | 99 | (no args) |
| CALL_FOR_AID | client→server | 67 | INT32 respawnerID |
| RESPAWN | client→server | 70 | (no args) |
| UNSTUCK | client→server | 71 | (unknown — unimplemented) |
| GiveRespawner | server→client | TBD | INT32 aRespawnerMobID |

---

## 8. Address Summary

| Address | Name | Notes |
|---------|------|-------|
| `0x00e6e330` | GameBeing_OnDeadStateChanged | BSF_Dead handler; toggles interaction type |
| `0x00dff610` | GameBeing_GetInteractionConfig | Returns 4-slot interaction descriptor; BSF_Dead gate |
| `0x00e791d0` | GameBeing_ApplyDeadInteraction | Applies death color tint to CharacterName widget |
| `0x00cc2eb0` | SGWScriptedWindow_ParseBeginAidWaitEvent | Parses onBeginAidWait event; confirmed field names |
| `0x00cea4a0` | SGWScriptedWindow_OnBeginAidWait_Dispatch | Thunk → ParseBeginAidWaitEvent |
| `0x00aa1c00` | Lua_callForAid | Lua C binding; fires EmitNetOut_callForAid |
| `0x00aea880` | EmitNetOut_callForAid | Pattern-B emitter; sets respawnerID field |
| `0x00c81430` | EmitNetOut_GiveRespawner | Pattern-B emitter; sets aRespawnerMobID field |
| `0x00cb7ee0` | EventNetOut_GiveRespawner_Ctor | 12-byte event ctor; vtable 0x019B37E0 |
| `0x00c6fc40` | RouteOutgoingEntityRpc | Universal outgoing RPC router |
| `0x00e68570` | GameBeing_RightClickGatePredicate | Right-click gate; reads pThis+0x158 bit0 |
| `0x00e01c90` | GameBeing_OnStateFieldUpdate | XOR-delta dispatch (W-state) |
| `0x019B4670` | s_"TimeToAid" | onBeginAidWait field name string |
| `0x019B467C` | s_"respawners" | onBeginAidWait field name string |
| `0x019B4688` | s_"respawnerID" | onBeginAidWait field name string |
| `0x019B4694` | s_"respawnerName" | onBeginAidWait field name string |
| `0x019B37E0` | Event_NetOut_GiveRespawner::vftable | GiveRespawner event vtable |
| `0x0185d37c` | DAT — loot interaction descriptor | Returned by GetInteractionConfig when BSF_Dead |
| `0x017f7ea0` | DAT — default interaction descriptor | Returned when entity alive |

---

## 9. Open Questions

**OQ-1** — SGWPlayerRespawner client-side: No RTTI or function found under that name.
Either the respawner entity type is purely server-side, or it ships under a different
client-facing name. The `EmitNetOut_GiveRespawner` event carries only an int (mob id),
suggesting the client treats granted respawners as opaque id references, not typed entities.

**OQ-2** — Issue #233 per-player unlock: No binary evidence of a per-player mechanism.
Resolution requires a server-side design decision. See section 5 for recommended Rust fix.

**OQ-3** — Entity class tag at `pThis+0x13c`: values 1-4 observed in switch at
`GameBeing_GetInteractionConfig`. Exact class names (NPC / player / vehicle / interactive)
are inferred but not confirmed against RTTI class names.

---

## 10. Cross-References

- `docs/gameplay/death-respawn-system.md` — high-level gameplay description, wire formats
- `docs/reverse-engineering/findings/state-flag-broadcast.md` — BSF_* master table, #232/#249
- `docs/reverse-engineering/findings/right-click-routing-on-corpse.md` — right-click gate at 0x00e68570
- `docs/reverse-engineering/findings/spawn-system-mechanics.md` — RespawnerMobId property, respawner entity definition
- `docs/reverse-engineering/findings/cme-event-signal.md` — Pattern A/B emit pipeline
- `crates/services/src/cell/cell_methods/player/combat/respawn.rs` — Rust implementation
- `crates/services/src/cell/abilities/damage_apply/mod.rs` — onBeginAidWait send logic
- `crates/services/src/cell/spawner/respawners.rs` — RespawnerDef, SQL query
