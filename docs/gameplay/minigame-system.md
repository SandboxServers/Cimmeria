# Minigame System

> **Last updated**: 2026-03-01
> **Status**: ~0% implemented

## Overview

The minigame system provides puzzle-based mini-activities integrated into the game world. Minigames are triggered by interacting with objects or NPCs, have difficulty levels and tech competency requirements, and produce result callbacks. The system supports a "helper" call protocol where players can request assistance from registered helpers, spectating other players' minigames, and NPC-triggered minigame contacts.

The `MinigamePlayer` interface in `entities/defs/interfaces/MinigamePlayer.def` is the largest interface by method count (25 properties, 78+ methods). The `Minigame` class in `python/cell/Minigame.py` provides a basic play/result framework (58 lines).

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| Minigame class | DONE | Name, difficulty, gameId, seed, play/result |
| Minigame dialog prompt | STUB | `onStartMinigameDialog` sends game info to client |
| Start/end minigame | STUB | `startMinigame`, `endCurrentMinigame` defined |
| Result handling | STUB | `handleMinigameResults` with ticket validation |
| Helper registration | STUB | `registerToMinigameHelp` with note/range |
| Helper call protocol | STUB | Request -> PhaseTwo -> Accept/Decline flow |
| Spectating | STUB | `requestSpectateList`, `spectateMinigame` |
| NPC contacts | STUB | `showMinigameContact`, `minigameContactRequest` |
| Mob/item attempt tracking | STUB | `minigameMobAttemptTracker`, `minigameItemAttemptTracker` |
| Item integration | STUB | `addItemToMinigame`, `consumeItemByMinigame` |
| Cheat detection | STUB | `updateMinigameItemCheats` with class activations |
| Actual minigame logic | NOT IMPL | Only the framework exists |

## Entity Definition (MinigamePlayer.def)

### Properties (25)

| Property | Type | Flags | Purpose |
|----------|------|-------|---------|
| `minigame` | PYTHON | CELL_PRIVATE | Current minigame state |
| `pendingInstance` | INT32 | CELL_PRIVATE | Pending minigame instance ID |
| `pendingMinigamePosition` | VECTOR3 | CELL_PRIVATE | Position for pending minigame |
| `pendingItem` | INT32 | CELL_PRIVATE | Triggering item ID |
| `pendingMob` | INT32 | CELL_PRIVATE | Triggering mob ID |
| `pendingSeed` | INT32 | CELL_PRIVATE | Random seed for minigame |
| `pendingTC` | INT32 | CELL_PRIVATE | Tech competency override |
| `minigameMobAttemptTracker` | PYTHON | CELL_PRIVATE | Per-mob attempt counts |
| `minigameItemAttemptTracker` | PYTHON | CELL_PRIVATE | Per-item attempt counts |
| `minigameRegistrationCost` | INT32 | CELL_PRIVATE | Cost to register as helper |
| `minigameRegistered` | UINT8 | CELL_PRIVATE | Is registered as helper |
| `minigameRegisteredWantsRequests` | UINT8 | CELL_PRIVATE | Accepts help requests |
| `minigameRegisteredNote` | WSTRING | CELL_PRIVATE | Helper registration note |
| `minigameRegisteredRange` | UINT8 | CELL_PRIVATE | In-range-only flag |
| `minigameRegistrationAvailable` | UINT8 | CELL_PRIVATE | Registration available |
| `pendingHelper*` | various | CELL_PRIVATE | Pending helper call data (5 props) |
| `pendingMinigameRequests` | PYTHON | CELL_PRIVATE | Queue of help requests |
| `currentMinigameRequest` | PYTHON | CELL_PRIVATE | Active help request |
| `minigameCallTracker` | PYTHON | CELL_PRIVATE | Call history |
| `minigameWaitingOnCash` | PYTHON | CELL_PRIVATE | Pending cash transaction |
| `minigameSavedTimeInfo` | FLOAT | CELL_PRIVATE | Saved timer state |
| `minigameContacts` | PYTHON | CELL_PRIVATE | NPC contacts list |

### Client Methods (Server -> Client)

| Method | Args | Purpose |
|--------|------|---------|
| `onStartMinigame` | URL | Launch minigame in embedded browser |
| `onStartMinigameDialog` | Name, Difficulty, TCLevel, Verb, ArchetypeBitfield, CanPlay, CanCall, CanSpectate | Pre-game dialog |
| `onStartMinigameDialogClose` | (none) | Close dialog |
| `onEndMinigame` | (none) | End current minigame |
| `onSpectateList` | playerIds, playerNames | List of spectatable players |
| `onMinigameRegistrationPrompt` | Cost | Registration cost prompt |
| `minigameRegistrationInfo` | Registered, InRangeOnly, WantsRequests, Note | Registration state |
| `addOrUpdateMinigameHelper` | PlayerId, Name, Note, Level, Archetype, Friend | Helper list update |
| `removeMinigameHelper` | PlayerId | Remove from helper list |
| `minigameCallDisplay` | CallingPlayerId, Name, Archetype, Level, TipAmount, ExpiresAt, GameName, GameDifficulty, GameVerb, GameTC, NPCTitle | Incoming call request |
| `minigameCallResult` | ResultCode, StartTime | Call outcome |
| `minigameCallAbort` | CallingPlayerId | Call aborted |
| `showMinigameContact` | Id, Name, Title, Icon, Time, Success, Cost | NPC contact display |

### Cell Methods (Key Exposed)

| Method | Args | Purpose |
|--------|------|---------|
| `startMinigame` | (none) | Start pending minigame |
| `endCurrentMinigame` | (none) | End active minigame |
| `debugStartMinigame` | GameId | Debug: force start |
| `requestSpectateList` | (none) | Get spectatable players |
| `spectateMinigame` | playerId | Watch another player |
| `registerToMinigameHelp` | note, inRangeOnly | Register as helper |
| `minigameCallAccept` | CallingPlayerId | Accept help call |
| `minigameCallDecline` | CallingPlayerId | Decline help call |
| `minigameCallAbort` | (none) | Abort active call |
| `minigameStartCancel` | (none) | Cancel start |
| `minigameContactRequest` | ContactId | Request NPC contact minigame |

## Minigame Class (python/cell/Minigame.py)

```python
class Minigame:
    name            # Display name
    difficulty      # 1-5
    gameId          # Config.MINIGAME_NAMES key
    gameName        # Resolved name string
    techCompetency  # Required tech level
    archetypes      # Bitmask of allowed archetypes
    callback        # Result handler function
    player          # Active player (weakref)
    seed            # Random seed (0..0x7fffffff)
```

### Methods

| Method | Purpose |
|--------|---------|
| `play(player)` | Start minigame via `player.requestStartMinigame()` |
| `onMinigameResult(resultId)` | Invoke callback with result code |
| `setSeed(seed)` | Set random seed |

## Helper Call Protocol

```
Player A (caller):
  minigameCallRequest(RemotePlayerName, TipAmount)
    |-> Base: minigameCallRequest -> look up remote player
    |-> Cell: minigameCallRequestPhaseTwo -> display call to helper

Player B (helper):
  minigameCallAccept(CallingPlayerId)
    |-> Cell: minigameCallAcceptPhaseTwo(RemotePlayerId, InstanceId, StartTime, Ticket)
  OR
  minigameCallDecline(CallingPlayerId)
    |-> Cell: minigameCallDeclinePhaseTwo(RemotePlayerId, InstanceId, ResultCode)

Either player:
  minigameCallAbort()
    |-> minigameCallAbortPhaseTwo -> notify partner
    |-> minigameEndCall(Reason, TipAmount)
```

## Data References

- **Minigame names**: `Config.MINIGAME_NAMES` dictionary
- **Result codes**: `MINIGAME_RESULT_*` constants in `Constants.py`
- **Archetypes**: Bitmask of `EArchetype` values

## RE Priorities

1. **Minigame types** - Identify all game types and their URLs
2. **Result handling** - What happens after `handleMinigameResults` (rewards, mission progress)
3. **Tech competency** - How TC level gates minigame difficulty
4. **Helper tipping** - Cash flow for tip payments
5. **NPC contacts** - Contact acquisition and expiry mechanics

## Related Docs

- [mission-system.md](mission-system.md) - Missions that require minigame completion
- [inventory-system.md](inventory-system.md) - Items used in minigames
