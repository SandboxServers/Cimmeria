---
title: "Gate Travel System"
type: reference
audience: engineers
last_updated: 2026-09-18
---

# Gate Travel System

> **Last updated**: 2026-09-18
> **Status**: Zone transition and ring transport both work. Every gate *animation* is missing.

## Overview

Gate travel enables zone transitions via stargates and ring transporters. Stargates provide long-distance travel between worlds, while ring transporters provide local teleportation within or between nearby areas. Both systems involve multi-step sequences with animations, player visibility toggling, and movement locking.

Stargate zone transition is implemented in [`base/world_entry/gate_travel/`](../../crates/services/src/base/world_entry/gate_travel/): on `CellToBaseMsg::GateTravel` the base sends RESET_ENTITIES to tear down the client's view of the old space, persists the destination world and position, and seeds `pending_world_entry` so the client's next ENABLE_ENTITIES drives a fresh create-player + enter-world cycle. Ring transport lives in [`cell/ring_transport/`](../../crates/services/src/cell/ring_transport/) with an 8-state finite state machine.

> **What is *not* here yet.** Walking into the event horizon — the `REGION_FLAG_Stargate` (bit 2) region routing — and the pending-dial state machine that owns the 4-second dial timer and the `Stargate_MakeGate` / `Stargate_CrossGate` emits belong to the **Castle CA10 packet** and are not on this branch. Today the only way to travel is the explicit `onDialGate` RPC from the DHD UI. When CA10 lands it must carry the one-line `validate_gate_arrival` call in [`cell/gate_travel.rs:95`](../../crates/services/src/cell/gate_travel.rs#L95) with it, or the walk-through arrival loses the validation the dial arrival has.

## Arrival placement

A `resources.stargates` row's `x_pos` / `y_pos` / `z_pos` / `yaw` is the **stargate prop's own transform** — the `GLB-Stargate_Prefab_Seq` origin lifted out of the cooked map. The 2009 server arrived travellers on exactly that point and never validated it (`deprecated/python/cell/SGWPlayer.py:2129`). On a navmesh-backed world the prefab origin is usually inside the prefab's own footprint carve-out: at Harset it sits ~1.5 units above the floor with the nearest walkable vertex ~5 units away in XZ, well outside the ±3.0 search extents. An arriving player therefore lands off-mesh, every position update they send is suppressed, and witnesses see a frozen avatar while the only log is a `CorrectionSuppressed` with no obvious cause.

`resources.stargates` now carries four nullable columns — `arrival_x`, `arrival_y`, `arrival_z`, `arrival_yaw` — holding an absolute "stand here on arrival" point pinned in-game. All four are set or all four are `NULL`, enforced by the `stargates_arrival_all_or_nothing` CHECK; when `NULL` the arrival falls back to the gate row, `yaw` included. No gate is pinned yet: Harset's gate-3 value waits on the in-client placement session.

[`cell/arrival.rs`](../../crates/services/src/cell/arrival.rs) resolves the final placement. `validate_gate_arrival` is the gate-specific wrapper; `resolve_arrival` is the gate-agnostic core. (Ring transport calls the *validate-only* half, `check_arrival`, and never the respawner substitution — a ring pad is a pad the client is animating at, not a pin on a prop transform. See [ring-transport-system.md](ring-transport-system.md#bounded-aborts-cimmeria-not-2009).) The order is:

1. The authored `arrival_*` pin, or the gate row when there is no pin.
2. If the destination world has a **resident** navmesh, the point must pass both `NavMesh::is_point_valid` and the space AABB derived from the mesh extents. Both layers, because a point that is on-mesh but outside the AABB is hard-rejected by the very next client packet — with the correction budget already cleared by the authorised teleport, which is how a "recovery" turns into a permanent freeze one position over.
3. On failure, the **nearest** authored respawner for that world that is not a placeholder `(0,0,0)` row and that passes the same two checks. Logged at `warn` with `reason = "arrival_off_navmesh"` naming the world and both coordinates, so the operator-actionable seam is "re-pin this gate".
4. If nothing qualifies, there is **no arrival** and the dial is refused. The failure is logged at `error` with `reason = "arrival_unrecoverable"`, and the fix is to seed a respawner for the world or re-pin the gate.

The outcome is reported as an `ArrivalSource`: `Validated`, `Respawner`, `UnrecoverableOffMesh`, or `Unvalidated`. The last one covers destinations with nothing to check against — a world with no `.nav` file, or an **instanced** destination, which has no space until one is created and so never appears in `world_spaces`. Those arrivals are accepted as-is and logged at `debug`.

**`UnrecoverableOffMesh` carries no usable position.** `ResolvedArrival::position` still echoes the *rejected* input so the caller's warn can name it, but `ResolvedArrival::is_usable()` is false and every caller that moves a player gates on it. `handle_dial_gate` returns `false` and enqueues no `GateTravel`: the traveller keeps the position they had, which they can at least walk out of. Handing the rejected point to the transfer is the same silent freeze one layer further along — the traveller is torn out of a world they *could* stand in and re-created off-mesh on one they can't. `gmDHD` reports the refusal to the GM rather than the unconditional "dialing gate address N" it used to print for every outcome.

The helper deliberately does **not** call `NavMesh::get_nearest_point`. That returns its input unchanged on a miss, so its output can never be trusted without re-validating it; and an arrival Detour *can* reproject is an authored pin that is a metre or two wrong and should be corrected at authoring time rather than papered over on every arrival.

The yaw is carried through unchanged even when the position falls back to a respawner — respawner rows have no yaw of their own, and the authored gate facing is the only non-arbitrary answer.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| DHD UI display | DONE | `setupStargateInfo` sends the gate list at world entry; right-clicking a DHD prop (`INT_DHD`, bit 16) emits `onDisplayDHD` (120) — see below |
| Gate arrival validation | DONE | Per-gate `stargates.arrival_*` pin, validated against the destination navmesh with a respawner fallback — see [Arrival placement](#arrival-placement) |
| Walking into the gate | SEE CA10 | `REGION_FLAG_Stargate` (bit 2) region routing and the 4-second dial timer are the Castle CA10 packet's, not on this branch |
| Stargate address tracking | DONE | `knownStargateAddresses` property, give/remove |
| Stargate zone transition | DONE | `base/world_entry/gate_travel/` — RESET_ENTITIES, persist destination, replay world entry |
| Gate-travel contact event | DONE | Fires `ECONTACT_LIST_EVENT_GateTravel` to the traveller's contacts with the destination `world_id` |
| Ring transporter interaction | DONE | Sends the destination list |
| Ring transport FSM | DONE | 8-state machine: IDLE through COOLDOWN |
| Ring player teleportation | DONE | Position-based teleport with visibility toggle |
| Ring Kismet sequences | DONE | `Region_Teleport_Out` / `Region_Teleport_In` |
| Ring movement locking | DONE | `BSF_MovementLock` set/unset during transport |
| Ring cross-world transport | PARTIAL | Same-world works; cross-world path exists but untested |
| Ring multi-player sync | FIXME | Only the first player in the region gets the Matinee — the sequence drives a shared world prop |
| Stargate open/close animation | NOT IMPL | `Stargate_MakeGate` (6100) and `Stargate_DestroyGate` (6103) are never emitted |
| Stargate crossing animation | NOT IMPL | `Stargate_CrossGate` (6113) never emitted |
| DHD chevron lock animations | NOT IMPL | Events 6106–6112 exist in the DB for every gate; never triggered |
| Stargate witness visibility | NOT IMPL | Even once gate sequences are emitted, they must fan to witnesses, not just the traveller |
| Squad leader gate travel | NOT IMPL | `processSquadLeaderGateTravel` defined; blocked on the group system |
| Gate address discovery | PARTIAL | `giveStargateAddressStr` / `removeStargateAddressStr` defined |

## DHD interaction

Right-clicking a prop whose `interaction_type_flags` carry `INT_DHD` (bit 16) opens the dialling UI. [`cell/interactions/dhd.rs`](../../crates/services/src/cell/interactions/dhd.rs) claims the interaction, looks up the stargate belonging to the **player's current world**, and emits `onDisplayDHD` (flat index 120) with a single `UINT8` — the gate's point-of-origin glyph.

Three things are easy to get wrong here:

- **`address_origin` is a glyph (1-38), not an identifier.** It repeats across rows (value 1 on both `SGC W2` and `SGC`, 13 on both Dakara E2 and E3), so it must never be used as a key into the `stargates` map, which is keyed by `stargate_id`. The emit validates against the **authored glyph range**, not just the wire's `UINT8` domain: `0`, `39`-`255` and anything that fails `u8::try_from` all refuse to emit. The column is `INT32` and the wire slot is `UINT8`, so neither type is the domain — a value outside 1-38 serialises perfectly cleanly and reaches the client as a DHD with no symbol to render, which reads as a client bug rather than the seed error it is. Refusing to emit is what makes it findable (`reason = "address_origin_out_of_range"`).
- **The known-address list is not sent here.** It rides `setupStargateInfo` at world entry; the client filters against what it already has.
- **Two gates on one world resolve deterministically** by lowest `stargate_id`, because `stargates` is a `HashMap` and an unordered pick would hand the client a different glyph across restarts.

A DHD prop on a world with no `stargates` row logs `reason = "no_stargate_for_world"` and shows the player nothing. The 2009 server sent a free-text `onError` here; Cimmeria's `onErrorCode` is an enum-coded surface with no free-text arm, so there is nowhere for that string to go. Acceptable while every seeded DHD has a gate.

**The dial itself is still unenforced.** `handle_dial_gate` validates that the target address exists in the `stargates` cache and that it is not the player's current world, but it does **not** check the player's `known_stargates` array, and a successful arrival does not append to it. The 2009 unlock-on-visit rule and the client-visible refusal are packet H06's; until then any client can dial any seeded gate.

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
