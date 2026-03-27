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

When the server handles `callForAid` or `respawn`:

1. **`onEndAidWait`** (method 99) — closes the Defeat Window (Lua calls `PlayerDefeatWin:hide()`)
2. **`onStatUpdate`** — restore health/focus to max
3. **`onSequence` with `Entity_Spawn` (event_id 5000)** — triggers kismet spawn sequence which calls `APawn::TermRagdoll` (UE3 exec at `0x00529780`), ending ragdoll physics
4. **`onStateFieldUpdate(0)`** — clear all flags (dead, movement lock, combat, etc.)
5. **Position update** — teleport player to spawn point via `update_entity_position`

## Critical: Ragdoll is Kismet-Controlled

The client's ragdoll state is **entirely controlled by kismet sequences**, NOT by the state field:

- `BSF_Dead` in `onStateFieldUpdate` → updates movement speed, fires UI events, does NOT control ragdoll
- `Entity_Death` (5001) via `onSequence` → triggers `SeqEvent_EntityDeath` kismet → calls `InitRagdoll`
- `Entity_Spawn` (5000) via `onSequence` → triggers `SeqEvent_EntitySpawn` kismet → calls `TermRagdoll`

Without `Entity_Spawn`, clearing `BSF_Dead` leaves the player ragdolled.

## Kismet Events (from Ghidra)

| Event ID | Name | Ghidra Address | Kismet Class | Effect |
|----------|------|---------------|--------------|--------|
| 5000 | Entity_Spawn | 0x0186b646 | SeqEvent_EntitySpawn | End ragdoll, stand up |
| 5001 | Entity_Death | 0x0186b59e | SeqEvent_EntityDeath | Start ragdoll |
| 5002 | Entity_Despawn | — | SeqEvent_EntityDespawn | Remove entity |
| 5005 | Entity_CombatStateChanged | 0x019cb986 | SeqEvent_CombatStateChanged | Animation state transition |

## Event Set Sequences (DB)

Event set 1025 (Mob event set, used for players and NPCs):

| sequence_id | event_id | Event Name | kismet_script_name |
|------------|----------|------------|-------------------|
| 2753 | 5000 | Entity_Spawn | KIS-abilities_human.Death |
| 2140 | 5001 | Entity_Death | KIS-abilities_human.Death |

Both use the same kismet package which contains init/term ragdoll nodes.

## State Field Flags (EStateField)

| Bit | Value | Name | Death Role |
|-----|-------|------|------------|
| 0 | 1 | BSF_Dead | Set on death, cleared on respawn |
| 6 | 64 | BSF_MovementLock | Set on death to prevent WASD/jump |

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
No arguments.

## Python Reference

The Python emulator's `SGWBeing.onRevived()` only calls `self.unsetStateFlag(BSF_Dead)`. It never sends `Entity_Spawn` (5000). The Entity_Spawn event ID is defined in `Atrea/enums.py:544` but never used in the Python codebase. **This feature was never completed in the previous emulator.**
