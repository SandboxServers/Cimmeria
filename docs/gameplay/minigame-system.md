---
title: "Minigame System"
type: reference
audience: engineers
last_updated: 2026-10-20
---

# Minigame System

> **Last updated**: 2026-10-20
> **Status**: Content-triggered minigames work end-to-end — SmartFoxServer host, ticket handshake, five server-validated games (Livewire, CrystalGame, Alignment, GoauldCrystals, ConverseBasicHumanoid), five client-declared shells (Hack, Activate, Analyze, Bypass, Converse; 60 s countdown, then defeat), and victory and failure chain callbacks. The pre-game dialog (`startMinigame`, `minigameStartCancel`) and `endCurrentMinigame` work; the helper, spectate, call and debug cell methods are still stubs.
> **Companions**: [campaign record](../analysis/minigames/README.md), [wire formats](../reverse-engineering/findings/minigame-wire-formats.md), [client architecture](../reverse-engineering/findings/minigame-architecture.md), [security audit CAT-K](../security-audit/2026-05-31-server-authority/findings/CAT-K-minigame.md), [debug-area terminals](../content/debug-area.md#minigame-terminals).

## Overview

The minigame system provides puzzle-based mini-activities integrated into the game world. Minigames are triggered by interacting with objects or NPCs, have difficulty levels and tech competency requirements, and produce result callbacks. The system supports a "helper" call protocol where players can request assistance from registered helpers, spectating other players' minigames, and NPC-triggered minigame contacts.

The `MinigamePlayer` interface in `entities/defs/interfaces/MinigamePlayer.def` is the largest interface by method count (25 properties, 78+ methods).

The original SGW minigames were Flash SWFs that connected to a **SmartFoxServer 1.x** TCP endpoint, separate from the Mercury game channel. [`crates/minigame/src/minigame/`](../../crates/minigame/src/minigame/) reimplements that server in-process: `protocol.rs` speaks the SmartFox XML packet format, `session.rs` owns the ticket registry, `server/` is the TCP listener and connection lifecycle, and `games/` holds the per-game logic behind a `MinigameInstance` trait.

Base accepts a content-triggered launch only while the queued entity and player
still belong to the current connection. It sends the existing Mercury
`onStartMinigame(URL)`, launch feedback and `onEndMinigame` methods through an
opt-in reliable sender bound to that Base connection instance and character.
An address takeover cannot give the new connection the old packet's key,
sequence, ACKs or retransmission entry. A post-send connection takeover also
returns a failed launch outcome, including for a fragmented URL, and stops
later fragments. Base logs that outcome and whether exact-ticket cancellation
succeeded, without logging the ticket itself. If the URL cannot be sent, Base
cancels that exact SmartFox ticket so the player can retry. A terminal result
deposits a one-use, server-owned receipt for the claimed ticket. The internal
message to Base contains only the ticket; Base redeems the receipt and checks
the original entity, character, and connection instance before it closes the
minigame UI or forwards to Cell. It repeats that owner check after the awaited
close. The receipt carries the victory chains only for result 1 and the
failure chains only for result 2. Explicit cancellation or a new launch
invalidates an undrained receipt; normal SmartFox teardown leaves it
available. Because a cancel refuses the connection task's own Canceled
receipt, Base sends Cell the hold release itself (result 0 once the SWF has
connected, 3 before), with no chains. A launch Base cannot carry out (no
owner, server disabled, URL send failed) and a pending session the TTL sweep
evicts also release the hold with result 3. Receipts expire after 180 seconds
and the registry keeps at most 4,096. These checks do not validate a game's
puzzle or bind the Cell launch incarnation. Castle CS-02b2 owns that later
Cell gate, and Castle callbacks remain disabled.

## How a minigame actually launches

The working path is content-driven, not client-driven:

```text
Content chain fires Action::StartMinigame { minigame_type, difficulty,
                                           on_victory_chains, on_failure_chains }
  |-> Cell: CellToBaseMsg::StartMinigame
  |-> Base: SessionRegistry::register(...) -> ticket (seed, difficulty, tech
  |         competency, victory and failure chains all captured server-side)
  |-> Base: onStartMinigame(URL) to the player
  |         URL shape: http://unused/{host}/{port}/{gameName}/{entityId}/{ticket}
  |-> Flash SWF connects to the SmartFox TCP port, presents the ticket
  |-> Game plays; result reported back
  |-> Registry: deposit one terminal receipt for the claimed ticket
  |-> Base: redeem ticket-only MinigameResult, check original Base owner,
  |         notify that owner and forward receipt-derived result to Cell
  |-> Cell: runs on_victory_chains (result 1) or on_failure_chains (2, 4)
```

The other two launch paths join this one at `CellToBaseMsg::StartMinigame`: an ability effect (MG-24, below) and the optional pre-game dialog (MG-21, below). Wire detail for every message in the flow: [spec-launch-flow.md](../analysis/minigames/spec-launch-flow.md) and [minigame-wire-formats.md](../reverse-engineering/findings/minigame-wire-formats.md).

### Failure chains and flags

- **Chains.** A `start_minigame` action takes `on_victory_chains` and `on_failure_chains` (both lists of chain ids; the failure list defaults to empty). The cell fires the victory list on result 1 and the failure list on results 2 (Defeat) and 4 (Defeated); results 0 (Canceled) and 3 (not started) fire nothing (D-MG5). Every result code releases the player's minigame hold first, so a chain starts from a free player. See [Result codes](#result-codes).
- **Load-time check.** A `start_minigame` row whose type is not an exact `GameKind` name is dropped at load (see [Data References](#data-references)).
- **Flag.** `CIMMERIA_MINIGAME_START_DIALOG` switches launches to the pre-game dialog; it defaults off until the lab UAT confirms the dialog flow, and an invalid value logs WARN `event=feature_flag_invalid` and keeps the default.

### Testing by hand

The Debug Area has one terminal per game, each launching its game through the same chain path as live content, with a win chain (24056) and a loss chain (24057): see [debug-area.md § Minigame terminals](../content/debug-area.md#minigame-terminals). For an offline run with no client, use the REPL harness (`minigame_repl`, `crates/minigame/src/bin/minigame_repl.rs`); the golden transcripts in `crates/minigame/tests/scripts/` run through it.

### Optional pre-game dialog (MG-21)

With `CIMMERIA_MINIGAME_START_DIALOG` on (default off; see the table in `crates/server/src/main.rs`), a launch first shows the pre-game dialog instead of sending `StartMinigame` at once:

- Cell sends client method 53 (`onStartMinigameDialog`) with `Name`, `Difficulty`, `TCLevel`, `Verb`, `ArchetypeBitfield`, `CanPlay`, `CanCall`, `CanSpectate`. The offer waits 60 s.
- Play sends cell 24 (`startMinigame`). A live offer launches with source `start_dialog`. A missing or expired offer, or a player already in a minigame, sends client 54 (`onStartMinigameDialogClose`) and logs a WARN.
- Cancel sends cell 30 (`minigameStartCancel`). It drops the offer and sends nothing.

Two deviations from the original flow: the movement and state hold is taken at launch, not when the dialog opens, so an unanswered dialog never locks a player; and expiry is checked lazily when cell 24 arrives.

**From an ability (MG-24).** An effect whose `script_name` is `StartMinigame` (params `minigame_type` and `difficulty`) queues a launch for the caster, and the cell starts it once the ability's effects have resolved. It takes the same path as a content launch, so the dialog switch applies here too. A non-player caster, or one already playing, queues nothing (WARN `reason=npc_caster` or `already_playing`). Script: `start_minigame.rs` in `cimmeria-cell-effect-scripts`.

**The Converse abilities (MG-62).** Ability 1327 (effect 1643) and ability 2089 (effect 2813) carry `script_name` `StartMinigame` with `minigame_type` `ConverseBasicHumanoid` and `difficulty` 1 (`effect_nvps.sql`, ids 25501-25504). The caster plays. No chain follows: the session's own result is the only outcome.

**The Prison Boot (MG-63b).** Chain 1023 casts ability 1597 on the player while mission 689 is open. Its effect 1939 (`MovementHold`) holds the wearer for movement only: item use and abilities stay open, so the wearer can still start the escape game with item 3438. Effect 1942 (`MovementRelease`, `ReleasesEffect` 1939) lifts the boot on victory (chain 1025). A minigame hold taken and released while the boot is on keeps the movement lock until the boot comes off. Death leaves the entry (effect 1939 has no `EF_ClearOnDeath`), and a respawn's hard reset keeps its hold, so the wearer stays held until the minigame's release; this is accepted. Scripts: `movement_hold.rs` in `cimmeria-cell-effect-scripts`.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| SmartFoxServer 1.x host | DONE | `minigame/server/` (`mod.rs` lifecycle, `accept.rs`, `limits.rs`, `framing.rs`, `handshake.rs`, `result_dispatch.rs`) + `protocol.rs` |
| Connection admission | DONE | One connection per valid ticket (non-public peers match as one class), a login deadline, a 4 KiB frame cap and an idle timeout on the public port. See [Connection admission](#connection-admission) |
| Session / ticket registry | DONE | `minigame/session.rs`; registration captures seed, difficulty, victory chains and original Base connection instance |
| One-use result receipt | DONE | Claimed terminal result deposits once; Base atomically redeems the ticket and checks original owner before client send and Cell forward. Expiry: 180 s; cap: 4,096. Does not prove puzzle success or Cell launch identity |
| Session expiry | DONE | A registered session whose SWF never connects is swept after `PENDING_SESSION_TTL` (180 s). See [Session lifecycle](#session-lifecycle) |
| Abort on SWF close | DONE | `run_session` calls `MinigameInstance::aborted()` and reports result code 0 (Canceled) when the socket drops without an outcome. Explicit player close removes the registration first, so its later canceled report is refused |
| Content-triggered start | DONE | `Action::StartMinigame` → `CellToBaseMsg::StartMinigame` → `onStartMinigame(URL)` |
| Victory-chain callback | DONE | `MinigameResult` forwards to the cell, which runs `on_victory_chains` |
| Livewire | DONE | Fully ported in `minigame/games/livewire/`; an abort sends nothing, the client has closed |
| CrystalGame | DONE | Fully ported in `minigame/games/crystal_game/`; validation `server`: the server decides victory and the 16 s timeout, and a client `victory` is refused (WARN `client_victory_refused`). See [CrystalGame rules](#crystalgame-rules) |
| ConverseBasicHumanoid | SERVER (MG-CH3) | `games/converse_humanoid/`: the server deals, plays, scores and decides victory; a client `victory` is refused and a client `failure` is a defeat. See the rules below |
| Hack, Activate, Analyze, Bypass, Converse | PLACEHOLDER | `games/placeholder.rs` — the shell shows a 60 s countdown (`fullgamestate` then `timerupdate`, MG-64); a client `victory` counts only after 5 s (D-MG13); when 240 ticks (60 s) pass with no `victory` the server sends `failure` and the game ends as a defeat (result code 2, `on_failure_chains` fire). Matches the original `Placeholder.py`; these game types had no real SWF beyond a shell |
| Alignment | SERVER (MG-AL3b) | `games/alignment/`: a hex-triangle matching puzzle; validation `server`: the server decides every swap, cascade, score, victory and the timeout, checks each ability cheat bit before it spends it, and refuses `processreset`. Item cheats are not ported. See [Alignment rules](#alignment-rules) |
| GoauldCrystals | SERVER (MG-GC3b) | `games/goauld_crystals/`: a path-building puzzle; the server decides victory and the clock, checks every ability cheat bit before it spends it, and refuses `processreset`. Item cheats are not ported. See [GoauldCrystals rules](#goauldcrystals-rules) |
| `startMinigame` | DONE (MG-21) | Cell method 24 accepts a live pre-game offer through `minigame_launch::accept_offer` (WARN and `onStartMinigameDialogClose` when there is none). Only reachable with `CIMMERIA_MINIGAME_START_DIALOG` on |
| `endCurrentMinigame` | DONE | Cell method 25 cancels the current session and closes SmartFox |
| Debug start / spectate / join / instance | STUB | Cell methods 20–23 log `UNIMPLEMENTED` |
| Spectating | STUB | `requestSpectateList` (26), `spectateMinigame` (27) log `UNIMPLEMENTED` |
| Helper registration | STUB | `registerToMinigameHelp` (28), `updateRegisterToMinigameHelp` (29) log `UNIMPLEMENTED` |
| Helper call protocol | STUB | `minigameCallAccept` (31), `Decline` (32), `Abort` (33) log `UNIMPLEMENTED` |
| NPC contacts | STUB | `minigameContactRequest` (34) logs `UNIMPLEMENTED` |
| Tech competency | PARTIAL | Sent as `1` on every launch (D-MG18): the original took it from an instrument item and no instrument path exists (M9). Level, intelligence and `abilityBitfield` come from the player at launch (`minigame_launch`, MG-22); the session keeps only the ability bits the game implements (`GameKind::ability_bits`) |
| `minigameStartCancel` | DONE (MG-21) | Cell method 30 drops the player's pending offer and sends nothing (`cancel_offer`). Closing a running window is `endCurrentMinigame` |
| Mob/item attempt tracking | NOT IMPL | `minigameMobAttemptTracker`, `minigameItemAttemptTracker` unused |
| Item integration | NOT IMPL | `addItemToMinigame`, `consumeItemByMinigame` unused |
| Cheat detection | NOT IMPL | `updateMinigameItemCheats` unused |

### Games at a glance

`GameKind` in `games/kind.rs` is the registry: each kind's validation mode comes from its module's `PORTED` constant, and its ability mask from `ability_bits()`.

| Game | Validation | Difficulty | Ability mask | Cheats | `processreset` |
|---|---|---|---|---|---|
| Livewire | server | 1-4 (5 uses the tier 4 table) | `0` | None yet: item and ability cheats are ignored | Accepted: resets the game and sends `resetgame` and `fullgamestate` |
| CrystalGame | server | One board at every tier | `0` | None | Refused |
| Alignment | server | 1-4 | `0x7FFF` | Ability cheats only | Refused |
| GoauldCrystals | server | 1-4 (clamped); tech level 0-10 (clamped) | `0x7FFF` | Ability cheats only | Refused |
| ConverseBasicHumanoid | server | Ignored: Logic3 then mediation5 | `0x78000` (Trump bits 15-18) | Trump passives | Refused until a deal ends, then accepted after `newdeal` |
| Hack, Activate, Analyze, Bypass, Converse | client-declared | Not used | `0` | None | n/a |

The Livewire reset is an open parity item, not a finding: it re-runs `init_game` (new board, game not started) and grants no result, but the later ports refuse a reset because it would refill the clock. Whether Livewire's should be refused too is undecided. Per-game rules: [CrystalGame](#crystalgame-rules), [Alignment](#alignment-rules), [ConverseBasicHumanoid](#conversebasichumanoid-rules), [GoauldCrystals](#goauldcrystals-rules); specs in [docs/analysis/minigames/](../analysis/minigames/README.md).

### CrystalGame rules

- Lights-out mod 6 on a fixed 14x13 board at every tier; two scramble clicks are drawn from the session seed. A click adds 2 to its crystal and 1 to each neighbour (mod 6); the goal is every cell 3.
- The 16 s clock starts on the first valid click (`timerupdate`, then one `updateslot` per changed cell) and loses 0.25 s per 250 ms tick; at 0 the server sends `failure` (defeat). It also ends on the wall clock 16 s after that click, so a stalled client cannot outrun it. A game with no click within 60 s of starting is lost the same way.
- The server decides victory: a solved board sends `victory` and stops the clock. A client `victory` is refused.
- `processreset` is ignored (WARN `reset_refused`, no reply), as in the original; a reset would let a player idle a running clock.
- No cheats: `abilityBitfield` 0, `instcc` -1; both cheat commands get a WARN and no reply. Refusals are logged at WARN for the first 32 per session only.
- Out-of-range, fractional or missing coordinates, and any click after the end, are dropped with a WARN and no reply.

### Alignment rules

- **Board.** An 8x8 hex board of piece kinds, drawn per tier (1-4) from OS entropy, not the session seed: a 32-bit seed in the client's hands would let it solve the board offline. Only the offline REPL and its goldens (`games::create_seeded`) seed the board from the script's `seed`; the live `create` path cannot be seeded.
- **Swaps.** `swappieces` names a triangle of three neighbouring cells with the pieces the client believes are there. The server checks bounds, triangle shape, blockers, empties and the types, then rotates the pieces and floods the match. A swap with no match changes nothing (`spinCount` 3).
- **Cascade.** `cascade` and `checkForCascade` step gravity and re-matching until `cascadecheckcomplete`; `doscore` then scores the server's own tally once, with a multiplier and time bonus that grow with the cascade count (capped at 4).
- **Goals and timer.** Each tier has three goal icons with totals; the countdown loses 0.25 s per 250 ms tick, and a wall-clock deadline backs it so a late tick cannot hide a timeout. At 0 the server sends `failure`. `callhint` costs 5 s.
- **Victory.** Meeting every goal sends `activate`; the server sends `victory` on `dovictory` only after that. A client `victory` is refused (WARN `client_victory_refused`).
- **Ability cheats.** `activateabilitycheat` takes `cheatNum` 0-14 and its bit must still be set in the session mask (`0x7FFF` is implemented). The bit is cleared first, `abilitycheatstate` returns the new mask, and cheat clears never score. Bit 10 has nothing to restore. An unset bit or `cheatNum` 15 is refused with no reply; `activateitemcheat` gets no reply.
- **Reset.** `processreset` is refused (WARN `reset_refused`, no reply, state unchanged), as for CrystalGame: a reset would refill the clock and the ability bits.

### ConverseBasicHumanoid rules

- **Dominance.** Two cards compare by `(p - n + 8) % 9` over the nine suit values: below 4 is the NPC's point, 4 to 7 the player's, 8 a tie (spec section 6).
- **Deals.** Logic3 deals 3 cards a side with one container, then mediation5 deals 4. A deal ends when a hand ends with every card played (`newdeal`); the timers then hold until the client sends `processreset`, which is refused before that.
- **Hands.** A hand ends on `newhand`, which is refused until the NPC has answered, or after 6 s. Each container goes to the stronger card, and a sweep of all containers adds one point.
- **NPC.** Every 1.5 s the NPC plays the card that scores best against the player's hand. It answers a play into an empty NPC container within 0.5 s.
- **Trump passives.** Ability bits 15-18 add +5, +10, +15 and +25 to the accord bonus (spec section 7).
- **Victory.** `activatevictory` spends the points of a listed condition (focusdamage1 costs 3, focusdamage2 costs 4). At zero seconds a tie or a lead is a victory; the NPC ahead is a defeat.

### GoauldCrystals rules

- **Board.** A path-building puzzle: move greens through empty cells until a green path from the start goal powers the finish goal. Yellows and oranges swap with a neighbour; reds stay put. The seed 1, 4x4 tutorial board is kept (spec section 3).
- **Clock.** The countdown starts on the first click (`timerupdate`) and loses 0.25 s per 250 ms tick. A monotonic deadline is checked in the click handler as well as on the tick, so a late tick cannot hide a timeout. At 0 the server sends `failure` (defeat).
- **Victory.** After every board change the server recomputes the powered path. A powered finish sends `fullgamestate` and `victory`, stops the clock and ends the game. A client `victory` (and either item cheat) falls into `unknown_command`: a WARN and no reply.
- **Ability cheats.** `activateabilitycheat` takes `cheatNum` as an integer 0-14, and its bit must still be set in the session mask. The server clears the bit before the effect and returns the new mask in `abilitycheatstate`. A bad number or a bit not held is refused with no reply.
- **Reset.** `processreset` is refused (WARN `reset_refused`, no reply, state unchanged). A reset would refill the clock and the ability bits: a deliberate deviation from the original, for owner confirmation.
- **Refusals.** Every refused command is counted; the first eight are logged at WARN, then one in a hundred. Input after the end is dropped.

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
| `tech_competency` | `1` on every launch (D-MG18); the original read an instrument item, and no instrument path exists (M9) |
| `seed` | `rand::random::<u32>()`. Alignment ignores it for live play and draws from OS entropy |
| `abilities` | The player's known-ability mask (MG-22), filtered to the bits the game implements (`GameKind::ability_bits`) |
| `intelligence`, `player_level` | Read from the player entity at launch |
| `on_victory_chains` | The chains to run when the game is won |
| `on_failure_chains` | The chains to run when the game ends in result 2 or 4 |

## Session lifecycle

A session exists in `SessionRegistry` from the moment the chain fires until
the connection task finishes or the owner closes the window. Three things end it:

1. **The connection task.** Login validates the ticket and claims the
   session in one locked step (`authenticate_and_claim`), so a task can
   only ever own the session it authenticated against. A ticket
   authenticates one live connection: a second login with it while the
   first is connected is refused (WARN `reason=ticket_already_claimed`).
   When the connection ends — win, loss, or the player closing the window —
   the task unregisters the session. A connected session is never expired
   by age, because a Livewire round can run longer than the TTL.

   Without an explicit close request, the task ends when the server
   notices the socket is gone: a FIN or
   RST, a send that fails or blocks past 10 s, TCP keepalive giving up
   (about 90 s for an idle connection whose peer vanished), or the 30-minute
   idle timeout. Until then a SWF that reconnects after a half-open drop is
   refused as `ticket_already_claimed`. See
   [Connection admission](#connection-admission).
2. **The window-close RPC.** `endCurrentMinigame` releases the registry entry
   and signals the SmartFox task. That task reports Canceled (0), sends its
   close sequence and closes the socket. A new interaction can register a
   fresh ticket immediately; ticket-scoped teardown cannot remove it. A
   result carries its old ticket so Base also skips `onEndMinigame` when a
   replacement ticket is already registered, even if the SWF's FIN won a
   race with the close RPC. A refused launch sends a feedback line. The
   Preparation terminal keeps its shared Livewire marker after one player's
   victory so other players still at mission 641 step 3564 can launch; the
   mission step gates completed players from starting another board.
3. **The expiry sweep.** A session whose SWF never connects has no task to
   clean it up. `spawn_sweep` runs every `SWEEP_INTERVAL` (60 s) and drops
   every unconnected session older than `PENDING_SESSION_TTL` (180 s).
   `register` also sweeps before its duplicate check, so the next
   interaction recovers without waiting for a sweep tick.

Only one session may exist per entity at a time; a second launch inside the
TTL is rejected. Before the sweep existed, an abandoned launch pinned the
entity id and every later interaction with the same object was rejected
until the player relogged.

The TTL is a **backstop, not a port**. The original had no timeout at all:
its `MinigameRequestManager::QueueEntry` carries no timestamp. Cimmeria
now handles window close through `endCurrentMinigame` and the
pre-game dialog's cancel through `minigameStartCancel`. The TTL backs up a
client that crashes without sending either call.

### While playing

A launch holds the player (D-MG15). When the content action `StartMinigame`
reaches Base, the cell sets `BSF_PlayingMinigame` (bit 4) and takes one counted
`BSF_MOVEMENT_LOCK` reference, sent as a single `onStateFieldUpdate`. Every
result code releases both: the cell's `MinigameResult` handler calls
`release_minigame_hold` for victory, failure, cancel and the rest. Because the
lock is ref-counted, a stun taken before the launch keeps its own reference
through the release. The server rejects horizontal movement while bit 4 is set
(`reason=movement_locked`): a held player who moves more than 0.5 m from where
the hold found them is snapped back. The distance is measured from that spot,
not from the last accepted packet, so small steps cannot add up, and a server
teleport during the hold moves the spot with the player. A game master is
exempt, and a stun's lock alone does not trigger the refusal. Two paths that
never reach a SmartFox connection task report result 3 so they free the player
too: a launch Base cannot carry out (minigame server disabled, no client
address) and the expiry sweep. A duplicate-launch refusal sends no result, since
the first board is still running. A window close before the SmartFox connection
opens reports result 3 from Base, since no connection task exists to do it. A
result 0 or 3 for a ticket that a newer launch has replaced is dropped, so the
old board cannot free the new board's hold. Open items: a `create_game` failure
after the claim, and a logout or world change while a session is registered, do
not report yet.

### Connection admission

The SmartFox port (TCP 30000 by default) is published on every host
interface, so anyone can open a socket to it. The original C++ host had no
read timeout and no admission rule. Cimmeria admits **one connection per
valid ticket** (#1119) and adds timeouts sized to how the client behaves: one
character runs at most one minigame at a time, and the SWF sends `verChk` and
`login` on its own as soon as it loads, so a player holds one socket and logs
in within a second. There is no global or per-address connection cap. Tickets
are the bound: an address is admitted only while its in-flight connections are
fewer than the live tickets recorded for it, and a socket with no free ticket
is closed before a byte is read. That bound holds at the moment of each accept,
not for the connections already open: a connection can outlive its ticket (see
the slot lifetime below), so for a short time the open connections from an
address can exceed its live tickets. Each such connection was admitted against
a ticket, so the count stays bounded by the tickets issued, and the excess is
only the connections still closing.

`SessionRegistry::try_admit(peer_ip)` decides, in the accept loop, before a
byte is read. A session is a free ticket for a peer when its recorded client
address (set when the session starts) matches the peer and it is either
connected or still pending inside `PENDING_SESSION_TTL`. The peer is admitted
while its in-flight connections are fewer than those tickets, so N tickets from
one address admit exactly N connections. The returned `AdmissionGuard` is held
by the connection task for its whole life and frees the slot on every exit,
including a failed or timed-out handshake.

The slot is released on cancel (owner ruling, 2026-10-21). The connection binds
its guard to its ticket once the login has claimed the session
(`SessionRegistry::bind_slot`). From then on the slot frees at the first of:

- **A cancel.** `cancel`, `cancel_reporting_connected` and `cancel_if_ticket`
  remove the session and free the slot in the same step. The old socket may
  still be draining its close frames, but a player who closes the window and
  at once reopens the minigame is admitted, not refused for the 10 s a
  stalled send can take.
- **The game loop ending**, for any reason: a closed socket, a failed read, the
  idle timeout, a finished game. The loop calls `release_slot` before the
  teardown sends, so a peer that stopped reading cannot hold the slot for
  `SEND_TIMEOUT`. The session stays registered until the task removes it, so
  the result still deposits; a new launch for the same entity registers once
  that removal has run.
- **The connection task ending**, which drops the guard. This is the only path
  for a failed or timed-out handshake, and for a login with the wrong ticket or
  game, which closes the connection at once and leaves the session pending so
  the player can retry.

The release happens once; a later drop does not take a slot that already
belongs to the next connection. A second concurrent connection on a ticket that
is still being played is refused as `no_free_ticket`, and so is one whose
`release_slot` names a different ticket than the one bound.

Matching is by address, with one rule for private networks: every non-public
peer (loopback, 10/8, 172.16/12, 192.168/16, CGNAT 100.64/10, IPv4 link-local,
IPv6 ULA and link-local) is one class. On a LAN or a Docker host the TCP peer
can show a different private address from the one recorded at
`start_minigame`, so a non-public peer matches any non-public ticket. IPv4-mapped
IPv6 peers are canonicalized first.

A refused socket is closed with no byte sent. Three INFO rows record why:

| `reason` | When | Logging |
|---|---|---|
| `unexpected_peer` | No session is registered at all (most likely a scanner) | At most once a minute, with a `suppressed` count |
| `unexpected_peer_with_sessions` | Sessions are registered but none expects this address; a real player behind NAT or multiple WAN links may be the one refused | Not throttled; carries `pending_sessions` |
| `no_free_ticket` | The address has tickets, and every one already has a live connection | At most once a minute, with `suppressed`; carries `pending_sessions` |

Each refusal is INFO, never WARN: only WARN and ERROR reach Discord. A
session is not registered without the player's client address: the start is
refused with a WARN row `reason=no_client_addr`, and no `onStartMinigame` is
sent.

Other limits on the public port (`ListenerLimits::default()` in
[`server/limits.rs`](../../crates/minigame/src/minigame/server/limits.rs) holds
the two timeouts):

| Limit | Default | On breach |
|---|---|---|
| Accept to successful `login` | 30 s | Closed. DEBUG row `reason=handshake_timeout` |
| Frame size (`MAX_MESSAGE_LEN`, the read buffer) | 4096 bytes | Closed. Before login, DEBUG `reason=preauth_message_too_long`; in a session, WARN `reason=message_too_long` and result code 0 |
| No inbound frame in a logged-in session | 30 min | The session ends as a cancel (result code 0). INFO row `reason=idle_timeout` |
| One send to the client | 10 s (`SEND_TIMEOUT`) | The connection is treated as gone: no further frames are sent, and a session ends as a cancel. DEBUG row `reason=send_timeout` |
| TCP keepalive on accepted sockets | First probe after 60 s of silence, then every 10 s, 3 probes | The OS resets the socket and the session ends as a cancel, about 90 s for an idle connection. With data in flight, TCP retransmission and the 10 s send timeout apply instead |

`server::run_with_limits` takes other timeouts; the server configuration has
no keys for them yet. The idle timeout counts inbound frames only, because a
Livewire board keeps sending timer updates to a client that has gone. It is
long on purpose: the board can sit unstarted while the player reads it.
Keepalive frees a vanished peer well before that, and the send timeout keeps a
client that stopped reading from parking the game loop where neither would be
noticed.

Admission by address needs the real client address. On Linux, Docker's
default (iptables) port publishing keeps it for traffic from other hosts.
Where something in front of the port replaces it (Docker's userland proxy,
Docker Desktop, a TCP proxy), every peer shows the proxy's private address.
That peer matches only tickets recorded for a non-public address, so a player
whose ticket carries a public address is refused as
`unexpected_peer_with_sessions`.

What #1119 asked for, and where it stands:

| Item | State |
|---|---|
| Querying refusals in SigNoz | **Present as log fields only.** Every refusal is an INFO log row whose message starts `Minigame connection refused` and carries `peer` and `reason`. Filter the logs on the `reason` attribute. No saved query or dashboard is shipped. **Deferred** |
| Carrier-grade-NAT signal | **Present, under a different name.** The issue called it `ip_not_bound` with `pending_sessions > 0`. The shipped reason is `unexpected_peer_with_sessions` (D-04 kept the merged names), unthrottled and carrying `pending_sessions`, so filter on that reason with `pending_sessions > 0`. Nobody is alerted on it yet: watching it is manual. A CGNAT player whose minigame socket leaves from a different public address than the one recorded at `start_minigame` shows up there; see also the proxy case below, which produces the same row |
| Spectating | **Deferred.** Admission needs a ticket for every connection and nothing here admits a ticket-less spectator. The issue puts it out of scope for a separate issue; `requestSpectateList` and `spectateMinigame` stay stubs (see Remaining Work). Nothing in the admission code rules it out |

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
| 2 | Defeat | The game reported a loss, including a Livewire timeout. Fires `on_failure_chains` (D-MG5) |
| 3 | Interrupted (not started) | The launch never reached a running board: no minigame server, no client address, the expiry sweep, or a window close before the connection opened. Frees the player, fires nothing |
| 4 | Defeated | Accepted by the cell and fires `on_failure_chains` like 2; the SmartFox host does not send it today |

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
- **Load-time name check**: `content-engine/src/loader/action.rs::MINIGAME_TYPE_NAMES` copies `GameKind::name`; a `start_minigame` row whose target is not an exact, case-sensitive match is dropped at load with WARN `reason=unknown_minigame_type` (MG-64). Keep it in step with `games/kind.rs`.
- **Result codes**: `MINIGAME_RESULT_*` constants
- **Archetypes**: Bitmask of `EArchetype` values

## Remaining Work

1. **Item cheats** — Alignment and GoauldCrystals ignore `activateitemcheat` (no item-validation call into the cell yet, M8); Livewire ignores both cheat kinds (MG-LW1 delta 11)
2. **Livewire `processreset`** — still accepted, unlike the other ports; decide whether to refuse it for parity
3. **Tech competency** — still `1` (D-MG18); needs an instrument path (M9)
4. **Helper call protocol** — the whole request → PhaseTwo → accept/decline flow (cell 31-34), including tip cash movement
5. **Spectating** — `requestSpectateList` / `spectateMinigame` (26, 27)
6. **Helper registration and debug methods** — `registerToMinigameHelp` (28, 29) and the debug starts (20-23); see CAT-K-02 before implementing the debug set
7. **NPC contacts** — contact acquisition and expiry mechanics
8. **Dialog default** — `CIMMERIA_MINIGAME_START_DIALOG` stays off until the lab UAT of the dialog flow passes (mission 582 offer path included)
9. **Unreported teardown** — a `create_game` failure after the claim, and a logout or world change while a session is registered, do not report a result yet

## Related Docs

- [mission-system.md](mission-system.md) - Missions that require minigame completion
- [inventory-system.md](inventory-system.md) - Items used in minigames
- [Minigame campaign](../analysis/minigames/README.md) - decisions, packets and the per-game specs: [protocol common](../analysis/minigames/spec-protocol-common.md), [launch flow](../analysis/minigames/spec-launch-flow.md), [shells](../analysis/minigames/spec-shells.md), [Livewire parity](../analysis/minigames/spec-livewire-parity.md), [CrystalGame](../analysis/minigames/spec-crystalgame.md), [Alignment](../analysis/minigames/spec-alignment.md), [GoauldCrystals](../analysis/minigames/spec-goauldcrystals.md), [ConverseBasicHumanoid](../analysis/minigames/spec-conversebasichumanoid.md)
- [minigame-wire-formats.md](../reverse-engineering/findings/minigame-wire-formats.md) - Mercury and SmartFox message tables
- [minigame-architecture.md](../reverse-engineering/findings/minigame-architecture.md) - client-side analysis and the game type table
- [CAT-K-minigame.md](../security-audit/2026-05-31-server-authority/findings/CAT-K-minigame.md) - server-authority findings and their status
- [debug-area.md](../content/debug-area.md#minigame-terminals) - the six test terminals
