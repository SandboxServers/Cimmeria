# Gate Travel System

> **Last updated**: 2026-03-01
> **Status**: ~20% implemented

## Overview

Gate travel enables zone transitions via stargates and ring transporters. Stargates provide long-distance travel between worlds, while ring transporters provide local teleportation within or between nearby areas. Both systems involve multi-step sequences with animations, player visibility toggling, and movement locking.

The `GateTravel` class in `python/cell/GateTravel.py` is currently an empty stub. Ring transporters are fully implemented in `python/cell/RingTransporter.py` with an 8-state finite state machine.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| DHD UI display | DONE | `setupStargateInfo` sends gate lists to client |
| Stargate address tracking | DONE | `knownStargateAddresses` property, give/remove |
| Ring transporter interaction | DONE | `RingTransporter.interact()` sends destination list |
| Ring transport FSM | DONE | 8-state machine: IDLE through COOLDOWN |
| Ring player teleportation | DONE | Position-based teleport with visibility toggle |
| Ring Kismet sequences | DONE | `Region_Teleport_Out` / `Region_Teleport_In` |
| Ring movement locking | DONE | `BSF_MovementLock` set/unset during transport |
| Ring cross-world transport | PARTIAL | Same-world works; cross-world path exists but untested |
| Stargate zone transition | NOT IMPL | `processGateTravel` defined but GateTravel class is empty |
| Stargate animation | NOT IMPL | `stargateRotationOverride` defined |
| Squad leader gate travel | NOT IMPL | `processSquadLeaderGateTravel` defined |
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

- **Stargate addresses**: 29 defined in `db/resources.sql` (`ref_stargate`)
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
