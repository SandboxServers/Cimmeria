# CAT-K — Minigame

## Trust posture

The minigame system has a deliberately **good** architecture for server authority — content
chains issue 256-bit random tickets, the client connects to an in-process SmartFoxServer-
compatible TCP server with the ticket, and the SFS server (`crates/services/src/minigame/server.rs`)
is the **only** producer of `CellToBaseMsg::MinigameResult` that fires victory chains.
The client never directly asserts "I won." All seventeen `Event_NetOut_Minigame*` and
`Event_NetOut_debug*Minigame` wire messages currently land in cell-method handlers that
log `UNIMPLEMENTED` (`crates/services/src/cell/cell_methods/minigame.rs`) — none of them
mutate game state in the current Rust server.

However, the architecture has a **load-bearing hole**: the `PlaceholderGame`
(`crates/services/src/minigame/games/placeholder.rs`) handles every minigame except Livewire
(Hack, Activate, Analyze, Bypass, Converse, ConverseBasicHumanoid) and instant-victories
on receipt of the client SFS message `cmd=victory` with no challenge or validation. For
those minigame types the SFS server simply rubber-stamps whatever the client claims, which
is exactly the "client self-declares success" trust violation the minigame-systems-advisor
explicitly warns against. Mission gating on Hack/Bypass/Analyze/Activate/Converse is
trivially bypassable: any client with a valid ticket sends `<xtReq>cmd=victory</xtReq>`
and the `on_victory_chains` fire.

Beyond that core issue, ticket lifecycle has a DoS-shaped gap (no TTL, no IP binding),
several `<Exposed/>` MinigamePlayer cell methods are designed to take a client-supplied
target / instance / contact id with no validation (currently UNIMPLEMENTED — but the wire
shape is on file as the next contributor's design target), and the cross-domain XML policy
the SFS server emits is wide open.

Findings below are prioritised by exploitability assuming the unimplemented handlers stay
that way and the placeholder remains in place.

---

### CAT-K-01 — PlaceholderGame instant-victory on client `cmd=victory`

**Severity**: Critical
**Class**: Server-authority bypass / mission gating bypass
**Wire surface**: SmartFoxServer XT request (`<xtReq>cmd=victory</xtReq>`) over the TCP
minigame port — NOT a Mercury `Event_NetOut_*`, but the result of the StartMinigame URL
the server hands the client
**Demonstrable / Likely-theoretical**: Demonstrable (server code self-incriminates)

**Trust violation**
For every minigame type except Livewire (i.e. Hack, Activate, Analyze, Bypass, Converse,
ConverseBasicHumanoid — all the SWF-based ones that aren't yet ported from Python), the
SFS-side game instance is `PlaceholderGame`, whose `message()` handler responds to the
client command `victory` with `[Send("victory"), GameOutput::Victory]` — no challenge,
no time check, no progress tracking, no proof-of-play. The server immediately fires the
victory chains the content engine attached to the session. The cross-disciplinary
minigame-systems-advisor states "Client cannot self-declare success — the result the
cell sees is whatever the minigame server decided." This placeholder violates that rule
directly: the only thing "the minigame server decided" is to trust the client's claim.

**Evidence**
- Ghidra: `019b31c4` `Event_NetOut_MinigameComplete` — client wire type exists, but the
  exploit doesn't need it; the SFS connection is sufficient.
- Ghidra: `01841adc` `Event_SlashCmd_MinigameComplete` registered in
  `CMERegistry__RegisterAllEventEmitHandlers` (`005ca670`) — confirms a client console
  console `/MinigameComplete` slash command exists, though the active path is the SFS
  `cmd=victory`.
- Client behavioral log: n/a (server-side stub bypass).
- Cross-ref to Rust handler (for the fix author):
  `crates/services/src/minigame/games/placeholder.rs:39-47` — the `"victory"` arm.

**Attack scenario**
1. Player accepts a mission whose advance step uses content action
   `StartMinigame { minigame_type: "Hack", on_victory_chains: [X] }`.
2. Server registers a session with a 256-bit ticket and pushes the connect URL via
   `onStartMinigame(URL)`.
3. The legitimate Flash client would play the Hack puzzle. Instead, the attacker uses a
   trivial TCP client that connects to the minigame port, sends the verChk + login (with
   the ticket extracted from the URL), then sends
   `<msg t='xt'><body action='xtReq'><![CDATA[<dataObj><var n='cmd' t='s'>victory</var></dataObj>]]></body></msg>`.
4. `PlaceholderGame::message("victory")` returns `GameOutput::Victory` → SFS server emits
   `CellToBaseMsg::MinigameResult { result_code: 1, on_victory_chains: [X] }` → cell fires
   chain X → mission step completes.

**Suggested remediation (one line)**
Until the per-game Rust implementations land, stub these games with a server-driven timer
(e.g. minimum 30s) and reject `victory` before the timer elapses; longer-term, port the
Python game logic so the server decides victory from observed gameplay events.

**Would benefit from x64dbg trace?**
No — the server code is self-evidently bypassable; reproducing the exploit is a 50-line
Python TCP client, not a debugger session.

---

### CAT-K-02 — MinigamePlayer "debug*" cell methods exposed on regular SGWPlayer

**Severity**: High (latent — currently UNIMPLEMENTED)
**Class**: GM-only command exposed on non-GM entity
**Wire surface**: `Event_NetOut_debugStartMinigame`, `Event_NetOut_debugSpectateMinigame`,
`Event_NetOut_debugJoinMinigame`, `Event_NetOut_debugMinigameInstance`
**Demonstrable / Likely-theoretical**: Likely-theoretical (handlers exist as stubs; design
intent if implemented as-named violates server authority)

**Trust violation**
`entities/defs/interfaces/MinigamePlayer.def` declares `debugStartMinigame`,
`debugSpectateMinigame`, `debugJoinMinigame`, `debugMinigameInstance` as `<Exposed/>` cell
methods (lines 352–367). Every SGWPlayer implements MinigamePlayer
(`entities/defs/SGWPlayer.def:8`). The "debug" prefix conveys GM-only intent — and indeed
`SGWGmPlayer.def:395` has the **parallel** `gmDebugStartMinigame` that is the real GM
surface. The MinigamePlayer "debug*" versions are a stranded duplicate set that any
non-GM client can dispatch through the standard cell-method router
(`crates/services/src/cell/cell_methods/minigame.rs:31-62`, indices 20–23). Currently
all four handlers just log `UNIMPLEMENTED: debug*Minigame` and don't mutate state, so
this is latent. But the wire is open, the dispatch is wired, and the next contributor
implementing "debugStartMinigame" against the def-driven flat method index will
implement a non-GM debug surface unless they recognise the duplicate.

**Evidence**
- Ghidra: `019b3114` `Event_NetOut_debugStartMinigame`, `019b3148` `debugSpectateMinigame`,
  `019b3188` `debugJoinMinigame`, `019acf08` `debugMinigameInstance` — all real client
  emit points.
- Client behavioral log: n/a (UI exposure is GM client; wire is unconditional).
- Cross-ref to Rust handler:
  `crates/services/src/cell/cell_methods/minigame.rs:31-62` (the four `UNIMPLEMENTED`
  arms at indices 20–23).

**Attack scenario** (presumed if implemented as named)
1. A future PR implements `debugStartMinigame(gameId)` against the MinigamePlayer slot 20.
2. Non-GM client sends a method-call packet with `method_idx = 20 + flatten_offset` and
   payload `INT32 aGameId`.
3. Cell dispatcher routes to the new handler, which (per the "debug" naming) lets the
   client start any minigame ID without satisfying the chain prerequisite that normally
   gates it.
4. Mission chains gated on minigame completion become trivially skippable.

**Suggested remediation (one line)**
Either remove the four MinigamePlayer "debug*" `<Exposed/>` entries (preferred — keep
only the SGWGmPlayer parallel methods), or hard-gate the cell-method handlers behind
the same `access_level > 0` check the SGWGmPlayer entity selection uses
(`crates/services/src/base/world_entry/play_character.rs:89-94`).

**Would benefit from x64dbg trace?**
No — the wire slot is unambiguously named "debug" and is unambiguously on the non-GM
interface; the def file is the authority.

---

### CAT-K-03 — Minigame session has no TTL → mission stall via no-connect

**Severity**: Medium
**Class**: DoS / TOCTOU
**Wire surface**: Any chain action that fires `Action::StartMinigame` (player has no direct
control over the trigger, but can refuse to follow through)
**Demonstrable / Likely-theoretical**: Demonstrable (`SessionRegistry` records `created_at`
but never reads it)

**Trust violation**
`SessionRegistry::register` (`crates/services/src/minigame/session.rs:60-96`) rejects a
second registration for the same `entity_id`, but **never evicts stale sessions**.
`created_at` is captured but `register()` and `authenticate()` don't consult it. A session
is removed only when the SFS connection lifecycle completes
(`crates/services/src/minigame/server.rs:356`). If the player never connects to the
minigame port — which they can choose to do by simply ignoring the `onStartMinigame(URL)`
client method — the session lives forever, and every subsequent `StartMinigame` chain
action for that entity logs `Failed to register minigame session (duplicate?)` and
silently drops the `on_victory_chains` (`crates/services/src/base/world_entry/cell_dispatch/minigame.rs:79-84`).

**Evidence**
- Ghidra: n/a — server-only behaviour. The client emit that triggers the chain is e.g.
  `Event_NetOut_Interact` (`docs/protocol/message-catalog.md` interact rows), which is
  legitimate and out of scope.
- Client behavioral log: n/a.
- Cross-ref to Rust handler:
  `crates/services/src/minigame/session.rs:60-96` (`register`, no TTL check),
  `crates/services/src/minigame/session.rs:124-127` (`remove`, only called by SFS
  lifecycle exit).

**Attack scenario**
1. Player interacts with a mission NPC whose chain fires `Action::StartMinigame`.
2. Server registers session, sends `onStartMinigame(URL)` to client.
3. Client discards the URL (modified client, dropped packet, or `/MinigameStartCancel`
   slash command — see CAT-K-04).
4. Player re-interacts with the same NPC (or any other minigame-triggering chain).
5. `SessionRegistry::register` returns `None` because the entity already has a session;
   `cell_dispatch::minigame::start_minigame` logs the warn and does not push
   `onStartMinigame`. The player is silently stuck — no minigame UI, no progress.
6. The mission is now uncompletable until the player relogs (and even then, only if
   process restart wipes the in-memory registry — which it does, since the registry isn't
   persisted, but a long-lived server holds the stale session indefinitely).

This is exploitable by a **griefer** if `entity_id` collisions are possible (they're per-
world-entity), but more practically it is a **self-DoS** that makes the player look at
the mission and conclude "the game is broken." From a server-authority standpoint, this
is a missing TTL: the registry trusts that the player will follow through, which they
don't have to.

**Suggested remediation (one line)**
Add a TTL sweep (e.g. 60s without a successful SFS authenticate) inside `SessionRegistry`
and evict stale sessions; also have `cell_dispatch::minigame::start_minigame` evict any
existing session for the entity before registering (the chain action is the authoritative
"please start a new minigame" signal — superseding the abandoned one is correct).

**Would benefit from x64dbg trace?**
No — no client-side knowledge required; the server data structure is the witness.

---

### CAT-K-04 — `minigameStartCancel` / `endCurrentMinigame` cell methods accept no authentication of the cancel actor

**Severity**: Medium (latent — currently UNIMPLEMENTED)
**Class**: Authority — actor identity vs target session
**Wire surface**: `Event_NetOut_MinigameStartCancel`, `Event_NetOut_EndMinigame`
(MinigamePlayer cell methods 30 and 25)
**Demonstrable / Likely-theoretical**: Likely-theoretical (handlers are `UNIMPLEMENTED`,
but the def shape declares them `<Exposed/>` with client-supplied IDs)

**Trust violation**
`MinigamePlayer.def` `endCurrentMinigame` takes no args (lines 406-408) but the Rust
stub in `cell_methods/minigame.rs:76-90` unmarshals a 12-byte payload of
`(game_id, winner_id, loser_id)` from client. **The implementing contributor will need
to decide which side is canonical.** If they implement against the unmarshaled wire
shape, the client supplies `winner_id` and `loser_id` directly — a clean spoof of who
"won" a multi-player minigame. Similarly `minigameStartCancel` (index 30, no args in def)
unmarshals `game_id` from client — currently no verification that the canceler is the
session owner. The session is keyed by `entity_id` on the server side, so the right
implementation is "cancel the session for `entity_id == caller`, ignoring any client-
supplied game_id." But the stub is set up to read the client's game_id, which guides
the next contributor toward the wrong design.

**Evidence**
- Ghidra: `019be408` `Event_NetOut_EndMinigame`, `019be4b8`
  `Event_NetOut_MinigameStartCancel` — real client emits.
- Client behavioral log: n/a.
- Cross-ref to Rust handler:
  `crates/services/src/cell/cell_methods/minigame.rs:76-90` (END_CURRENT — unmarshals 12
  bytes vs def's 0),
  `crates/services/src/cell/cell_methods/minigame.rs:131-136` (START_CANCEL — unmarshals
  game_id from client).

**Attack scenario** (presumed if implemented against the current arg unmarshal shape)
1. Player A and Player B are in a 2-player minigame instance (e.g. a shared Livewire
   instance — the def supports this via `joinMinigame`).
2. Player A sends `endCurrentMinigame` with `winner_id = A` and `loser_id = B`.
3. Server (in the wrong implementation) trusts the client-supplied winner/loser,
   awards the victory chain to A even though the SFS instance hadn't completed.

**Suggested remediation (one line)**
Drop the client-supplied unmarshals; the server already knows `entity_id == caller`
and looks up the session by that key — the per-handler design must derive winner/loser
from server state (the SFS instance), not from client args.

**Would benefit from x64dbg trace?**
Yes — confirming the actual NetOut payload shape (does Event_NetOut_EndMinigame really
serialize 12 bytes, or is the Rust stub wrong about the wire) needs a live debugger or
a Ghidra trace of the EventHandler::vfunc_0 serialiser. The def says zero args.

---

### CAT-K-05 — `spectateMinigame(playerId)` accepts client-chosen target with no perception check

**Severity**: Medium (latent — currently UNIMPLEMENTED)
**Class**: Information disclosure / privacy
**Wire surface**: `Event_NetOut_SpectateMinigame`, `Event_NetOut_RequestSpectateList`
**Demonstrable / Likely-theoretical**: Likely-theoretical (UNIMPLEMENTED, but the wire
takes a client-supplied INT32 `playerId`)

**Trust violation**
`MinigamePlayer.def` `spectateMinigame` (lines 449-452) is `<Exposed/>` and takes
`INT32 playerId`. There is no def-level constraint that the spectator be in the same
group/squad/world as the target, or that the target's session is set to allow
spectators. The stub at `cell_methods/minigame.rs:99-104` unmarshals the player id but
doesn't act. The implementing contributor needs to validate that:
- the target player is in an active minigame session,
- the target's session has `allow_spectators` (no such field exists yet),
- the spectator is in the target's AoI or is otherwise authorised (group, squad, GM).

Without those checks, any client can spectate any other player's minigame, which is
both a privacy/info-disclosure issue (sees the puzzle state) and a co-op-cheat enabler
(off-screen friend can tell the spectated player which wire is the goal).

**Evidence**
- Ghidra: `019be498` `Event_NetOut_SpectateMinigame`, `019be474`
  `Event_NetOut_RequestSpectateList`.
- Client behavioral log: n/a.
- Cross-ref to Rust handler:
  `crates/services/src/cell/cell_methods/minigame.rs:98-104` (SPECTATE, UNIMPLEMENTED).

**Attack scenario** (presumed if implemented without check)
1. Attacker queries `RequestSpectateList` for game_id covering all active sessions in
   the world.
2. Attacker calls `spectateMinigame(targetPlayerId)` for any returned id.
3. Server pipes the target's SFS XT frames to the attacker via
   `onSpectateList`/equivalent client method, leaking the puzzle state in real time.

**Suggested remediation (one line)**
Before piping spectator frames, require that the target session's `allow_spectators`
flag is set (add the field to `MinigameSession`) and that the spectator is either in
the target's group/squad or in the target's AoI.

**Would benefit from x64dbg trace?**
No — the design constraint is clear from the def; the wire shape is just a single INT32.

---

### CAT-K-06 — `minigameCallRequest` accepts client-named recipient with no contact / rate gate

**Severity**: Medium (latent — currently UNIMPLEMENTED)
**Class**: Spam / griefing / unsolicited interaction
**Wire surface**: `Event_NetOut_MinigameCallRequest`
**Demonstrable / Likely-theoretical**: Likely-theoretical (UNIMPLEMENTED, but def shape
is clear)

**Trust violation**
`MinigamePlayer.def` `minigameCallRequest` is a **base method** (line 300) `<Exposed/>`
with args `(RemotePlayerName: WSTRING, TipAmount: INT32)`. The client picks the target
by name and supplies a "tip amount" (which translates to in-game currency the caller
allegedly promises to pay). Recipient resolution will need to scan online players by
name — which is fine — but the server design must also enforce:
- the caller is on the recipient's contact list (per the `minigameRegistrationInfo`
  protocol), OR the recipient has `wantsRequests = true`,
- a rate limit per caller (otherwise this is a free spam channel — every accept dialog
  the recipient sees is one the caller can fire),
- the `TipAmount` must be debited atomically with the tip-claim flow (otherwise the
  tip is a free promise — client can claim any value).

Currently UNIMPLEMENTED. Cell-method version at index 34
(`cell_methods/minigame.rs:159-170`) unmarshals `(target_entity_id, game_def_id)` —
which doesn't even match the base-method def (`RemotePlayerName: WSTRING, TipAmount:
INT32`). Whichever direction the implementing contributor takes, the validation matrix
above needs to land before the handler is wired up.

**Evidence**
- Ghidra: `019be4dc` `Event_NetOut_MinigameCallRequest`.
- Client behavioral log: n/a.
- Cross-ref to Rust handler:
  `crates/services/src/cell/cell_methods/minigame.rs:159-170` (CONTACT_REQUEST — note
  this maps to `minigameContactRequest`, NOT `minigameCallRequest`; the cell-method
  index for the call request itself isn't in the current dispatch table).

**Attack scenario** (presumed if implemented without validation)
1. Attacker scripts a 10-Hz loop sending `minigameCallRequest("Victim", 0)`.
2. Victim sees a `minigameCallDisplay` popup every 100ms — full UI takeover until they
   relog.

**Suggested remediation (one line)**
Hard rate-limit `minigameCallRequest` per caller (e.g. one outbound call per 30s), reject
if the recipient's `minigameRegisteredWantsRequests` is false and the caller isn't a
mutual contact, and lock the `TipAmount` against the caller's wallet at send time (debit
on accept, refund on decline/timeout/abort).

**Would benefit from x64dbg trace?**
Yes — confirming `Event_NetOut_MinigameCallRequest` payload includes the full set of
args the def declares (`RemotePlayerName`, `TipAmount`) and not just the `target_entity_id`
the stub unmarshals would help the implementing contributor.

---

### CAT-K-07 — Minigame ticket has no IP / connection binding

**Severity**: Low
**Class**: Session hijack (defense in depth)
**Wire surface**: SFS TCP login (`<msg t='sys'><body action='login'>`) on the minigame port
**Demonstrable / Likely-theoretical**: Demonstrable (the auth check only validates
`(entity_id, ticket, game_name)`)

**Trust violation**
`SessionRegistry::authenticate` (`crates/services/src/minigame/session.rs:99-121`)
validates the ticket and the requested game name but not the client IP. The `handle_connection`
function (`crates/services/src/minigame/server.rs:94-358`) accepts the connection's `peer`
address only for tracing, never for binding. The `onStartMinigame(URL)` push to the player
goes over UDP Mercury without encryption, so any path that exposes that packet (a MITM,
shoulder-surfing, debug log, future telemetry stream) lets a third party who learns the
ticket play the minigame on the player's behalf. Once they do, the legitimate player —
who's also racing for the connection — finds their session has been authenticated by
someone else and their own SFS handshake will produce the same session, since
`authenticate()` doesn't burn the ticket (it just looks it up; the session is removed only
at SFS connection close).

Two players using the same ticket would both be allowed through `authenticate()`. Both
spawn independent `LivewireGame` instances seeded from the same `session.seed`, both compete
to send `cmd=victory`, and **whichever completes first wins** — the SFS server clones the
session in authenticate and the registry's `remove(entity_id)` is called only on
connection close.

**Evidence**
- Ghidra: n/a — server-only flaw. The wire that leaks the ticket is Mercury
  `onStartMinigame(URL)` (`Event_NetIn_onStartMinigame` if you trace it inbound on the
  client side).
- Client behavioral log: n/a.
- Cross-ref to Rust handler:
  `crates/services/src/minigame/session.rs:99-121` (authenticate doesn't burn the ticket,
  doesn't check IP),
  `crates/services/src/minigame/server.rs:81-90` (peer captured for tracing only).

**Attack scenario**
1. Attacker obtains a ticket by any means (intercept the UDP packet carrying
   `onStartMinigame(URL)`, observe a server log, shoulder-surf the client's debug overlay).
2. Attacker connects to the minigame port with the same ticket and entity_id, completes
   `verChk` + `login`, and authenticates successfully.
3. Attacker plays the minigame (or sends `cmd=victory` for placeholder games, per
   CAT-K-01); the victim's chain fires from the attacker's session.

**Suggested remediation (one line)**
Burn the ticket atomically inside `authenticate()` (drop the session from the registry on
first successful auth, not on connection close) and optionally bind the ticket to the
calling player's currently-recorded UDP peer in `connected` so a different IP is rejected.

**Would benefit from x64dbg trace?**
No — server-side data structure proves the gap.

---

### CAT-K-08 — `BaseToCellMsg::MinigameResult` handler fires chains with no idempotency / mission-context check

**Severity**: Low (defense in depth — currently only one producer, but the design is
fragile)
**Class**: Idempotency / double-fire
**Wire surface**: Internal — `CellToBaseMsg::MinigameResult` (originates from the SFS
server) → `BaseToCellMsg::MinigameResult`
**Demonstrable / Likely-theoretical**: Likely-theoretical (no current producer fires twice;
a future contributor adding a second producer would silently break)

**Trust violation**
`base_messages/mod.rs:247-266` accepts `BaseToCellMsg::MinigameResult { entity_id,
result_code, on_victory_chains }` and unconditionally calls `fire_chain_by_id` for every
chain on victory. `fire_chain_by_id` (`event_dispatch/mod.rs:51-80`) "bypasses trigger
matching" — it has no mission-state check, no "has this chain already fired for this
entity" record. The current implementation is safe because the only producer
(`minigame/server.rs:39-45`) sends one message per session and removes the session before
exiting. But:
- A future cell handler implementing client-side `endCurrentMinigame` or
  `minigameContactRequest` may also need to fire chains, and if it routes through the
  same `BaseToCellMsg::MinigameResult`, no de-dup is in place.
- If the SFS server's `handle_connection` ever errors after sending the result but before
  the session is removed, a reconnect can re-authenticate and produce a second result.

**Evidence**
- Ghidra: n/a — internal Rust message flow.
- Cross-ref to Rust handler:
  `crates/services/src/cell/service/base_messages/mod.rs:247-266` (no idempotency),
  `crates/services/src/cell/content/event_dispatch/mod.rs:47-80` (fire_chain_by_id bypasses
  every gate).

**Attack scenario** (latent)
1. (Future) The session-removed-on-close ordering is changed in `handle_connection` to
   "send result, then send leave, then remove session" — and a runtime panic between
   send-result and remove-session leaves the session intact.
2. The player reconnects with the same ticket (which now isn't burned per CAT-K-07) and
   plays again.
3. The chain fires a second time — duplicate XP, duplicate item, duplicate mission
   advance.

**Suggested remediation (one line)**
Inside `BaseToCellMsg::MinigameResult` handling, record `(entity_id, session_id_or_ticket_hash)`
and refuse to re-fire chains for an entity whose previous chain firing is still
in-progress, or atomically delete the session and use its absence as the gate.

**Would benefit from x64dbg trace?**
No.

---

### CAT-K-09 — `RegisterToMinigameHelp` / `UpdateRegisterToMinigameHelp` accept self-registration with no skill check

**Severity**: Low (latent — UNIMPLEMENTED; minor griefing surface if implemented blindly)
**Class**: Authority — claimed-skill vs actual-skill
**Wire surface**: `Event_NetOut_RegisterToMinigameHelp`, `Event_NetOut_UpdateRegisterToMinigameHelp`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
`MinigamePlayer.def` `registerToMinigameHelp` (lines 454-458) is `<Exposed/>` and takes
`(note: WSTRING, inRangeOnly: UINT8)`. The handler is supposed to mark the caller as
"available to help with minigames" so other players can call them via `minigameCallRequest`.
The implementing contributor must validate that the registrant actually meets the per-game
prerequisite (e.g. a `Hack` helper needs a minimum tech_competency or specific ability
unlock). Without that, any low-level character can register as a helper, accept the
tip, and then fail every call they accept.

Cell-method stub at `cell_methods/minigame.rs:105-117` parses
`(game_def_id, help_level)` — which doesn't match the def either; the def has
`(note: WSTRING, inRangeOnly: UINT8)`. Same arg-shape mismatch as CAT-K-04 and CAT-K-06.

**Evidence**
- Ghidra: `019be424` `Event_NetOut_RegisterToMinigameHelp`, `019be448`
  `Event_NetOut_UpdateRegisterToMinigameHelp`.
- Cross-ref to Rust handler:
  `crates/services/src/cell/cell_methods/minigame.rs:105-130`.

**Attack scenario** (presumed if implemented without skill check)
1. Low-level griefer registers as a helper for `Hack` minigames with a high
   `minigameRegistrationCost` (tip).
2. Players in need of `Hack` help see the griefer in the helper list and call them.
3. Griefer accepts the call (charging the tip), then deliberately fails the minigame.

**Suggested remediation (one line)**
Before persisting the registration, look up the registrant's per-game minimum prerequisite
(probably from `Atrea.cooked_data` or a server-side resource table) and reject the
registration if unmet.

**Would benefit from x64dbg trace?**
No.

---

### Not Filed

- **`processmove` rate limiting in Livewire.** A bot client could send `processmove` for
  every wire faster than humanly possible. The server validates each cut (wire exists,
  not already cut, correct prefix, game started) so each individual operation is sound;
  bot detection is out of scope for the server-authority audit. Not filed.

- **Cross-domain XML policy `allow-access-from domain='*' to-ports='{external_port}'`.**
  This is required for the Flash XMLSocket protocol that the original SWFs spoke — narrowing
  the policy breaks legitimate clients. The policy is informational only on the SFS port
  (no cross-origin requests can authenticate without a valid 256-bit ticket). Not filed.

- **`updateMinigameItemCheats(instcc, CA0..CA4)` base method.** The name suggests
  client-asserted "cheat activations" (consumable/instrument uses), which sounds alarming.
  But the def has it as a CELL_PRIVATE-mailbox-callable base method without `<Exposed/>` —
  the wire path is server-driven (Atrea computes the cheat-class from the equipped
  instrument and pushes it down). Currently UNIMPLEMENTED in Rust. Not filed; the def
  protects it.

- **`addItemToMinigame` / `remItemFromMinigame` / `consumeItemByMinigame` base methods.**
  All non-`<Exposed/>` (lines 249-267 in `MinigamePlayer.def`). Server-to-server only.
  Not filed.

- **`onStartMinigame(URL)` URL string format.** The URL the server builds is
  `http://unused/{ip}/{port}/{gameName}/{entityId}/{ticket}`. The client parses this to
  extract the connect info. If an attacker can inject into the `gameName` field (e.g.
  via a content-defined chain) they could control part of the URL. But `gameName` comes
  from `Action::StartMinigame { minigame_type }` which is server-defined in content data,
  not client-supplied. Not filed.

- **`SGWGmPlayer` debug methods (`gmDebugStartMinigame`, `debugMinigameComplete`,
  `gmGiveMinigameContact`, etc.).** All `<Exposed/>` but on the GM-entity, which the
  current Rust server **never instantiates** (`play_character.rs:89-94` always picks
  SGWPlayer 0x02). When the SGWGmPlayer entity is eventually added, GM gating on
  `access_level > 0` must come with it — but that's a CAT-N concern, not CAT-K. Not
  filed here; flagged in CAT-N if not already covered.

- **`processStartMinigameAction` / `processForceMinigameAction` cell methods.** Not
  `<Exposed/>` (lines 410-423 in def). Server-to-server only. Not filed.

- **`<minigame>` property of type PYTHON on MinigamePlayer.** A `PYTHON` type property
  is opaque pickled state — the original BigWorld engine deserialised it server-side and
  pushed dict-shaped state to the client. If the Rust port ever round-trips that property
  through the client, the client could send back arbitrary pickled bytes. Currently
  this property is unused server-side. Not filed today; revisit if/when entity property
  sync starts replicating PYTHON-typed values.
