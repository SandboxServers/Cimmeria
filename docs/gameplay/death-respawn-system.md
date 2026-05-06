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

When the cell handles `callForAid` or `respawn` ([`cell/cell_methods/player/combat.rs::handle_respawn`](../../crates/services/src/cell/cell_methods/player/combat.rs)) the path forks on whether the resolved respawn point is in the same world as where the player died:

- **Same-world respawn (the common case)**: keep the cell entity (and its instance) alive, reset its server-side state in place, and drive a *client-only* `RESET_ENTITIES` + world-entry replay via `CellToBaseMsg::RespawnReload`. **The instance — NPCs, kismet sequences (door states, completed encounters), regions — survives.** Dying in the middle of an active mission room then respawning brings you back to the same room mid-state, not a freshly spawned copy.
- **Cross-world respawn**: fall through to the gate-travel pipeline. The player is leaving the space anyway (different world entirely), so destroying+recreating the cell entity is correct.

### Same-world flow (`CellToBaseMsg::RespawnReload`)

1. **Resolve target** — `resolve_respawn_target(respawner_id, entity_id, space_mgr)` returns `(world, [x, y, z])`. Priority: explicit respawner_id → first respawner registered for the player's current world → Castle default for `Castle_CellBlock`/unknown → in-place at the player's current position for any other world.
2. **`onEndAidWait`** (method 99) — close the Defeat Window before the loading screen renders.
3. **Reset entity state in place** — HEALTH/FOCUS to max, `clear_all_state_flags` (drops both `state_field` and the per-flag refcount map — a raw `state_field = 0` would leave stale counters), `clear_all_cooldowns`, `update_entity_position` to the spawn point.
4. **Flush bandolier ammo** — drain `bandolier_ammo_dirty` to DB so the post-reload `query_player_load_data` sees latest values.
5. **`CellToBaseMsg::RespawnReload { entity_id, space_id, world_name, position }`** — `space_id` is the entity's existing space (no `BaseToCellMsg::CreateEntity` round-trip — would create a fresh space and orphan the old instance).
6. BaseApp [`handle_respawn_reload`](../../crates/services/src/base/world_entry/respawn_reload.rs):
   - Persists the spawn point to `sgw_player` (relog returns to spawn, not corpse).
   - Sends `RESET_ENTITIES` so the client destroys all client-side entities (including the ragdolled pawn).
   - Sets `pending_world_entry` (with the existing space_id), `pending_player_load_data`, and **`pending_respawn_reload = true`**.
7. Client → server: `ENABLE_ENTITIES` → BaseApp sends `CREATE_BASE_PLAYER + onClientMapLoad`.
8. Same-world means the client skips `mapLoaded` (terrain unchanged). [`handle_on_client_ready`](../../crates/services/src/base/world_entry_appearance.rs) fast-forwards through `handle_map_loaded` when `pending_map_loaded` is still set, which sends `VIEWPORT + CELL + FORCED_POSITION + entity data` and stages `pending_client_ready`.
9. The on-ready finalization continues on the same call — but **skips `BaseToCellMsg::ConnectEntity` and `InitPlayerState`** because `pending_respawn_reload` is set: the cell entity is already initialised, re-running InitPlayerState would re-load missions from DB (potentially regressing in-flight state).
10. BeingAppearance + onEntityTint resent (visuals only). Player is back on their feet.

The `RESET_ENTITIES` step destroys the ragdolled pawn outright. The pawn re-created by mapLoaded starts fresh — no kismet `TermRagdoll` call needed because the dead pawn no longer exists.

### Cross-world flow (`CellToBaseMsg::GateTravel`)

Identical to stargate travel: flush bandolier, `space_mgr.destroy_entity`, send `GateTravel`. BaseApp creates a new cell entity in the destination world, sends RESET_ENTITIES, and replays the full world-entry flow including `ConnectEntity` + `InitPlayerState`. The instance teardown is unavoidable (the player is leaving the space).

## Why a reload (not the in-place kismet path)

A previous attempt drove ragdoll exit in place via `onSequence Entity_Spawn` (5000), expecting the client kismet to call `APawn::TermRagdoll` on the local pawn. Empirically that didn't work — the player stayed face-down on the floor after the position snap. Ghidra inspection of `SGW.exe` confirmed why:

- `SeqEvent_EntitySpawn` is a registered UClass (`USeqEvent_EntitySpawn` RTTI at `01dc5b68`, registered by `FUN_006b18f0`), and `Event_NetIn_onSequence` correctly routes through `SequenceManager` (callback class RTTI at `01e21d20`). So the **wire dispatch path is real**.
- But sequences 2753 (Entity_Spawn) and 2140 (Entity_Death) both point to the **same cooked kismet package** `KIS-abilities_human.Death`. The package name + the Python emulator's documented "Entity_Spawn was never completed" note + two empirical attempts at an in-place burst (different orderings, same ragdoll-stuck symptom) are consistent with the package's `SeqEvent_EntitySpawn` node having no output wired to `TermRagdoll`. The cooked `.upk` is not server-modifiable.

The reload sidesteps the kismet wiring entirely. It also avoids the `pending_map_loaded` handshake gap that broke an even earlier reload-style respawn handler — the gate-travel path uses `pending_world_entry` + `ENABLE_ENTITIES`, not the `pending_client_ready`/`pending_map_loaded` pair, so there's no handshake field that has to be set on both sides.

A previous version of this doc described the in-place flow as the working implementation. That was wrong — keep this section in mind if you're tempted to re-introduce an `onSequence Entity_Spawn` shortcut.

## Critical: Ragdoll is Kismet-Controlled

The client's ragdoll state is **entirely controlled by kismet sequences**, NOT by the state field:

- `BSF_Dead` in `onStateFieldUpdate` → updates movement speed, fires UI events, does NOT control ragdoll
- `Entity_Death` (5001) via `onSequence` → triggers `SeqEvent_EntityDeath` kismet → calls `InitRagdoll` (this DOES work; the death package wires it)
- `Entity_Spawn` (5000) via `onSequence` → would trigger `SeqEvent_EntitySpawn` kismet → would call `TermRagdoll`, but the cooked package's spawn-event output is not wired

The respawn path can't rely on Entity_Spawn for ragdoll exit; the only reliable way to stop the local pawn from ragdolling is to destroy it (RESET_ENTITIES) and re-create it.

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
| 0 | 1 | BSF_Dead | Set on death; cleared by mapLoaded after the respawn reload |
| 6 | 64 | BSF_MovementLock | Set on death; cleared by mapLoaded |

The post-reload `mapLoaded` packet emits `onStateFieldUpdate(0)` as part of its standard init burst, so the cell doesn't need to issue a separate state-field clear during respawn.

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
No arguments. Sent by the cell at the start of respawn so the Defeat Window closes before the loading screen renders.

### CellToBaseMsg::GateTravel (cell→base, internal RPC)

Not on the client wire; this is the inter-service handoff that triggers the reload. See [`crates/services/src/cell/messages/cell_to_base.rs`](../../crates/services/src/cell/messages/cell_to_base.rs) for the variant.

```
GateTravel {
    entity_id: u32,
    target_world_name: String,   // resolved respawner world
    position: [f32; 3],          // resolved spawn point
    rotation: [f32; 3],          // [0, 0, 0] for respawn (gate-travel uses [0, 0, yaw])
}
```

BaseApp handles this in [`base/world_entry/gate_travel/mod.rs::handle_gate_travel`](../../crates/services/src/base/world_entry/gate_travel/mod.rs) — same code path stargates use.

## Python Reference

The Python emulator's `SGWBeing.onRevived()` only calls `self.unsetStateFlag(BSF_Dead)` and never sends `Entity_Spawn` (5000). The Entity_Spawn event ID is defined in `Atrea/enums.py:544` but never emitted in the Python codebase — and the inline note in that file says the kismet handler "was never completed." Cimmeria's reload-based respawn matches the empirical client behavior the Python emulator's note hints at.
