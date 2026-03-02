# Pet System

> **Last updated**: 2026-03-01
> **Status**: ~0% implemented

## Overview

The pet system allows players to summon and control companion NPCs that fight alongside them. Pets extend the `SGWMob` entity with owner tracking, ability management (including toggling abilities on/off), stance control, leveling, and despawn timers. Pets respond to owner events (death, leash, respawn) and can resolve abilities on spawn.

The `SGWPet` entity is defined in `entities/defs/SGWPet.def` (parent: `SGWMob`). The Python script `python/cell/SGWPet.py` contains only stub initialization for ability and stance lists.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| Pet entity definition | DONE | Full property and method set defined |
| Owner tracking | DEFINED | `ownerID`, `ownerBase` properties |
| Ability list | STUB | `onPetAbilityList` sends list to client |
| Stance list | STUB | `onPetStanceList` sends list to client |
| Ability toggling | STUB | `toggleAbility` with on/off flag |
| Stance changing | STUB | `changePetStance` with `onPetStanceUpdate` |
| Pet leveling | STUB | `setPetLevel` defined |
| Owner death response | STUB | `onOwnerDeath` cell method |
| Owner leash response | STUB | `onOwnerLeash` cell method |
| Owner respawn response | STUB | `onOwnerRespawn` with despawn flag |
| Despawn timer | DEFINED | `petDespawnTimerId` property |
| Ability on spawn | DEFINED | `abilityToResolve`, `abilityInformation` |
| XP transfer | DEFINED | `transferXP` float property |
| Position tracking | DEFINED | `ownerLastPosition`, `petLastPosition`, `lastOwnerPositionCheck` |
| Pet AI | NOT IMPL | No AI behavior scripts |
| Pet persistence | STUB | `saveToDB` defined but no save logic |

## Entity Definition (SGWPet.def)

**Parent**: `SGWMob` (inherits all mob properties, combat, ability manager, etc.)

### Properties

| Property | Type | Flags | Purpose |
|----------|------|-------|---------|
| `ownerID` | INT32 | CELL_PUBLIC | Entity ID of pet owner |
| `ownerBase` | MAILBOX | CELL_PUBLIC | Base mailbox of owner |
| `transferXP` | FLOAT | CELL_PRIVATE | XP transfer ratio (default 1.0) |
| `petDespawnTimerId` | CONTROLLER_ID | CELL_PRIVATE | Despawn countdown timer |
| `abilityToResolve` | INT32 | CELL_PRIVATE | Ability to use on spawn |
| `abilityInformation` | PYTHON | CELL_PRIVATE | Runtime params for spawn ability |
| `toggledAbilities` | ARRAY\<INT32\> | CELL_PRIVATE | Abilities toggled OFF |
| `lastOwnerPositionCheck` | FLOAT | CELL_PRIVATE | Last owner distance check time |
| `lastTeleportTime` | FLOAT | CELL_PRIVATE | Last teleport-to-owner time |
| `ownerLastPosition` | VECTOR3 | CELL_PRIVATE | Owner position cache |
| `petLastPosition` | VECTOR3 | CELL_PRIVATE | Pet position cache |
| `petStance` | INT8 | CELL_PRIVATE | Current stance (default 1) |

### Client Methods (Server -> Client)

| Method | Args | Purpose |
|--------|------|---------|
| `onPetAbilityList` | ARRAY\<INT32\> | Send pet's ability IDs to owner |
| `onPetStanceList` | ARRAY\<INT8\> | Send available stances to owner |
| `onPetStanceUpdate` | INT8 stance | Notify stance change |

### Cell Methods

| Method | Args | Purpose |
|--------|------|---------|
| `onOwnerDeath` | (none) | Owner died -- despawn or go passive |
| `onOwnerLeash` | (none) | Owner moved too far -- leash pet |
| `onOwnerRespawn` | shouldDespawn (INT8) | Owner respawned |
| `saveToDB` | playerDbId (INT32) | Persist pet state |
| `toggleAbility` | abilityId (INT32), onOff (INT8) | Toggle ability active state |
| `changePetStance` | stance (INT8) | Change pet behavior stance |
| `setPetLevel` | level (INT8) | Set pet level |
| `sendPetInfoToOwner` | ownerMailbox (MAILBOX), ownerPetAbilities (ARRAY\<INT32\>) | Send abilities to owner |

## Pet Stance System

Stances control pet AI behavior mode. The `petStance` property defaults to 1.

```
TODO: Decompile stance enum values from client
Likely values: Passive, Defensive, Aggressive, Follow
```

## Pet Ability Toggling

The `toggledAbilities` array tracks abilities that the player has turned OFF. When the pet AI selects abilities to use, it should skip any ability whose ID is in this list.

## Data References

- **Parent entity**: `SGWMob` (inherits all mob combat systems)
- **Enumerations**: Pet stance enum (needs decompilation)
- **Database**: Pet persistence table (needs schema creation)

## RE Priorities

1. **Pet AI** - Behavior tree for pet combat (stance-driven)
2. **Pet summoning** - How pets are created (from items? abilities?)
3. **Pet persistence** - `saveToDB` schema and what is saved
4. **Leash distance** - How `lastOwnerPositionCheck` triggers `onOwnerLeash`
5. **Spawn ability** - How `abilityToResolve` is used when pet spawns

## Related Docs

- [combat-system.md](combat-system.md) - Pet uses mob combat system
- [ability-system.md](ability-system.md) - Pet abilities
- [stat-system.md](stat-system.md) - Pet stats (inherited from SGWMob)
