---
title: "Minigame System"
type: reference
audience: engineers
last_updated: 2026-10-10
---

# Minigame System

> **Last updated**: 2026-10-10
> **Status**: Content-triggered minigames work end-to-end — SmartFoxServer host, ticket handshake, Livewire, six auto-win placeholders, and victory-chain callback. The player-facing `MinigamePlayer` cell methods (helpers, spectating, manual start) are all stubs.

## Overview

The minigame system provides puzzle-based mini-activities integrated into the game world. Minigames are triggered by interacting with objects or NPCs, have difficulty levels and tech competency requirements, and produce result callbacks. The system supports a "helper" call protocol where players can request assistance from registered helpers, spectating other players' minigames, and NPC-triggered minigame contacts.

The `MinigamePlayer` interface in `entities/defs/interfaces/MinigamePlayer.def` is the largest interface by method count (25 properties, 78+ methods).

The original SGW minigames were Flash SWFs that connected to a **SmartFoxServer 1.x** TCP endpoint, separate from the Mercury game channel. [`crates/minigame/src/minigame/`](../../crates/minigame/src/minigame/) reimplements that server in-process: `protocol.rs` speaks the SmartFox XML packet format, `session.rs` owns the ticket registry, `server/` is the TCP listener and connection lifecycle, and `games/` holds the per-game logic behind a `MinigameInstance` trait.

## How a minigame actually launches

The working path is content-driven, not client-driven:

```text
Content chain fires Action::StartMinigame { minigame_type, difficulty,
                                           on_victory_chains }
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
| SmartFoxServer 1.x host | DONE | `minigame/server/` (`mod.rs` lifecycle, `accept.rs`, `limits.rs`, `framing.rs`, `handshake.rs`, `result_dispatch.rs`) + `protocol.rs` |
| Connection limits | DONE | Total and per-address connection caps, a login deadline, a 4 KiB frame cap and an idle timeout on the public port. See [Connection limits](#connection-limits) |
| Session / ticket registry | DONE | `minigame/session.rs`; ticket carries seed, difficulty, and the victory chains |
| Session expiry | DONE | A registered session whose SWF never connects is swept after `PENDING_SESSION_TTL` (180 s). See [Session lifecycle](#session-lifecycle) |
| Abort on SWF close | DONE | `run_session` calls `MinigameInstance::aborted()` and reports result code 0 (Canceled) when the socket drops without an outcome |
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
| `endMinigameForPlayer` / `minigameStartCancel` | NOT IMPL | The original's two client-driven session-cancel RPCs. Cell method 30 logs `UNIMPLEMENTED`; the TTL sweep is the stand-in |
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
| `difficulty` | The `start_minigame` chain action's `difficulty` param, 1-5, default 1 |
| `tech_competency` | **Hardcoded to `1`** — reading it from the player entity is still a TODO |
| `seed` | `rand::random::<u32>()` |
| `abilities`, `intelligence`, `player_level` | Hardcoded to `0`, `0`, `1` |
| `on_victory_chains` | The chains to run when the game is won |

## Session lifecycle

A session exists in `SessionRegistry` from the moment the chain fires until
the connection task that owns it finishes. Two things end it:

1. **The connection task.** Login validates the ticket and claims the
   session in one locked step (`authenticate_and_claim`), so a task can
   only ever own the session it authenticated against. A ticket
   authenticates one live connection: a second login with it while the
   first is connected is refused (WARN `reason=ticket_already_claimed`).
   When the connection ends — win, loss, or the player closing the window —
   the task unregisters the session. A connected session is never expired
   by age, because a Livewire round can run longer than the TTL.

   The task only ends when the server notices the socket is gone: a FIN or
   RST, a send that fails or blocks past 10 s, TCP keepalive giving up
   (about 90 s for an idle connection whose peer vanished), or the 30-minute
   idle timeout. Until then a SWF that reconnects after a half-open drop is
   refused as `ticket_already_claimed`. See
   [Connection limits](#connection-limits).
2. **The expiry sweep.** A session whose SWF never connects has no task to
   clean it up. `spawn_sweep` runs every `SWEEP_INTERVAL` (60 s) and drops
   every unconnected session older than `PENDING_SESSION_TTL` (180 s).
   `register` also sweeps before its duplicate check, so the next
   interaction recovers without waiting for a sweep tick.

Only one session may exist per entity at a time; a second launch inside the
TTL is rejected. Before the sweep existed, an abandoned launch pinned the
entity id and every later interaction with the same object was rejected
until the player relogged.

The TTL is a **backstop, not a port**. The original had no timeout at all:
its `MinigameRequestManager::QueueEntry` carries no timestamp. It relied on
two client-driven RPCs instead — `endMinigameForPlayer` and
`minigameStartCancel` — neither of which Cimmeria implements yet. Wiring
those through to `SessionRegistry::remove` is the faithful fix; the TTL
still earns its place afterwards for the case the original had no answer to
either, a client that crashes without sending anything.

### Connection limits

The SmartFox port (TCP 30000 by default) is published on every host
interface, so anyone can open a socket to it. The original C++ host had no
connection cap and no read timeout. Cimmeria adds limits sized to how the
client behaves: one character runs at most one minigame at a time, and the
SWF sends `verChk` and `login` on its own as soon as it loads, so a player
holds one socket (briefly two while a closed SWF's socket drains) and logs
in within a second.

| Limit | Default | On breach |
|---|---|---|
| Open connections, all peers | 256 | The new socket is closed at once. INFO row `reason=total_connection_cap`, at most once a minute, with a `suppressed` count |
| Open connections per source address | 8 | The new socket is closed at once. DEBUG row `reason=per_ip_connection_cap` |
| Accept to successful `login` | 30 s | Closed. DEBUG row `reason=handshake_timeout` |
| Frame size (`MAX_MESSAGE_LEN`, the read buffer) | 4096 bytes | Closed. Before login, DEBUG `reason=preauth_message_too_long`; in a session, WARN `reason=message_too_long` and result code 0 |
| No inbound frame in a logged-in session | 30 min | The session ends as a cancel (result code 0). INFO row `reason=idle_timeout` |
| One send to the client | 10 s (`SEND_TIMEOUT`) | The connection is treated as gone: no further frames are sent, and a session ends as a cancel. DEBUG row `reason=send_timeout` |
| TCP keepalive on accepted sockets | First probe after 60 s of silence, then every 10 s, 3 probes | The OS resets the socket and the session ends as a cancel, about 90 s for an idle connection. With data in flight, TCP retransmission and the 10 s send timeout apply instead |

Ahead of these, the listener admits only peers whose IP address matches a
registered minigame session: the source address of the player's game
connection, recorded when the session starts. Any other socket is closed
before a byte is read and before a connection slot is taken. Those refusals
log a throttled INFO row `reason=unexpected_peer` (at most once a minute,
with a `suppressed` count) and a DEBUG row for each one. A session whose
game connection had no known address admits nobody, and its player's SWF
is refused.

The defaults are `ListenerLimits::default()` in
[`server/limits.rs`](../../crates/minigame/src/minigame/server/limits.rs).
`server::run_with_limits` takes other values; the server configuration has
no keys for them yet. A cap of 0 is raised to 1 with a WARN. The idle
timeout counts inbound frames only, because a Livewire board keeps sending
timer updates to a client that has gone. It is long on purpose: the board
can sit unstarted while the player reads it. Keepalive frees a vanished
peer well before that, and the send timeout keeps a client that stopped
reading from parking the game loop where neither would be noticed.

The per-address cap needs the real client address. On Linux, Docker's
default (iptables) port publishing keeps it for traffic from other hosts.
Where something in front of the port replaces it (Docker's userland proxy,
Docker Desktop, a TCP proxy), every peer shares one address and that cap
behaves like a second, lower total cap.

### Logging

Everything a peer sends before it logs in is unauthenticated, and on a
public port most of it is scanner traffic: TLS ClientHellos, HTTP requests,
RDP probes. A TLS or RDP probe has a `0x00` byte within its first few bytes,
which ends an SFS frame, so the server parses a few bytes of binary with no
`<msg>` envelope. Before 2026-10-10 the codec logged each one as the WARN
`Unknown SFS message type` with empty `msg_type` and `body_action` fields,
and the Discord warn harvest posted them.

The codec no longer logs. `parse_message` returns a `ParseError`
(`Malformed`, `NotSfs`, `UnknownType`, `BadExtensionData`), and the
connection task decides the level:

| Who sent it | Level | Fields |
|---|---|---|
| A peer that has not logged in | DEBUG | `reason=non_sfs_preauth`, `peer`, `phase`, `parse_error`, `len`, an escaped 48-character `sample`. A well-formed frame in the wrong phase logs `reason=unexpected_preauth_message`, a bad API version INFO `reason=bad_api_version` |
| A login naming an entity id with no session | DEBUG | `reason=no_session`, `peer`, `entity_id`, `game` |
| A login with the wrong ticket or game for a registered entity | INFO | `reason=ticket_mismatch` or `game_name_mismatch`, `peer`, `entity_id`, `entity_name` |
| A login with a valid ticket already held by a live connection | WARN | `reason=ticket_already_claimed`, `peer`, `entity_id`, `entity_name`, `game` |
| A logged-in session | WARN | `Unknown SFS message type` with `entity_id`, `entity_name`, `game`, `msg_type`, `body_action`, `reason` (`unknown_sfs_type`, `malformed_xml`, `no_sfs_envelope`, `bad_extension_data`) and `sample` |

DEBUG reaches SigNoz (`cimmeria_minigame=debug` in the OTLP filter), so the
pre-login rows stay queryable there. Only WARN and ERROR reach Discord. The
entity id in a login is a small integer anyone can guess, and the ticket is
256 bits from a CSPRNG, so a ticket mismatch is never a real player: it
stays at INFO so a peer looping crafted logins cannot reach Discord. Only a
login that holds the real ticket can WARN. The accept-error WARN is
throttled to once a minute with a `suppressed` count.

### Result codes

| Code | Name | When |
|---|---|---|
| 0 | Canceled | The session ended with no outcome — socket dropped, SWF closed, idle timeout, oversized frame. Inert on the cell today, but it is what the original used to clear `BSF_PlayingMinigame` and release the movement lock |
| 1 | Victory | The game reported a win. The only code that fires `on_victory_chains` |
| 2 | Defeat | The game reported a loss, including a Livewire timeout. Carries no chains |

A cancel does not emit a Discord notification; a win or loss does.

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
7. **Session-cancel RPCs** — `endMinigameForPlayer` and `minigameStartCancel` (cell method 30). Until they land, an abandoned launch is recovered by the TTL sweep rather than by the client telling the server it closed the window

## Related Docs

- [mission-system.md](mission-system.md) - Missions that require minigame completion
- [inventory-system.md](inventory-system.md) - Items used in minigames
