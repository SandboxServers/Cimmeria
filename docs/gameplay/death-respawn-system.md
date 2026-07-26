---
title: "Death & Respawn System"
type: reference
audience: engineers
last_updated: 2026-05-27
---

# Death & Respawn System

## Death Flow

When an entity's health reaches zero:

1. **Server sets `BSF_Dead` (bit 0) + `BSF_MovementLock` (bit 6)** on the entity's state field
2. **Server sends `onSequence` with `Entity_Death` (event_id 5001)** — triggers kismet death sequence in the client, which calls `APawn::InitRagdoll` (UE3 exec at `0x00529740`)
3. **Server sends `onStateFieldUpdate`** — client processes flag changes for UI/movement
4. **For NPCs**: AI transitions to `AiState::Dead`, threat list and nav path cleared, velocity zeroed, interaction type cleared (or set to loot)
5. **For players**: server sends `onBeginAidWait(timeToAid, respawnerList)` which opens the Defeat Window

## Defeat Window

- Client UI: `SGWGame/Content/UI/Core/PlayerDefeat/PlayerDefeat.lua`
- Shows "Player Defeated..." with a countdown timer and respawner list
- Player can click "Release" → sends `callForAid(respawnerID)` cell method
- Timer expiry → client sends `respawn` cell method

## Respawn Flow

When the cell handles `callForAid` or `respawn` ([`cell/cell_methods/player/combat/mod.rs::handle_respawn`](../../crates/services/src/cell/cell_methods/player/combat/mod.rs)) the path forks on whether the resolved respawn point is in the same world as where the player died:

- **Same-world respawn (the common case)**: keep everything alive — cell entity, instance, AoI entities on the client, kismet state. Send a small in-place burst that re-creates only the local pawn actor on the client. **The instance — NPCs, kismet sequences (door states, completed encounters), regions — survives.** Dying in a room with an opened stasis door means coming back to that same opened door, not a freshly-spawned copy of the room.
- **Cross-world respawn**: fall through to the gate-travel pipeline. The player is leaving the space anyway (different world entirely), so destroying+recreating the cell entity and tearing down the client view is correct.

### Same-world flow (`CellToBaseMsg::ReanchorPlayer`)

1. **Resolve target** — `resolve_respawn_target(respawner_id, entity_id, space_mgr)` returns `(world, [x, y, z])`. Priority: explicit respawner_id → first respawner registered for the player's current world → Castle default for `Castle_CellBlock`/unknown → in-place at the player's current position for any other world.
2. **`onEndAidWait`** (method 99) — close the Defeat Window first.
3. **Reset cell-entity state in place** — HEALTH/FOCUS to max, `clear_all_state_flags` (drops both `state_field` and the per-flag refcount map — a raw `state_field = 0` would leave stale counters), `clear_all_cooldowns`, `update_entity_position` to the spawn point.
4. **`onStatUpdate`** — push the refreshed HEALTH/FOCUS to the HUD.
5. **`onStateFieldUpdate(0)`** — clears BSF_Dead / BSF_MovementLock / dead-cursor visuals on the owning client.
6. **`CellToBaseMsg::ReanchorPlayer { entity_id, space_id, position, rotation }`** — BaseApp [`handle_reanchor_player`](../../crates/services/src/base/world_entry/reanchor_player.rs) emits two packets to the client:
   - **Burst** — `BASEMSG_CREATE_BASE_PLAYER` + `BASEMSG_SPACE_VIEWPORT_INFO` + `BASEMSG_CREATE_CELL_PLAYER` + `BASEMSG_FORCED_POSITION`. `CREATE_BASE_PLAYER` is the load-bearing piece; it invokes the client's `createBasePlayer` hook (same path as initial login), which destroys the ragdolled pawn actor and instantiates a fresh standing one.
   - **Property replay** (separate bundle, after the client's creation transaction settles) — `BeingAppearance` + `onEntityTint`, drawn from `ConnectedClientState`'s `cached_appearance_args` / `cached_tint_args` (populated during initial world entry in `map_loaded.rs`). Without this the recreated pawn would render blank.

   **No `RESET_ENTITIES`, no `onClientMapLoad`, no terrain reload.**

The result: ragdoll cleared, pawn standing with full appearance, while every other client-side entity is untouched. Instance preservation comes from never sending `RESET_ENTITIES` — door states / completed encounters / triggered sequences all survive the respawn.

### Cross-world flow (`CellToBaseMsg::GateTravel`)

Identical to stargate travel: flush bandolier, `space_mgr.destroy_entity`, send `GateTravel`. BaseApp creates a new cell entity in the destination world, sends `RESET_ENTITIES`, and replays the full world-entry flow including `ConnectEntity` + `InitPlayerState`. The instance teardown is unavoidable (the player is leaving the space).

## Why a re-anchor (not the in-place kismet path, not a full reload)

Two earlier approaches both failed:

**Approach 1 — `onSequence Entity_Spawn` (5000) kismet** drove ragdoll exit through the client's `SeqEvent_EntitySpawn` kismet, expecting the kismet to call `APawn::TermRagdoll`. Empirically: player stayed face-down on the floor. Ghidra confirmed why:

- `SeqEvent_EntitySpawn` is a registered UClass (`USeqEvent_EntitySpawn` RTTI at `01dc5b68`, registered by `FUN_006b18f0`), and `Event_NetIn_onSequence` correctly routes through `SequenceManager` (callback class RTTI at `01e21d20`). Wire dispatch is real.
- But sequences 2753 (Entity_Spawn) and 2140 (Entity_Death) both point to the **same cooked kismet package** `KIS-abilities_human.Death`. The package name + Python emulator's "Entity_Spawn was never completed" note + two empirical attempts at different orderings of the in-place burst → consistent with the package's `SeqEvent_EntitySpawn` node having no output wired to `TermRagdoll`. Cooked `.upk` is not server-modifiable.
- Tracing `BSF_Dead` (state-field bit 0): the GameBeing handler at `FUN_00e6e330` updates color/cursor flags but **never calls TermRagdoll**. Ragdoll exit on the local pawn is purely kismet-driven.

**Approach 2 — full `RESET_ENTITIES` + world-entry reload** (gate-travel applied to respawn) succeeded at clearing ragdoll because it destroyed and re-created the local pawn outright. But it also destroyed all *other* client entities — which re-fired kismet `OnInit`/`OnSpawn` on every actor in the level, resetting door states / completed encounters / triggered sequences. Visible regression: dying with the stasis room door open meant coming back to find it closed.

**Approach 3 — `VIEWPORT_INFO` + `CREATE_CELL_PLAYER` + `FORCED_POSITION` only** (no `CREATE_BASE_PLAYER` prefix): instance preserved (good — kismet untouched) but player still ragdolled (bad). The client's `createCellPlayer` handler treats a re-issue for an existing player id as a space/viewport update, not a pawn recreate. Confirmed via Ghidra: only `createBasePlayer` invokes the player-create callback that destroys/recreates the local pawn actor.

**Approach 4 — `CREATE_BASE_PLAYER` prefix without property replay**: pawn was destroyed cleanly (un-ragdolled, good) but the recreated pawn had no properties, leaving the player invisible. The base entity's properties are wiped along with the old pawn; we need to re-emit them after the recreate.

**Current approach — Approach 4 + cached property replay**: send `CREATE_BASE_PLAYER` + `VIEWPORT_INFO` + `CREATE_CELL_PLAYER` + `FORCED_POSITION` as a burst, then in a separate bundle replay `BeingAppearance` and `onEntityTint` from `ConnectedClientState`'s cached world-entry args. The burst destroys/recreates the pawn (un-ragdoll); the replay repopulates its visuals. The replay must be a separate bundle because the client treats `CREATE_CELL_PLAYER` as the start of a creation transaction and drops entity methods sent in the same bundle (see [`map_loaded.rs:74-81`](../../crates/services/src/base/world_entry/map_loaded.rs)). All other client-side entities and kismet state survive untouched.

## Critical: Ragdoll is Kismet-Controlled

The client's ragdoll state is **entirely controlled by kismet sequences**, NOT by the state field:

- `BSF_Dead` in `onStateFieldUpdate` → updates movement speed, fires UI events, does NOT control ragdoll
- `Entity_Death` (5001) via `onSequence` → triggers `SeqEvent_EntityDeath` kismet → calls `InitRagdoll` (this DOES work; the death package wires it)
- `Entity_Spawn` (5000) via `onSequence` → would trigger `SeqEvent_EntitySpawn` kismet → would call `TermRagdoll`, but the cooked package's spawn-event output is not wired

The respawn path can't rely on Entity_Spawn for ragdoll exit. The only reliable way to clear it is to destroy and re-create the pawn actor — either selectively via `CREATE_BASE_PLAYER` re-issue (current approach) or globally via `RESET_ENTITIES` (rejected: kismet reset).

## Kismet Events (from Ghidra)

| Event ID | Name | Ghidra Address | Kismet Class | Effect |
|----------|------|---------------|--------------|--------|
| 5000 | Entity_Spawn | 0x0186b646 | SeqEvent_EntitySpawn | (intended: end ragdoll) — not wired in shipped `KIS-abilities_human.Death` |
| 5001 | Entity_Death | 0x0186b59e | SeqEvent_EntityDeath | Start ragdoll (works; wired in the death kismet) |
| 5002 | Entity_Despawn | — | SeqEvent_EntityDespawn | Remove entity |
| 5005 | Entity_CombatStateChanged | 0x019cb986 | SeqEvent_CombatStateChanged | Animation state transition |

## Event Set Sequences (DB)

Event set 1025 (Mob event set, used for players and NPCs):

| sequence_id | event_id | Event Name | kismet_script_name |
|------------|----------|------------|-------------------|
| 2753 | 5000 | Entity_Spawn | KIS-abilities_human.Death |
| 2140 | 5001 | Entity_Death | KIS-abilities_human.Death |

Both rows reference the same cooked package. Only the death-side wiring is functional in the shipped client.

## State Field Flags (EStateField)

| Bit | Value | Name | Death Role |
|-----|-------|------|------------|
| 0 | 1 | BSF_Dead | Set on death; cleared in place by `onStateFieldUpdate(0)` during respawn |
| 6 | 64 | BSF_MovementLock | Set on death; cleared the same way |

## Wire Format

### onBeginAidWait (method 98)
```
[TimeToAid: i32]           // seconds until auto-respawn (30)
[array_count: u32]         // number of respawners
Per respawner:
  [respawnerID: i32]       // respawner entity ID
  [name: WSTRING]          // u32 char_count + UTF-16LE
```

### onEndAidWait (method 99)
No arguments. Sent by the cell at the start of respawn so the Defeat Window closes before the re-anchor.

### CellToBaseMsg::ReanchorPlayer (cell→base, internal RPC)

Not on the client wire; this is the inter-service handoff. See [`crates/services/src/cell/messages/cell_to_base.rs`](../../crates/services/src/cell/messages/cell_to_base.rs) for the variant.

```
ReanchorPlayer {
    entity_id: u32,
    space_id: u32,    // entity's existing space — NOT a fresh space_id
    position: [f32; 3],
    rotation: [f32; 3],
}
```

BaseApp handles this in [`base/world_entry/reanchor_player.rs::handle_reanchor_player`](../../crates/services/src/base/world_entry/reanchor_player.rs). Emits a `CREATE_BASE_PLAYER` (same wire layout as `phases::build_create_player` minus the `onClientMapLoad`) followed by `build_enter_world_body`, as a single standalone packet. No `RESET_ENTITIES`, no `onClientMapLoad`, no pending-state plumbing.

### CellToBaseMsg::GateTravel (cell→base, cross-world only)

Used for cross-world respawn (respawner in a different world). Same path stargates use; full instance teardown on both sides. See [`base/world_entry/gate_travel/mod.rs`](../../crates/services/src/base/world_entry/gate_travel/mod.rs).

## Python Reference

The Python emulator's `SGWBeing.onRevived()` only calls `self.unsetStateFlag(BSF_Dead)` and never sends `Entity_Spawn` (5000). The Entity_Spawn event ID is defined in `Atrea/enums.py:544` but never emitted in the Python codebase — and the inline note in that file says the kismet handler "was never completed." Cimmeria's `CREATE_BASE_PLAYER`-based re-anchor sidesteps the kismet path entirely by re-running the same pawn-create hook the client uses on login, and avoids the kismet-reset side-effect that a `RESET_ENTITIES`-based reload would produce.
