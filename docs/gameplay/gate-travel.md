---
title: "Gate Travel System"
type: reference
audience: engineers
last_updated: 2026-09-19
---

# Gate Travel System

> **Last updated**: 2026-09-19
> **Status**: Zone transition and ring transport both work. The two stargate animations the 2009 server emitted (6100, 6113) now fire and fan to witnesses; DHD chevrons and squad travel are still missing.

## Overview

Gate travel enables zone transitions via stargates and ring transporters. Stargates provide long-distance travel between worlds, while ring transporters provide local teleportation within or between nearby areas. Both systems involve multi-step sequences with animations, player visibility toggling, and movement locking.

Stargate zone transition is implemented in [`base/world_entry/gate_travel/`](../../crates/services/src/base/world_entry/gate_travel/): on `CellToBaseMsg::GateTravel` the base sends RESET_ENTITIES to tear down the client's view of the old space, persists the destination world and position, and seeds `pending_world_entry` so the client's next ENABLE_ENTITIES drives a fresh create-player + enter-world cycle. Ring transport lives in [`cell/ring_transport/`](../../crates/services/src/cell/ring_transport/) with an 8-state finite state machine.

> **Where the placement is chosen.** Castle CA10 split the dial from the crossing: `onDialGate` arms a 4-second dial and the player then walks into the `REGION_FLAG_Stargate` (bit 2) volume to cross. Both that crossing and the no-gate-volume immediate fallback funnel through one function, `cell::gate_travel::perform_gate_travel`, which holds the single `validate_gate_arrival` call. There is deliberately exactly one — a second call in either caller would validate, and warn, twice per crossing.

## Arrival placement

A `resources.stargates` row's `x_pos` / `y_pos` / `z_pos` / `yaw` is the **stargate prop's own transform** — the `GLB-Stargate_Prefab_Seq` origin lifted out of the cooked map. The 2009 server arrived travellers on exactly that point and never validated it (`deprecated/python/cell/SGWPlayer.py:2129`). On a navmesh-backed world the prefab origin is usually inside the prefab's own footprint carve-out: at Harset it sits ~1.5 units above the floor with the nearest walkable vertex ~5 units away in XZ, well outside the ±3.0 search extents. An arriving player therefore lands off-mesh, every position update they send is suppressed, and witnesses see a frozen avatar while the only log is a `CorrectionSuppressed` with no obvious cause.

`resources.stargates` now carries four nullable columns — `arrival_x`, `arrival_y`, `arrival_z`, `arrival_yaw` — holding an absolute "stand here on arrival" point pinned in-game. All four are set or all four are `NULL`, enforced by the `stargates_arrival_all_or_nothing` CHECK; when `NULL` the arrival falls back to the gate row, `yaw` included. No gate is pinned today. Harset's gate 3 carried a pin placed from map data (PL-A-01) from 2026-09-19 until 2026-09-25, when NPC-AI NA29 dropped it: the NA26 `harset.nav` has the gate dais, so the gate row is standable and travellers arrive on it, the way the 2009 server did (see [the ledger](../analysis/harset-rebuild/placements/A-arrival-and-travel.md#pl-a-01--the-pin-was-dropped-na29)).

[`cell/arrival.rs`](../../crates/services/src/cell/arrival.rs) resolves the final placement. `validate_gate_arrival` is the gate-specific wrapper; `resolve_arrival` is the gate-agnostic core. (Ring transport calls the *validate-only* half, `check_arrival`, and never the respawner substitution — a ring pad is a pad the client is animating at, not a pin on a prop transform. See [ring-transport-system.md](ring-transport-system.md#bounded-aborts-cimmeria-not-2009).) The order is:

1. The authored `arrival_*` pin, or the gate row when there is no pin.
2. If the destination world has a **resident** navmesh, the point must pass both `NavMesh::is_point_valid` and the space AABB derived from the mesh extents. Both layers, because a point that is on-mesh but outside the AABB is hard-rejected by the very next client packet — with the correction budget already cleared by the authorised teleport, which is how a "recovery" turns into a permanent freeze one position over.
3. On failure, the **nearest** authored respawner for that world that is not a placeholder `(0,0,0)` row and that passes the same two checks. Logged at `warn` with `reason = "arrival_off_navmesh"` naming the world and both coordinates, so the operator-actionable seam is "re-pin this gate".
4. If nothing qualifies, there is **no arrival** and the dial is refused. The failure is logged at `error` with `reason = "arrival_unrecoverable"`, and the fix is to seed a respawner for the world or re-pin the gate.

The outcome is reported as an `ArrivalSource`: `Validated`, `Respawner`, `UnrecoverableOffMesh`, or `Unvalidated`. The last one covers destinations with nothing to check against — a world with no `.nav` file, or an **instanced** destination, which has no space until one is created and so never appears in `world_spaces`. Those arrivals are accepted as-is and logged at `debug`.

**`UnrecoverableOffMesh` carries no usable position.** `ResolvedArrival::position` still echoes the *rejected* input so the caller's warn can name it, but `ResolvedArrival::is_usable()` is false and every caller that moves a player gates on it. `perform_gate_travel` refuses and enqueues no `GateTravel` — on the crossing and on the immediate-travel fallback alike: the traveller keeps the position they had, which they can at least walk out of. Handing the rejected point to the transfer is the same silent freeze one layer further along — the traveller is torn out of a world they *could* stand in and re-created off-mesh on one they can't. `gmDHD` reports the refusal to the GM rather than the unconditional "dialing gate address N" it used to print for every outcome.

The helper deliberately does **not** call `NavMesh::get_nearest_point`. That returns its input unchanged on a miss, so its output can never be trusted without re-validating it; and an arrival Detour *can* reproject is an authored pin that is a metre or two wrong and should be corrected at authoring time rather than papered over on every arrival.

The yaw is carried through unchanged even when the position falls back to a respawner — respawner rows have no yaw of their own, and the authored gate facing is the only non-arbitrary answer.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| DHD UI display | DONE | `setupStargateInfo` sends the gate list at world entry; right-clicking a DHD prop (`INT_DHD`, bit 16) emits `onDisplayDHD` (120) — see below |
| Gate arrival validation | DONE | Per-gate `stargates.arrival_*` pin, validated against the destination navmesh with a respawner fallback — see [Arrival placement](#arrival-placement) |
| Walking into the gate | DONE | `REGION_FLAG_Stargate` (bit 2) region routing plus the 4-second dial timer (CA10); the crossing shares the dial's arrival validation |
| Stargate address tracking | DONE | `knownStargateAddresses` property, give/remove |
| Known-address enforcement on dial | DONE | `handle_dial_gate` refuses an address not in `known_stargates` with `onErrorCode` 180 — see [Dial authorization](#dial-authorization) |
| Address unlock on arrival | DONE | A committed arrival learns both worlds' gates in the destination-persistence UPDATE. **New behaviour, not 2009** — see [Address unlock on arrival](#address-unlock-on-arrival) |
| Stargate zone transition | DONE | `base/world_entry/gate_travel/` — RESET_ENTITIES, persist destination, replay world entry |
| Stargate dial timer | DONE | `onDialGate` arms a 4 s timer (`SGWPlayer.beginDialing`) instead of travelling; drained by `cell::gate_travel::gate_dial_tick` on the 100 ms cell tick |
| Stargate walk-through crossing | DONE | Travel fires when the player enters the gate's `REGION_FLAG_Stargate` volume, not on the dial. Worlds with no such region fall back to travelling on the dial |
| Gate-travel contact event | DONE | Fires `ECONTACT_LIST_EVENT_GateTravel` to the traveller's contacts with the destination `world_id` |
| Ring transporter interaction | DONE | Sends the destination list |
| Ring transport FSM | DONE | 8-state machine: IDLE through COOLDOWN |
| Ring player teleportation | DONE | Position-based teleport with visibility toggle |
| Ring Kismet sequences | DONE | `Region_Teleport_Out` / `Region_Teleport_In` |
| Ring movement locking | DONE | `BSF_MovementLock` set/unset during transport |
| Ring cross-world transport | PARTIAL | Same-world works; cross-world path exists but untested |
| Ring multi-player sync | FIXME | Only the first player in the region gets the Matinee — the sequence drives a shared world prop |
| Stargate open animation | DONE | `Stargate_MakeGate` (6100) fires 4 s after a successful dial. `Stargate_DestroyGate` (6103) stays unemitted — the 2009 `cancelDialing` never sent it either (D-CA10) |
| Stargate crossing animation | DONE | `Stargate_CrossGate` (6113) fires on entering the gate volume, before the `GateTravel` teardown |
| DHD chevron lock animations | NOT IMPL | Events 6106–6112 exist in the DB for every gate; never triggered |
| Stargate witness visibility | DONE | Both gate sequences fan to every witness of the dialer plus the dialer, one `onSequence` each. The 2009 server sent to `self.client` only; this is a deliberate addition |
| Squad leader gate travel | NOT IMPL | `processSquadLeaderGateTravel` defined; blocked on the group system |
| Gate address discovery | DONE | Two grant paths: a committed gate arrival, and the content action `grant_stargate_address` (Harset H55), which is the port of 2009's `Act_StargateAddress` node. The content grant is visible without a relog — it sends client method 66 `updateStargateAddress`. `giveStargateAddressStr` / `removeStargateAddressStr` are defined and unimplemented; there is still no revoke path |

## DHD interaction

Right-clicking a prop whose `interaction_type_flags` carry `INT_DHD` (bit 16) opens the dialling UI. [`cell/interactions/dhd.rs`](../../crates/services/src/cell/interactions/dhd.rs) claims the interaction, looks up the stargate belonging to the **player's current world**, and emits `onDisplayDHD` (flat index 120) with a single `UINT8` — the gate's point-of-origin glyph.

Three things are easy to get wrong here:

- **`address_origin` is a glyph (1-38), not an identifier.** It repeats across rows (value 1 on both `SGC W2` and `SGC`, 13 on both Dakara E2 and E3), so it must never be used as a key into the `stargates` map, which is keyed by `stargate_id`. The emit validates against the **authored glyph range**, not just the wire's `UINT8` domain: `0`, `39`-`255` and anything that fails `u8::try_from` all refuse to emit. The column is `INT32` and the wire slot is `UINT8`, so neither type is the domain — a value outside 1-38 serialises perfectly cleanly and reaches the client as a DHD with no symbol to render, which reads as a client bug rather than the seed error it is. Refusing to emit is what makes it findable (`reason = "address_origin_out_of_range"`).
- **The known-address list is not sent here.** It rides `setupStargateInfo` at world entry; the client filters against what it already has.
- **Two gates on one world resolve deterministically** by lowest `stargate_id`, because `stargates` is a `HashMap` and an unordered pick would hand the client a different glyph across restarts.

A DHD prop on a world with no `stargates` row logs `reason = "no_stargate_for_world"` and shows the player nothing. The 2009 server sent a free-text `onError` here; Cimmeria's `onErrorCode` is an enum-coded surface with no free-text arm, so there is nowhere for that string to go. Acceptable while every seeded DHD has a gate.

## Dial authorization

`onDialGate` carries `targetAddressId` as a raw client `INT32`, so the address book is the only thing standing between a crafted packet and a cross-world teleport into unearned content. [`cell/gate_travel/address_book.rs`](../../crates/services/src/cell/gate_travel/address_book.rs) refuses any address not in `CellEntity::known_stargates`, which the base loads from `sgw_player.known_stargates` and hands to the cell on `InitPlayerState`.

The check is the first thing `handle_dial_gate` does after the `-1` cancel sentinel, which matters three times over:

- It runs **before** the 4-second dial is armed, so a refusal cannot leave a gate that opens on a timer.
- It runs **before** the `stargates` cache lookup, so "that address does not exist" and "that address is not yours" are the same observable. A client cannot probe the id space.
- Like 2009's three reject branches, it cancels any dial already in flight (`SGWPlayer.py:2061`). Without that, dialling a gate you hold and then one you do not would leave the first destination armed and crossable.

The refusal reaches the player as `onErrorCode` (121): `SystemID = 0` (`ERRORCODE_SYSTEM_Ability`, the only token the enum defines), `InstanceID = 0`, `ErrorCodeID = 180` (`CONDITION_FEEDBACK_EntityDoesNotHaveStargateAddress`). `InstanceID` is deliberately zero rather than the stargate id — under system 0 the client reads that field as an ability id.

**Transit is not gated.** The check is on the dial and only the dial, matching 2009, which gates `onDialGate` and never `GateTravel.stargatePassed`. A player may walk through a wormhole somebody else opened.

**`gmDHD` is not exempted in the primitive.** An `access_level` branch would put a second authorization surface on a check whose whole value is having exactly one. Instead the GM arm — already authorized against the session's access level — tops the caller's *in-memory* address book up with a `reason = "gm_address_grant"` audit warn before dialling. Nothing is persisted; this mirrors 2009's `giveaddress` console command.

## Address unlock on arrival

A committed gate arrival appends the addresses the trip taught the traveller, in the same `sgw_player` UPDATE that persists the destination world and position ([`base/world_entry/gate_travel/persist_arrival.rs`](../../crates/services/src/base/world_entry/gate_travel/persist_arrival.rs)).

**This is new behaviour, not a restoration.** There is no unlock-on-visit anywhere in the 2009 Python: `SGWPlayer.addStargateAddress` has exactly two callers, the GM console command `giveaddress` and the Atrea authoring node `Act_StargateAddress`. Addresses were authored content. Cimmeria has no content-engine equivalent, so with the dial gate enforced this is the only grant path in the game.

**Both ends of the trip are learned, and the origin is the half that does any work.** On the dial route the destination is a no-op by construction — the dial gate refuses an address you do not already hold, so a dialled destination is always already known. What a traveller does not have is a way back, and `handle_gate_travel` is also the transport for GM `.gotolocation` cross-world, the respawn fork, content `cross_world_teleport` and cross-world rings, none of which consult the address book. The origin world's gates are resolved inside the UPDATE from the row's own pre-update `world_location`, which is the only source that stays correct across consecutive hops.

Two ordering constraints hold this together. The write runs after every mid-transfer abort branch, so nothing is persisted for a transfer that did not happen, and before `query_player_load_data`, which fills the `setupStargateInfo` list the client is about to receive. Get the second wrong and the client renders an address book one hop out of date while the cell enforces the current one.

**A newly created character still starts with an empty book.** `base::character_create` does not name `known_stargates`, so the column defaults to `'{}'`. That predates this work — the dial UI only ever offered known destinations — and Harset H55 decided to leave it that way: in 2009 the first address was always authored content, and content now has a verb that can author it. See *Address grants from content* below.

## Address grants from content

`grant_stargate_address` is the content-engine port of the 2009 Atrea authoring node `Act_StargateAddress` (`entities-editor/editor/Nodes.xml:2428`), which called `SGWPlayer.addStargateAddress`. That node and the GM `giveaddress` console command were its only two callers, so in 2009 stargate addresses were authored content and nothing else. Cimmeria had no equivalent until Harset H55, which is why a character who had never travelled could dial nowhere.

The seed verb takes `target_id` = `resources.stargates.stargate_id`. It is the address itself — not a world id, and not the repeating `address_origin` glyph.

A grant has to reach three places, and all three are emitted from [`cell/content/executor/stargate.rs`](../../crates/services/src/cell/content/executor/stargate.rs):

1. `CellEntity::known_stargates`, which is what the dial gate above enforces against.
2. The client, via `updateStargateAddress` (client method 66: `INT32 addressId`, `UINT8 hasAddress = 1`, `UINT8 hidden = 0`). The client is handed its whole address book exactly once, by `setupStargateInfo` at map load, so without this the grant is invisible until a relog.
3. `sgw_player.known_stargates`, through `CellToBaseMsg::GrantStargateAddress` and an idempotent append in [`base/world_entry/gate_travel/address_grant.rs`](../../crates/services/src/base/world_entry/gate_travel/address_grant.rs) — deliberately the same statement shape as the arrival append beside it, minus the origin-world union.

Legs 1 and 2 go out **before** leg 3 is confirmed. The cell's copy is the thing the dial gate reads, so making the client's copy wait on a database round trip would reopen the divergence the arrival path closes: the server accepting a dial the client's UI does not offer. A lost leg-3 write costs the address at next login and warns; a lost leg-2 send makes a granted address undialable in silence.

The grant is idempotent at both ends. A player who already holds the address gets no write, no client method and no base round trip, and the SQL append is a set difference rather than an `array_append` — `known_stargates` is a bare `integer[]` with no uniqueness constraint, so a duplicate would be silent, permanent, and visible in the player's DHD.

Refusals are never silent: an id with no `stargates` row, a non-player actor, and either failed send each warn with a `reason` field, and the grant is noted in the player journal so a `.bug` bookmark shows when the address was learned.

**Castle mission 708 is the first consumer.** Chain 1357 (the Livewire victory that repairs the DHD) grants `stargate_id = 3`, Harset. Step 4462 — "Use the DHD to dial the Stargate to Harset" — was unreachable before that row existed.

**There is no revoke verb.** 2009's node had a `Remove` port and no shipped content used it; `revoke_stargate_address` can be added when a chain needs one.

## Entity Definition (GateTravel.def)

### Properties

| Property | Type | Flags | Purpose |
|----------|------|-------|---------|
| `knownStargateAddresses` | ARRAY\<PYTHON\> | CELL_PRIVATE | Player's discovered gate addresses |
| `oldWorldID` | INT32 | CELL_PRIVATE | Previous world before travel |
| `gateCounter` | INT32 | CELL_PRIVATE | Gate usage counter |
| `destinationGate` | INT32 | CELL_PRIVATE | Target gate address ID |
| `destinationGateArrivalTime` | FLOAT | CELL_PRIVATE | Expected arrival timestamp |

### Client Methods (Server -> Client)

| Method | Args | Purpose |
|--------|------|---------|
| `setupStargateInfo` | worldStargateList, knownStargateList, hiddenStargateList | Initialize DHD UI |
| `updateStargateAddress` | addressId, hasAddress, hidden | Update single address |
| `stargateRotationOverride` | yaw | Override gate rotation |
| `onStargatePassage` | addressId | Notify successful gate travel |

### Cell Methods (Client -> Server)

| Method | Exposed | Args | Purpose |
|--------|---------|------|---------|
| `onDialGate` | YES | TargetAddressId, SourceAddressId | Player dials a gate |
| `giveStargateAddressStr` | NO | AddressId, Hidden | Grant gate address |
| `removeStargateAddressStr` | NO | AddressId | Remove gate address |
| `closeGatesTo` | NO | AddressId | Close gates to address |
| `processGateTravel` | NO | userData | Execute gate travel |

### Base Methods

| Method | Args | Purpose |
|--------|------|---------|
| `processSquadLeaderGateTravel` | memberId, userData | Squad leader triggers group travel |
| `processGateTravel` | userData | Execute gate travel on base |

## Ring Transporter FSM

The ring transporter uses an 8-state finite state machine:

```
STATE_IDLE
  |-> selectDestination() --> STATE_SEND_WAIT
       |-> regionTriggered() / players present --> STATE_SEND_WARMUP
            |-> __beginTransport(): lock movement, play TeleportOut sequence
            |-> remoteRegion.remoteSend()
            |-> 3.5s timer: hide players (setVisible=false)
            |-> 4.0s timer --> STATE_REMOTE_LOAD_WAIT
                 |-> __doTransport(): teleportTo(destination)
                 |-> remoteTransport()

Remote side:
STATE_IDLE
  |-> remoteWait() --> STATE_RECV_WAIT
       |-> remoteSend() --> STATE_RECV_WARMUP
            |-> __beginTransport()
            |-> remoteTransport() --> STATE_REMOTE_LOAD_WAIT
                 |-> __doTransport()
                 |-> remoteCountUpdate()
                 |-> playerLoaded() x N --> STATE_REMOTE_WARMUP
                      |-> Play TeleportIn sequence
                      |-> 3.0s timer --> STATE_COOLDOWN
                           |-> setVisible(true)
                           |-> 2.5s timer --> STATE_IDLE
                                |-> unsetStateFlag(BSF_MovementLock)
                                |-> onTeleportIn()
```

## Ring Transport Timings

| Phase | Duration | Action |
|-------|----------|--------|
| Warmup (send) | 3.5s | Players hidden |
| Transport (send) | 4.0s | Teleport executed |
| Warmup (receive) | 3.0s | TeleportIn sequence |
| Cooldown | 2.5s | Players visible, movement unlocked |

## Data References

- **Stargate addresses**: 28 in `db/resources/Worlds/Seed/stargates.sql`; the nullable `arrival_x/y/z/yaw` columns and the `stargates_arrival_all_or_nothing` CHECK are declared in `db/resources/Worlds/Tables/stargates.sql`
- **Respawners**: `db/resources/Worlds/Seed/respawners.sql` — the arrival fallback pool. A world with no row has no recovery from an off-mesh gate
- **Ring transporter regions**: `RingTransporterRegion` definitions
- **Kismet events**: `Region_Teleport_Out`, `Region_Teleport_In`

## RE Priorities

1. **Stargate travel** - Implement `processGateTravel` for zone transitions
2. **Gate animation** - Stargate dialing/kawoosh sequence from client
3. **Squad gate travel** - `processSquadLeaderGateTravel` group teleport protocol
4. **Hidden addresses** - How hidden gate addresses work in the DHD UI
5. **Cross-world rings** - Verify ring transport across world boundaries

## Related Docs

- [combat-system.md](combat-system.md) - Movement lock during transport
- [group-system.md](group-system.md) - Squad leader gate travel
- [ring-transport-system.md](ring-transport-system.md) - The ring FSM in full, including the bounded aborts and the shared arrival validation
- [../engine/space-management.md](../engine/space-management.md) - Space extents, navmesh loading, and which worlds are instanced
