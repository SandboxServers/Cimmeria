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

When the cell handles `callForAid` or `respawn` ([`cell/cell_methods/player/combat.rs::handle_respawn`](../../crates/services/src/cell/cell_methods/player/combat.rs)) it hands off to the **gate-travel reload path** — the same pipeline stargates use. Step by step:

1. **Resolve target** — `resolve_respawn_target(respawner_id, entity_id, space_mgr)` returns `(world, [x, y, z])`. Priority: explicit respawner_id → first respawner registered for the player's current world → Castle default for `Castle_CellBlock`/unknown → in-place at the player's current position for any other world (avoids silently snapping the player cross-world if a content gap leaves a world without a respawner).
2. **`onEndAidWait`** (method 99) — close the Defeat Window before kicking off the loading screen, otherwise the panel renders on top of the loading screen for one tick.
3. **Bandolier-ammo flush** — drain `bandolier_ammo_dirty` into `BandolierAmmoUpdate` so per-slot ammo persists across the destroy/recreate. Mirrors `handle_dial_gate`.
4. **`SpaceManager::destroy_entity`** — tear down the cell entity. The reload re-creates it via `BaseToCellMsg::CreateEntity`; `InitPlayerState` repopulates `player_id` / abilities / bandolier / missions on the fresh entity; mapLoaded re-seeds stats from archetype defaults.
5. **`CellToBaseMsg::GateTravel`** — hand off to BaseApp ([`base/world_entry/gate_travel/mod.rs`](../../crates/services/src/base/world_entry/gate_travel/mod.rs)). Base sends `BaseToCellMsg::CreateEntity` to recreate the cell entity at the spawn point, persists the destination world+position to `sgw_player`, sends `RESET_ENTITIES` to the client, and sets `pending_world_entry`. The client's next `ENABLE_ENTITIES` then drives the standard create-player + enter-world + mapLoaded sequence.

The `RESET_ENTITIES` step destroys the ragdolled pawn outright. The pawn re-created by mapLoaded starts fresh — no kismet `TermRagdoll` call needed because the dead pawn no longer exists.

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
