---
title: "Minigame System"
type: reference
audience: engineers
last_updated: 2026-07-25
---

# Minigame System

> **Last updated**: 2026-07-25
> **Status**: Content-triggered minigames work end-to-end — SmartFoxServer host, ticket handshake, Livewire, six auto-win placeholders, and victory-chain callback. The player-facing `MinigamePlayer` cell methods (helpers, spectating, manual start) are all stubs.

## Overview

The minigame system provides puzzle-based mini-activities integrated into the game world. Minigames are triggered by interacting with objects or NPCs, have difficulty levels and tech competency requirements, and produce result callbacks. The system supports a "helper" call protocol where players can request assistance from registered helpers, spectating other players' minigames, and NPC-triggered minigame contacts.

The `MinigamePlayer` interface in `entities/defs/interfaces/MinigamePlayer.def` is the largest interface by method count (25 properties, 78+ methods).

The original SGW minigames were Flash SWFs that connected to a **SmartFoxServer 1.x** TCP endpoint, separate from the Mercury game channel. [`crates/services/src/minigame/`](../../crates/services/src/minigame/) reimplements that server in-process: `protocol.rs` speaks the SmartFox XML packet format, `session.rs` owns the ticket registry, `server.rs` is the TCP listener, and `games/` holds the per-game logic behind a `MinigameInstance` trait.

## How a minigame actually launches

The working path is content-driven, not client-driven:

```
Content chain fires Action::StartMinigame { minigame_type, on_victory_chains }
  |-> Cell: CellToBaseMsg::StartMinigame
  |-> Base: SessionRegistry::register(...) -> ticket (seed, difficulty, tech
  |         competency, victory chains all captured server-side)
  |-> Base: onStartMinigame(URL) to the player
  |         URL shape: http://unused/{host}/{port}/{gameName}/{entityId}/{ticket}
  |-> Flash SWF connects to the SmartFox TCP port, presents the ticket
  |-> Game plays; result reported back
  |-> Base: MinigameResult -> notifies client, forwards to the cell
  |-> Cell: runs the chain from on_victory_chains
```

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| SmartFoxServer 1.x host | DONE | `minigame/server.rs` + `protocol.rs` |
| Session / ticket registry | DONE | `minigame/session.rs`; ticket carries seed, difficulty, and the victory chains |
| Content-triggered start | DONE | `Action::StartMinigame` → `CellToBaseMsg::StartMinigame` → `onStartMinigame(URL)` |
| Victory-chain callback | DONE | `MinigameResult` forwards to the cell, which runs `on_victory_chains` |
| Livewire | DONE | Fully ported in `minigame/games/livewire/` |
| Hack, Activate, Analyze, Bypass, Converse, ConverseBasicHumanoid | PLACEHOLDER | `games/placeholder.rs` — the only accepted client message is `victory`, which is an instant win. Matches the original `Placeholder.py`; these game types had no real SWF beyond a shell |
| Alignment, GoauldCrystals | NOT IMPL | Still Python-only; the factory has commented-out arms awaiting a port. An unrecognised game name silently falls back to the auto-win placeholder |
| `startMinigame` / `endCurrentMinigame` | STUB | Cell methods 24 / 25 log `UNIMPLEMENTED` |
| Debug start / spectate / join / instance | STUB | Cell methods 20–23 log `UNIMPLEMENTED` |
| Spectating | STUB | `requestSpectateList` (26), `spectateMinigame` (27) log `UNIMPLEMENTED` |
| Helper registration | STUB | `registerToMinigameHelp` (28), `updateRegisterToMinigameHelp` (29) log `UNIMPLEMENTED` |
| Helper call protocol | STUB | `minigameCallAccept` (31), `Decline` (32), `Abort` (33) log `UNIMPLEMENTED` |
| NPC contacts | STUB | `minigameContactRequest` (34) logs `UNIMPLEMENTED` |
| Tech competency | PARTIAL | The ticket carries a tech-competency field, but it is hardcoded to `1` — the value is not yet read from the player entity |
| Mob/item attempt tracking | NOT IMPL | `minigameMobAttemptTracker`, `minigameItemAttemptTracker` unused |
| Item integration | NOT IMPL | `addItemToMinigame`, `consumeItemByMinigame` unused |
| Cheat detection | NOT IMPL | `updateMinigameItemCheats` unused |

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

## Session Ticket

A ticket is minted by `SessionRegistry::register` when the content chain starts a minigame, and is the only thing the Flash client presents to the SmartFox endpoint. Everything gameplay-relevant is captured server-side at mint time, so the client cannot influence difficulty, seed, or reward:

| Field | Source |
|-------|--------|
| `entity_id`, `player_id` | The triggering player |
| `game_name` | `Action::StartMinigame { minigame_type }` |
| `difficulty` | The content chain |
| `tech_competency` | **Hardcoded to `1`** — reading it from the player entity is still a TODO |
| `seed` | `rand::random::<u32>()` |
| `abilities`, `intelligence`, `player_level` | Hardcoded to `0`, `0`, `1` |
| `on_victory_chains` | The chains to run when the game is won |

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

- **Game-name dispatch**: `minigame/games/mod.rs::create` — the authoritative list of which names resolve to real logic versus the placeholder
- **Result codes**: `MINIGAME_RESULT_*` constants
- **Archetypes**: Bitmask of `EArchetype` values

## Remaining Work

1. **Port Alignment and GoauldCrystals** — the factory has commented-out arms; today an unknown or unported name silently resolves to the auto-win placeholder, which is indistinguishable from a real win to the rest of the server
2. **Tech competency** — read it from the player entity instead of the hardcoded `1`; the same applies to `abilities`, `intelligence`, and `player_level`
3. **Player-initiated start** — `startMinigame` (24) and the debug starts (20–23) are stubs, so a player can only enter a minigame that a content chain launched for them
4. **Helper call protocol** — the whole request → PhaseTwo → accept/decline flow, including tip cash movement
5. **Spectating** — `requestSpectateList` / `spectateMinigame`
6. **NPC contacts** — contact acquisition and expiry mechanics

## Related Docs

- [mission-system.md](mission-system.md) - Missions that require minigame completion
- [inventory-system.md](inventory-system.md) - Items used in minigames
