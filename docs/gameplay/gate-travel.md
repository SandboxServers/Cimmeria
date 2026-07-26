---
title: "Gate Travel System"
type: reference
audience: engineers
last_updated: 2026-07-25
---

# Gate Travel System

> **Last updated**: 2026-07-25
> **Status**: Zone transition and ring transport both work. Every gate *animation* is missing.

## Overview

Gate travel enables zone transitions via stargates and ring transporters. Stargates provide long-distance travel between worlds, while ring transporters provide local teleportation within or between nearby areas. Both systems involve multi-step sequences with animations, player visibility toggling, and movement locking.

Stargate zone transition is implemented in [`base/world_entry/gate_travel/`](../../crates/services/src/base/world_entry/gate_travel/): on `CellToBaseMsg::GateTravel` the base sends RESET_ENTITIES to tear down the client's view of the old space, persists the destination world and position, and seeds `pending_world_entry` so the client's next ENABLE_ENTITIES drives a fresh create-player + enter-world cycle. Ring transport lives in [`cell/ring_transport/`](../../crates/services/src/cell/ring_transport/) with an 8-state finite state machine.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| DHD UI display | DONE | `setupStargateInfo` sends gate lists to client |
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

- **Stargate addresses**: 28 in `db/resources/Worlds/Seed/stargates.sql`
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
