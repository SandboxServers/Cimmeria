# Duel System

> **Last updated**: 2026-03-01
> **Status**: ~0% implemented

## Overview

The duel system enables structured PvP combat between two players within designated duel areas. Duel areas are defined by `SGWDuelMarker` entities placed in the world, which use a proximity detector to track participating entities. Duels end when one participant is defeated.

The `SGWDuelMarker` entity is defined in `entities/defs/SGWDuelMarker.def` (parent: `SGWSpawnableEntity`). The Python script `python/cell/SGWDuelMarker.py` is an empty stub.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| Duel marker entity | DEFINED | `SGWDuelMarker` with detector and entity tracking |
| Defeat detection | STUB | `onEntityDefeated` cell method defined |
| Duel challenge protocol | NOT IMPL | Challenge/response events referenced in client |
| Duel forfeit | NOT IMPL | Forfeit event referenced in client |
| Duel area enforcement | NOT IMPL | `duelDetectorID` property exists |
| Win/loss tracking | NOT IMPL | No outcome recording |

## Entity Definition (SGWDuelMarker.def)

**Parent**: `SGWSpawnableEntity`

### Properties

| Property | Type | Flags | Purpose |
|----------|------|-------|---------|
| `duelDetectorID` | CONTROLLER_ID | CELL_PRIVATE | Proximity detector controller |
| `duelEntities` | ARRAY\<MAILBOX\> | CELL_PRIVATE | Entities participating in the duel |

### Cell Methods

| Method | Args | Purpose |
|--------|------|---------|
| `onEntityDefeated` | entityId (INT32) | Notify marker that a participant was defeated |

## Expected Protocol (from Client References)

Based on client-side event names referenced in the README:

### Client -> Server (NetOut)

| Event | Purpose |
|-------|---------|
| `DuelChallenge` | Challenge another player to a duel |
| `DuelResponse` | Accept or decline a duel challenge |
| `DuelForfeit` | Forfeit an active duel |

### Server -> Client (NetIn)

| Event | Purpose |
|-------|---------|
| `onDuelChallenge` | Incoming duel challenge notification |
| `onDuelEntitiesSet` | Duel participants established |
| `onDuelEntitiesRemove` | Participant left duel area |
| `onDuelEntitiesClear` | Duel ended, clear all participants |

## Expected Duel Flow

```
Player A: DuelChallenge(targetPlayerId)
  |-> Server validates: both in duel area, not in combat, not already dueling
  |-> Player B: onDuelChallenge(challengerName)

Player B: DuelResponse(accepted)
  |-> If accepted:
       |-> Both players: onDuelEntitiesSet(participants)
       |-> Enable PvP between participants
       |-> Combat proceeds using normal ability/damage systems
  |-> If declined:
       |-> Challenger notified

During duel:
  |-> Player leaves area: onDuelEntitiesRemove
  |-> Player health reaches 0: onEntityDefeated(entityId)
       |-> Duel ends, winner/loser determined
       |-> Both players: onDuelEntitiesClear

Forfeit:
  Player: DuelForfeit
  |-> Duel ends, forfeiting player loses
  |-> Both players: onDuelEntitiesClear
```

## Data References

- **Entity type**: `SGWDuelMarker` (extends SGWSpawnableEntity)
- **World placement**: Duel markers are placed in game world as spawnable entities
- **Detector**: `CONTROLLER_ID` references a BigWorld proximity controller

## RE Priorities

1. **Duel protocol** - Decompile client-side duel challenge/response message format
2. **PvP flag handling** - How duels enable PvP between normally non-hostile players
3. **Duel area bounds** - How `duelDetectorID` defines the valid duel region
4. **Death handling** - Whether duel defeat uses normal death or special "downed" state
5. **Rewards/penalties** - Any XP, rating, or currency effects from duel outcomes

## Related Docs

- [combat-system.md](combat-system.md) - Combat mechanics used during duels
- [stat-system.md](stat-system.md) - Stats applied during PvP
