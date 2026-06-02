# CAT-B — Movement / Teleport / Position

## Trust posture summary

The movement subsystem trusts the client to be the authoritative source of
position, velocity, direction, and "I'm standing on object X" claims. Inbound
`AVATAR_UPDATE_EXPLICIT` (0x03) writes the client-supplied position vector
straight into the cell's authoritative `CellEntity::position` with no speed,
distance, navmesh, vertical-bounds, or rate validation — every per-tick
position update is a free teleport for an attacker. A `is_position_valid`
navmesh helper exists but is never called from the write path. Several
client-hinted region/destination handlers (`onDialGate`,
`setRingTransporterDestination`, `triggerClientHintedGenericRegion`) accept
target IDs without verifying the player is physically near the source pad /
region, turning them into one-shot teleports that bypass intermediate
geography. Player-initiated `setMovementType` is correctly NPC-only-guarded
(no exploit), `Unstuck` (cell method 71) is a logged stub today, and the
GM-prefixed `gmGoto*` / `gmSummon` Cheats methods have no server handler
(fall through to `warn!`). The base layer correctly substitutes the
session's `player_entity_id` for the client-supplied entityId prefix on
cell-method calls (line 122/134 of `cell_arms.rs`), so cross-entity
spoofing through that channel is blocked. The dominant exploit shape is
"send a single 0x03 packet with arbitrary world-space coordinates and the
server obeys" — every higher-level system that depends on player position
(AoI, region triggers, navmesh distance, gating quests) is downstream of
this primitive and is corrupted in turn.

---

### CAT-B-01 — `AVATAR_UPDATE_EXPLICIT` writes client position with no validation

**Severity**: Critical
**Class**: Speed hack / teleport / position spoofing
**Wire surface**: System message `0x03` (`AVATAR_UPDATE_EXPLICIT`)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
The client sends a 40-byte payload containing `[spaceId:u32][vehicleId:u32]
[pos:3×f32][vel:3×f32][dir:3×i8][flags:u8][cells:3×u8][updateId:u8]`. The
server reads pos/vel/dir from offsets 8–34, looks up the player's
`entity_id` from session state (good), and forwards the values to the cell
via `BaseToCellMsg::EntityMove`. The cell handler at
`crates/services/src/cell/service/base_messages/mod.rs:138-167` calls
`space_mgr.update_entity_position(entity_id, position, direction, velocity)`
which unconditionally writes the new position and updates the spatial grid
(`crates/services/src/cell/space_manager/entities.rs:147-177`). There is no
delta cap, no `last_position → new_position` distance check, no navmesh
reachability check, no Z-axis cap, no rate limit, no comparison to the
last server-confirmed position. A modified client can send a single 0x03
packet with `pos = (0, 0, 0)` or any other coordinate in any world and the
server's authoritative spatial state will reflect it on the next tick.
AoI, region triggers, gating quests, threat radius — every downstream
system reads from this state.

**Evidence**
- Ghidra: `019d08b8` — string literal `avatarUpdateExplicit` confirms the
  client-side message name; the 0x03 system-message payload shape is
  pinned by the spec-cited `messages.cpp` table.
- Client behavioral log: n/a (continuous 10 Hz traffic — the canonical
  movement signal).
- Cross-ref to Rust handler (for the fix author):
  `crates/services/src/base/connect_loop/encrypted/mod.rs:211-281` (parse +
  forward), `crates/services/src/cell/service/base_messages/mod.rs:138-167`
  (apply), `crates/services/src/cell/space_manager/entities.rs:147-177`
  (write).

**Attack scenario**
1. Stand at any reachable position to establish a baseline.
2. Send one 0x03 packet with `pos = (any_x, any_y, any_z)` for the
   destination world's coordinate range.
3. Server overwrites the entity position. Next AoI tick fires
   `EnteredAoI`/`LeftAoI` from the new location — the attacker now sees
   and interacts with entities at the spoofed position (loot, NPCs,
   regions). Quest regions can be flipped from anywhere on the map.

**Suggested remediation (one line)**
Reject `EntityMove` when `||new_pos − last_server_pos|| / (server_tick_dt)`
exceeds the entity's class-derived max speed, AND when
`!is_position_valid(new_pos)`; pre-bake a `last_server_pos` snapshot
keyed off the *server's* tick clock — never client-supplied timestamps.
Consult `movement-physics-advisor` for the canonical speed-validation
primitive shape.

**Would benefit from x64dbg trace?**
No — the trust violation is fully demonstrable from the Rust handler alone;
the 0x03 framing is already pinned by the wire-format constants in
`encrypted/mod.rs:485-503`.

---

### CAT-B-02 — `onDialGate` ignores `source_address_id`, teleports from anywhere

**Severity**: High
**Class**: Authoritative location bypass / world teleport
**Wire surface**: Cell method 35 (`onDialGate`) — `Event_NetOut_OnDialGate`
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
The handler at `crates/services/src/cell/gate_travel.rs:35-108` takes a
client-supplied `(target_address_id, source_address_id)` pair, validates
that `target_address_id` is a known stargate, then issues
`CellToBaseMsg::GateTravel` to the destination. `source_address_id` is
explicitly unused (prefixed `_source_address_id`, line 38). There's no
check that the player is physically near any stargate, let alone the one
they claim to be using. A player can stand anywhere in the world and dial
any cross-world stargate — bypassing the in-world walk to the source
gate, any mission gating tied to "reach gate X first" content actions,
and any region-trigger entry conditions on the source pad. Combined with
[[CAT-B-01]], a player can send a single 0x03 to spawn-bump themselves
into a world they shouldn't have access to without ever having to walk
to a gate or have completed the unlock quest.

**Evidence**
- Ghidra: client emits `onDialGate` via the standard `Event_NetOut_*` path
  (no Cheats prefix); the cell method index 35 is the documented GateTravel
  interface (see `crates/services/src/cell/cell_methods/gate_travel.rs:7`).
- Client behavioral log: n/a (player-driven UI event).
- Cross-ref to Rust handler:
  `crates/services/src/cell/cell_methods/gate_travel.rs:9-40` (dispatch),
  `crates/services/src/cell/gate_travel.rs:35-108` (handle).

**Attack scenario**
1. Player creates a low-level character on the starter world.
2. Without walking to any stargate, send `onDialGate(target=2, source=0)`
   where target=2 is a stargate to an end-game world that requires
   completion of multiple unlock missions.
3. Server validates the target exists in `space_mgr.stargates`, calls
   `handle_gate_travel`, persists the destination world to `sgw_player`,
   and ships RESET_ENTITIES + new world entry. Attacker is now on the
   end-game world with no mission gating.

**Suggested remediation (one line)**
Validate that the player's current position is within an interaction
radius (e.g., 5m) of the source stargate's pad position before dispatching
`GateTravel`; fail closed if the source-gate position is unknown, and
log-and-reject if the player isn't standing on it. Route the redesign
back to `movement-teleport-advisor` to confirm whether a region-trigger
ack should be the gating signal instead.

**Would benefit from x64dbg trace?**
No — the unused `_source_address_id` parameter is a code-level proof.

---

### CAT-B-03 — `setRingTransporterDestination` accepts any source ring globally

**Severity**: High
**Class**: Authoritative location bypass / ring teleport
**Wire surface**: Cell method 91 (`setRingTransporterDestination`) — `Event_NetOut_SetRingTransporterDestination`
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
`handle_select_destination` at
`crates/services/src/cell/ring_transport/runtime.rs:244-350` accepts
`(source_region_id, destination_region_id)` from the client and validates:
(a) `source` exists and is `Idle`, (b) `destination` is in
`source.destination_ids`, (c) `destination` is not the source itself.
It does **not** validate that the player is physically standing on the
source ring's pad — there's no entity-position check against the source
region's `RingRegion::x/y/z` or its `point_set_id` containment. The
related `triggerClientHintedGenericRegion` (see [[CAT-B-04]]) is what
normally sets `player.ring_source_id` when the player walks onto the pad,
but `handle_select_destination` doesn't consult `ring_source_id` either —
it overwrites it on line 332. A player can send
`setRingTransporterDestination(source=any_idle_ring, dest=any_valid_dest)`
from anywhere in the world and the FSM will run, locking the source
ring's state into `SendWait`, and (when auto-start fires) eventually
running the full transport ceremony.

**Evidence**
- Ghidra: `019b3ec8` cluster confirms the `Event_NetOut_SetRingTransporterDestination`
  vtable (`00ae9ed0`, `00ae9ff0`) and `SGWNetworkManager::EventHandler<...>`
  (`00d68c30`) — emit path is the standard cell-method dispatch.
- Client behavioral log: n/a (UI-driven destination selector).
- Cross-ref to Rust handler:
  `crates/services/src/cell/cell_methods/player/world/mod.rs:207-228`
  (dispatch arm), `crates/services/src/cell/ring_transport/runtime.rs:244-350`
  (handle), `crates/services/src/cell/ring_transport/transporter/mod.rs:245-256`
  (`validate_destination` — note: doesn't check player position).

**Attack scenario**
1. Discover ring-region IDs via `onRingTransporterList` (server broadcasts
   the list when any player interacts with a ring, which AoI replays).
2. Without standing on any ring pad, send
   `setRingTransporterDestination(source=Castle_pad, dest=Asuras_pad)`.
3. Server runs `validate_destination` — passes because Castle is Idle and
   Asuras is a valid destination. Source enters `SendWait`, destination
   `RecvWait`. The auto-start kicks in via
   `should_auto_start`, and the player gets teleported to Asuras without
   walking to the Castle ring pad. Combined with the mission-gate check
   that's *also* anchored only to the source ring's
   `required_mission_id` (which the player may or may not have completed),
   this is a cross-world teleport from arbitrary coordinates.

**Suggested remediation (one line)**
Before `enter_send_wait`, verify `player.ring_source_id ==
Some(source_region_id)` — i.e., the player's region-trigger state shows
they actually walked onto the pad — and reject otherwise; treat any other
state as a protocol-level error. Consult `movement-teleport-advisor` on
whether to additionally distance-check against `RingRegion::x/y/z`.

**Would benefit from x64dbg trace?**
No.

---

### CAT-B-04 — `triggerClientHintedGenericRegion` discards client position, trusts region_id

**Severity**: High
**Class**: Region-trigger spoofing / chain-event injection
**Wire surface**: Cell method 85 (`triggerClientHintedGenericRegion`) — `Event_NetOut_TriggerClientHintedGenericRegion`
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
The handler at
`crates/services/src/cell/cell_methods/player/world/mod.rs:128-191`
reads a client-supplied `(region_id, b_entering, x, y, z)` payload. The
position triple `(x, y, z)` is parsed and immediately discarded
(prefixed `_x`, `_y`, `_z` — lines 132-134). The handler looks up the
region by id, fires the `enter_region` / `exit_region` content chain
event for the region's `tag`, and forwards to the ring transport FSM
if the region matches a ring pad. There is no validation that the
player's *actual* position (from the server-tracked
`CellEntity::position`) is inside the region's bounds. A malicious
client can send `triggerClientHintedGenericRegion(region=N,
b_entering=true, 0, 0, 0)` from anywhere to claim entry into any
loaded region — firing every content chain bound to
`enter_region::<tag>` for arbitrary regions (quest-region triggers,
mission-advance triggers, content-engine actions). The ring forwarding
also fires `RingTransporter::region_triggered(entering=true,
entity_id)` which is what sets `player.ring_source_id` — meaning this
single handler is sufficient to *also* prepare the ring-transporter
exploit described in [[CAT-B-03]] without a position check.

**Evidence**
- Ghidra: client-emit path identical to other cell methods (standard
  `Event_NetOut_*` shape); spec source is
  `entities/defs/interfaces/SGWPlayer.def`.
- Client behavioral log: n/a (continuous as the player walks).
- Cross-ref to Rust handler:
  `crates/services/src/cell/cell_methods/player/world/mod.rs:128-191`,
  `crates/services/src/cell/ring_transport/runtime.rs:375-409`
  (`handle_region_trigger` — region_triggered call).

**Attack scenario**
1. Enumerate loaded region IDs by entering the world normally and
   observing `triggerClientHintedGenericRegion` round-trips, OR
   bruteforce u32 region_ids until log-grep shows `region_tag` was
   resolved.
2. Send `triggerClientHintedGenericRegion(region=<mission_region_id>,
   b_entering=true, 0, 0, 0)` from a safe far-away spot.
3. Server fires `fire_enter_region` and the chain engine advances the
   mission as if the player walked into the trigger area. Repeat for
   chained quest steps to skip-flag content; combine with the ring
   exploit path to short-circuit cross-world travel.

**Suggested remediation (one line)**
Before firing the chain event, validate the player's
`space_mgr.get_entity(entity_id).position` actually intersects the
region's geometry (bounding-box for simple regions; point-in-polygon for
named-region shapes), and drop the client's `(x,y,z)` entirely — the
server already has the canonical position.

**Would benefit from x64dbg trace?**
No — the discarded `_x`/`_y`/`_z` is a code-level proof.

---

### CAT-B-05 — `EntityMove` has no anti-replay / dedup of position updates

**Severity**: Medium
**Class**: Replay attack / position rewind
**Wire surface**: System message `0x03` (`AVATAR_UPDATE_EXPLICIT`)
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
The 0x03 message is parsed and dispatched on the **unreliable** path
(`FLAG_RELIABLE` not set on send — see comment at
`crates/services/src/mercury/aoi/update.rs:14-18`). Because the message
is unreliable, the Mercury channel's `rx_window` dedup (which protects
reliable traffic at `crates/mercury/src/channel/mod.rs:442-456`) does
not apply. The payload carries an `updateId:u8` at the tail (offset 39
of the 40-byte payload — see comment at line 212 of `encrypted/mod.rs`)
that the server reads as part of the constant-length consumption but
never validates against any per-client sequence counter. A captured
position packet can be replayed at the server later — when the player
has moved elsewhere — to rewind their authoritative position to the
captured value. Combined with [[CAT-B-01]] (no movement validation),
replay → rewind is a one-packet primitive.

**Evidence**
- Ghidra: `019d08b8` `avatarUpdateExplicit` — emit path is one packet per
  position update; the SGW client doesn't sign or chain them.
- Client behavioral log: n/a.
- Cross-ref to Rust handler:
  `crates/services/src/base/connect_loop/encrypted/mod.rs:211-281`
  (no `updateId` validation; field at `payload[39]` is read by the
  constant-length consumer but never inspected).

**Attack scenario**
1. Attacker (or a passive observer in possession of pre-encryption
   packets) captures a 0x03 packet for player P at position A.
2. Player P walks across the world to position B.
3. Attacker (sharing a session key, or testing from the same client)
   re-injects the captured 0x03. Server writes position A back over
   position B, jerking the avatar back and re-firing AoI/region
   triggers as if the rewind were legitimate.

**Suggested remediation (one line)**
Track the last accepted `updateId` per session in
`ConnectedClientState`; reject 0x03 when `updateId <= last_seen`
(wrapping rules per the client's update-counter spec), so replays are
silently dropped.

**Would benefit from x64dbg trace?**
Yes — confirming the `updateId` field semantics (monotonic? wrapping?
reset on world transition?) in the running client would tighten the
remediation; the comment at `encrypted/mod.rs:212` is the only current
documentation.

---

### CAT-B-06 — Client spaceId in 0x03 ignored — no cross-space sanity check

**Severity**: Medium
**Class**: Position smuggling / space-grid corruption
**Wire surface**: System message `0x03` (`AVATAR_UPDATE_EXPLICIT`)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
The 0x03 payload's first 4 bytes are `spaceId` (per the comment block at
`crates/services/src/base/connect_loop/encrypted/mod.rs:212`). The
server reads `payload[0..4]` and discards it ("not used here -- client
confirms which space"). The position written to the cell is interpreted
in whatever space the entity is *currently* assigned to on the server
side. If the server's notion of the player's space ever diverges from
the client's (e.g., after a gate-travel race where the cell's
`CreateEntity` reply arrives late, or after instance reset), the client
might be sending coordinates valid for one world while the server
writes them into another space's spatial grid — corrupting the grid
with out-of-range coordinates and breaking AoI for every player in
that space. There's no check that the client-supplied `spaceId`
matches `space_mgr.entity_space.get(&entity_id)`.

**Evidence**
- Ghidra: client emits `spaceId` as the first u32 — see Ghidra wire
  shape in `encrypted/mod.rs:211-213`.
- Client behavioral log: n/a.
- Cross-ref to Rust handler:
  `crates/services/src/base/connect_loop/encrypted/mod.rs:224`
  ("`payload[0..4] = spaceId (not used here -- client confirms which
  space)`").

**Attack scenario**
1. Trigger a gate-travel; race the `BaseToCellMsg::CreateEntity` reply.
2. Send a flood of 0x03 packets with the old world's `spaceId` while the
   cell's `entity_space` map still points to the old space. The first
   packets get written to the old space (server reads
   `entity_space[entity_id]`, which is still the old value); subsequent
   ones — once the new space binding lands — get written to the new
   space. Coordinates valid for one world land in the other's grid.
3. Spatial grid degrades; AoI distance checks return nonsense; nearby
   players may see ghosts or miss real entities.

**Suggested remediation (one line)**
Read the spaceId from the payload and drop the message unless it equals
`space_mgr.entity_space.get(&entity_id)`; log a `warn!` on mismatch so
race conditions are observable.

**Would benefit from x64dbg trace?**
No.

---

### CAT-B-07 — Cell-method `entityId` prefix is read then ignored — defense-in-depth gap

**Severity**: Low
**Class**: Defense-in-depth / wire-shape hardening
**Wire surface**: Every cell-method call (0x80–0xBF range)
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
The cell-method dispatcher at
`crates/services/src/base/connect_loop/cell_arms.rs:86-89` reads a
4-byte `entity_id_from_client` from the wire (always present per the
comment block), but then **substitutes** `player_eid` from the session
state when forwarding to the cell (lines 122 and 134). This is the
correct behavior for security — but the value parsed off the wire is
never compared to `player_eid`, so a client sending an `entityId`
prefix that doesn't match its session's player is silently accepted.
Today this is harmless (the substitution catches it) but the lack of
the cheap check means a malicious-but-curious client can experiment
with the wire format without ever surfacing the protocol violation in
the server logs. A future refactor that, for any reason, started
trusting `entity_id_from_client` (e.g., to support GM-spoofed entity
calls) would silently re-open this. The two-line guard (warn + drop
on mismatch) is a regression-resistant pin for the substitution
invariant.

**Evidence**
- Cross-ref to Rust handler:
  `crates/services/src/base/connect_loop/cell_arms.rs:86-89` (parse) +
  `:122, :134` (substitute).

**Attack scenario**
1. Send `cell method N` with `entityId = 0xDEADBEEF` (a foreign player's
   id). Server silently accepts the message and dispatches the method
   against the attacker's own `player_eid` — no protocol-level error
   surfaces in logs.
2. Detection: attacker discovers via empirical wire-shape probing that
   the prefix is unverified, which is the first step in finding the
   "what if a future refactor uses this?" gap.

**Suggested remediation (one line)**
Compare `entity_id_from_client` to `player_eid` and `warn!`-log + drop on
mismatch.

**Would benefit from x64dbg trace?**
No.

---

### CAT-B-08 — `Unstuck` cell method is a no-op stub — UX gap, not security today

**Severity**: Low
**Class**: Future-implementation hazard
**Wire surface**: Cell method 71 (`unstuck`) — `Event_NetOut_Unstuck`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
At `crates/services/src/cell/cell_methods/player/combat/mod.rs:114-117`,
the `UNSTUCK` arm logs `UNIMPLEMENTED: unstuck` and returns. No state
is mutated; no exploit exists today. Flagging because: (a) Unstuck is
the canonical example of "server must compute the destination from the
navmesh" — when this is implemented, the implementer must NOT take the
target position from the client; (b) when implemented, Unstuck must
have a server-side cooldown (otherwise it's just another teleport
primitive at zero cost) and must reject during combat (no escape from
threat). Filing as a pre-emptive pin so the future PR catches review.

**Evidence**
- Ghidra: `019b4374` `Event_NetOut_Unstuck` confirms the client
  emit path; the handler stub is at the cited Rust line.
- Cross-ref to Rust handler:
  `crates/services/src/cell/cell_methods/player/combat/mod.rs:114-117`.

**Attack scenario**
(N/A today — handler is a no-op.) Future risk: if implemented without
server-computed destination + cooldown + combat-state gate, every player
gets a 0-cooldown teleport-to-safe-position primitive.

**Suggested remediation (one line)**
When implementing, anchor the destination to a navmesh sample around the
last server-confirmed safe position (NOT a client-supplied target), add
a server-side cooldown of ≥30 s, and reject when `BSF_IN_COMBAT` is set;
route the design to `movement-teleport-advisor`.

**Would benefit from x64dbg trace?**
No.

---

### CAT-B-09 — Navmesh `is_position_valid` exists but is unused on the inbound write path

**Severity**: Medium
**Class**: Defense-in-depth / navmesh containment
**Wire surface**: System message `0x03` (`AVATAR_UPDATE_EXPLICIT`)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
`SpaceManager::is_position_valid` at
`crates/services/src/cell/space_manager/spatial.rs:53-67` returns
`true` when the position lies on the walkable navmesh of the entity's
space. The function is defined; it loads navmeshes at space-startup
(`crates/services/src/cell/space_manager/lifecycle.rs:26-47`); and it
returns `true` (fail-open) when no navmesh is loaded. But it is
**never called** from `update_entity_position` (the inbound write
path), so a client can send positions deep underwater, inside walls,
clipping into NPC spawn rooms, above ceilings, or under terrain, and
the server accepts them as authoritative. This is a sibling to
[[CAT-B-01]] but distinct because the *fix* is local: the validator
already exists. Wiring it into the inbound path is one conditional
plus a rejection (or snap-to-nearest, depending on policy).

**Evidence**
- Cross-ref to Rust handler:
  `crates/services/src/cell/space_manager/spatial.rs:53-67`
  (`is_position_valid` defined);
  `crates/services/src/cell/space_manager/entities.rs:147-177`
  (`update_entity_position` — no call to `is_position_valid`).

**Attack scenario**
1. Send 0x03 with `pos = (x, y_underground_terrain, z)` for a known
   underground vault containing high-value loot spawns.
2. Server writes the position. AoI tick fires `EnteredAoI` for loot
   NPCs the player wouldn't normally see; quest regions inside the
   bound geometry fire from the spoofed position.
3. Repeat for any out-of-bounds region of interest.

**Suggested remediation (one line)**
In `update_entity_position`, call `is_position_valid(entity_id,
&new_pos)`; on `false`, reject the move (or snap to the last
confirmed valid position) and `warn!`-log; route the snap-vs-reject
policy choice through `movement-physics-advisor`.

**Would benefit from x64dbg trace?**
No.

---

### CAT-B-10 — 0x02 / 0x04 / 0x05 avatar variants are silently dropped — protocol gap

**Severity**: Low
**Class**: Protocol completeness / wire-format hardening
**Wire surface**: System messages `0x02` (`AVATAR_UPD_IMPLICIT`), `0x04`
(`AVATAR_UPDW_IMPLICIT`), `0x05` (`AVATAR_UPDW_EXPLICIT`)
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
At `crates/services/src/base/connect_loop/encrypted/mod.rs:484-491`, the
constant-length payload widths for 0x02 (36 bytes), 0x04 (36), and 0x05
(40) are honored so the bundle scanner advances correctly, BUT the
dispatcher (lines 193-447) has no match arm for those ids. They fall
through to `_ => tracing::trace!(... "Unhandled client message")` at
line 444. A modern SGW client emits 0x02 (implicit) updates routinely
between 0x03 (explicit) batches per the BigWorld implicit/explicit
split. The server therefore sees only a subset of the position
traffic, and a hostile client could deliberately emit only 0x02 to
keep its position appearing stationary to the server while moving
freely on the client's view — combined with [[CAT-B-01]] this isn't
itself a teleport primitive (the server's position view would just
stale), but it confounds any future server-side movement-validation
heuristic that relies on the per-tick delta being well-defined.

**Evidence**
- Cross-ref to Rust handler:
  `crates/services/src/base/connect_loop/encrypted/mod.rs:484-491`
  (parse), `:444` (silent fall-through).

**Attack scenario**
Combined with future speed-hack detection (when added), an attacker can
*delay* tripping the detector by sending 0x02 (which the server
silently drops, leaving its position view static) interleaved with
infrequent 0x03 small-delta packets. Without 0x02 being processed,
the server can't reconstruct a continuous track to compare against.

**Suggested remediation (one line)**
Decide explicitly whether 0x02/0x04/0x05 are no-ops (then promote the
`trace!` to a `debug!` with the parsed position so they're loggable) or
proper position updates (then dispatch to `EntityMove` like 0x03).
Document the policy in `docs/protocol/`.

**Would benefit from x64dbg trace?**
Yes — confirming the client's actual 0x02/0x04/0x05 emit cadence in a
running session would tighten this triage (the implicit/explicit split
semantics matter for the right answer).

---

## Not Filed

- **GotoXYZ / Goto / GotoLocation / Summon (`gm`-prefixed Cheats cell
  methods)**: Client-side strings exist (`019c3658` `gmGoto`,
  `019c366c` `gmGotoLocation`, `019c3688` `gmGotoXYZ`, `019c3710`
  `gmSummon`); the SGW.exe emits them via `Event_NetOut_*` from
  Cheat-bound slash commands. The Rust server has **no handler** for
  these method indices — they fall through to the `Unhandled cell
  method` warn-arm at `crates/services/src/cell/dispatch/router.rs:101-106`.
  Not exploitable today. CAT-N (GM commands) owns the broader
  GM-gating concern; flagging here as a sibling-only note. If/when
  these are implemented, this is the canonical example of a method
  that MUST be gated on `client_state.access_level == GM`.

- **`SetMovementType` cell method (index 1, SGWBeing)**: Routed at
  `crates/services/src/cell/cell_methods/being.rs:65-104`, but the
  downstream `broadcast_movement_type` at
  `crates/services/src/cell/abilities/messaging.rs:209-222` early-
  returns with a `warn!` when the entity is a player (which is the
  case for any client-driven call after `cell_arms` substitution).
  Effectively NPC-only; not exploitable today. A future change that
  drops the `is_player` guard would re-open this.

- **`SetCrouched` cell method (index 5, SGWCombatant)**: Sets/clears
  the `BSF_CROUCHING` state-field bit at
  `crates/services/src/cell/cell_methods/combatant.rs:31-58`. No
  server-side accuracy buff, no cooldown-relevant behavior, no AoI
  fan-out fires; pure cosmetic state announcement. Confirmed against
  the codebase that no ability/effect modifier reads `BSF_CROUCHING`.

- **`ChangeWeaponState` (`Event_NetOut_ChangeWeaponState`)**: Comment
  at `crates/services/src/cell/cell_methods/player/world/mod.rs:520-525`
  marks this as "dead scaffolding (the 2009 client never shipped one — the
  archaeology agent confirmed `Event_NetOut_ChangeWeaponState` is dead
  scaffolding)". No emit path in SGW.exe per Ghidra (only
  `Event_NetOut_ChangeWeaponState` RTTI and the
  `register_NetOut_ChangeWeaponState` stub — no caller). Cannot fire.

- **`Physics` (`Event_NetOut_Physics`)**: Ghidra confirms the
  `Event_NetOut_Physics` class exists with handler at `00d686f0` (a
  vfunc-0 destructor); no `Event_NetOut_Physics::vfunc_2` emit-into-
  bundle function is callable (no slash-command or UI emit). Likely
  dev-only / never wired. The Rust server has no handler. Not
  exploitable today.

- **`onPlayerTeleport` (method 116)**: Server → client direction only
  (`crates/services/src/cell/client_methods/player::ON_PLAYER_TELEPORT`).
  Client-side handler is the streaming-load waiting flag — does not
  move the avatar (the comment at
  `crates/services/src/mercury/aoi/update.rs:57-59` is explicit).
  No inbound from client; nothing to validate. The "known footgun" in
  the brief — "streaming load hint, not authoritative" — is
  already correctly modeled in code: `FORCED_POSITION` (0x31) is the
  authoritative snap, paired with onPlayerTeleport as the hint. Server
  composes both in `build_teleport_bundle`
  (`crates/services/src/base/world_entry/teleport.rs:151-176`).

- **AoI `aoi_radius` widening / cheat**: A client cannot change its own
  `aoi_radius` (server uses the `CellEntity` default of 100.0; clients
  send no `aoi_radius` field). AoI is server-driven from the entity's
  position alone. This is correctly server-authoritative. Filing
  [[CAT-B-01]] (the position primitive) already covers any radius-
  amplification consequences.

- **Cell-boundary handoff (central question 7)**: Cimmeria is
  single-cell today (one cell process per space, see
  `crates/services/src/cell/`); there's no cell-to-cell handoff to
  exploit. When sharding is introduced later, the right place to
  audit is the handoff message in `crates/services/src/cell/messages/`
  — not in scope for today's audit.

- **`WorldInstanceReset` (cell method 92)**: `UNIMPLEMENTED` stub at
  `crates/services/src/cell/cell_methods/player/world/mod.rs:230-233`.
  Belongs to CAT-O (World / Space / Gate). Future-implementation
  hazard, but not actionable here.

- **Velocity-vector spoofing in 0x03**: The velocity triple at
  payload[20..32] is stored on `CellEntity.velocity` at
  `crates/services/src/cell/space_manager/entities.rs:174` and
  re-broadcast verbatim to AoI witnesses via `build_avatar_update`
  (`crates/services/src/mercury/aoi/update.rs:21-50`). A client can
  set arbitrary velocity for visual effect on witnesses, but the
  server never *uses* the velocity for any authoritative computation
  — distance checks use position only. Pure visual; not security-
  relevant. Promote to CAT-B-NN if a future feature reads velocity
  for cooldown/damage modulation.

- **Direction (yaw/pitch/roll) spoofing in 0x03**: Same as velocity —
  stored, re-broadcast to witnesses, never used authoritatively. Not
  filed.
