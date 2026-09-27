---
title: "Duel System"
type: reference
audience: engineers
last_updated: 2026-09-27
---

# Duel System

> **Last updated**: 2026-09-27
> **Status**: Challenge and response implemented (SS-D1); no engaged duel, PvP flag or end paths yet (SS-D2, SS-D3).

## Overview

The duel system enables structured PvP combat between two players within designated duel areas. Duel areas are defined by `SGWDuelMarker` entities placed in the world, which use a proximity detector to track participating entities. Duels end when one participant is defeated.

The `SGWDuelMarker` entity is defined in `entities/defs/SGWDuelMarker.def` (parent: `SGWSpawnableEntity`).

## Implementation Status

The challenge and the answer are implemented (social-systems campaign SS-D1, [work packets](../analysis/social-systems/work-packets.md)). The duel state lives on the cell in `DuelRegistry` (`crates/cell-world/src/cell/duel/`), keyed by `player_id`.

| Method | Index | Handler |
|--------|-------|---------|
| `sendDuelChallenge` | base 0xD9 | `crates/base/src/base/dispatch/duel.rs`: duel rate limit (D-SS21), squad duels refused, target resolved online (D-SS13), Ignore seam (D-SS15), then `DuelBaseToCell::Challenge` to the cell |
| `sendDuelResponse` | 102 | `cell::duel::response`: acts only on the challenge addressed to the caller, consumed once, expired after 30 s |
| `duelForfeit` | 103 | `social.rs` — still logs `UNIMPLEMENTED` (SS-D3) |

On the cell, `cell::duel::challenge` refuses a self-challenge (text 872), a target in another space or beyond 20 units (877), either side already in a challenge or duel (873), and the same pair within 60 s of a decline or expiry. Otherwise it stores the challenge and sends the target `onDuelChallenge` [143] with the challenger's entity id and an empty squad list. Decline or expiry tells both players "Duel aborted" (878). Accept starts a 5-second countdown. Every refusal is a feedback line to the challenger. The duel texts are sent as literal feedback lines, because the client has no path that renders a duel moniker by id (SS-E1 D-Q6).

Until SS-D2 engages duels, the end of the countdown aborts the duel with 878, so neither player is left marked busy. The server sends no `onDuelEntitiesSet` [151] or `Clear` [153] (D-SS25). The 30 s, 5 s, 20-unit and 60 s values are project policy, not recovered data.

| Feature | Status | Notes |
|---------|--------|-------|
| Duel marker entity | DEFINED | `SGWDuelMarker` with detector and entity tracking; no Rust spawner support |
| Duel response | IMPLEMENTED | `sendDuelResponse` (CM 102): accept, decline and expiry (SS-D1) |
| Duel forfeit | STUB | `duelForfeit` (CM 103) dispatched, logs `UNIMPLEMENTED` |
| Defeat detection | NOT IMPL | `onEntityDefeated` cell method defined on the marker, no handler |
| Duel challenge issue | IMPLEMENTED | `sendDuelChallenge` (base 0xD9) and `onDuelChallenge` [143] (SS-D1) |
| Engaged duel, PvP flag | NOT IMPL | SS-D2; the countdown currently ends in "Duel aborted" |
| Duel area enforcement | NOT IMPL | `duelDetectorID` property exists; no proximity controller |
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
