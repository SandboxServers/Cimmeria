# Gate Travel Wire Formats

> **Date**: 2026-03-01
> **Last updated**: 2026-09-19 (`onDHDReply` static audit only)
> **Type**: Reference
> **Audience**: Engineers implementing gate-travel messages
> **Phase**: 3 — Missing Systems RE
> **Confidence**: HIGH for the existing gate-travel findings; MEDIUM for the declaration-derived `onDHDReply` signature (binary payload and live-client UI unverified)
> **Sources**: `GateTravel.def`, `alias.xml`, Ghidra decompilation of universal RPC dispatcher (`0x00c6fc40`)

---

## Overview

Gate travel (Stargate dialing, address discovery, wormhole passage) uses the BigWorld entity method RPC system. All messages route through the universal RPC dispatcher — wire formats are derived directly from `.def` method signatures.

**Interface**: `GateTravel` (implemented by `SGWPlayer`)

---

## Client → Server Messages

All client→server messages are cell methods marked `<Exposed/>`, sent via `methodID | 0x80` header.

### `onDialGate` — Dial a Stargate

Player activates a DHD to dial a gate address.

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `TargetAddressId` | `INT32` | 4B | Index into `ref_stargate` table |
| `SourceAddressId` | `INT32` | 4B | Index into `ref_stargate` table |

**Total wire size**: 1B header + 8B args = **9 bytes**

---

## Server → Client Messages

Server→client messages are ClientMethods. The server sends a method ID + serialized args.

### `onDHDReply` — DHD Feedback (Declared)

Declared directly in [SGWPlayer.def](../../../entities/defs/SGWPlayer.def), `ClientMethods/onDHDReply`, rather than the GateTravel interface [DEF SGWPlayer.def:onDHDReply]. The [canonical client dispatch table](../../protocol/client-method-dispatch-table.md) assigns **method 100**. The declaration comment says "Give the client feedback on attempted DHD use."

| Argument | Declared Type | Evidence |
|---|---|---|
| `aMessage` | `WSTRING` | The method's only `<Arg>` in `SGWPlayer.def` |

**Confidence: MEDIUM for the payload signature.** This is declaration evidence, not a binary-verified byte layout. No method-specific length prefix, payload offset, or total wire size is established by this audit. The recorded MemberCallback RTTI associates `Event_NetIn_onDHDReply` with VCommunicator [SGW 0x00cf5440]; `onDisplayDHD` has a separate VGateTravel subscriber [SGW 0x00e2fd90]. Neither observation proves that an NPC entity ID selects the handler or that text appears in a particular chat/dialog widget.

**Rust status (2026-09-19):** `ON_DHD_REPLY = 100` is declared, and wire-log `decode_100` reads `aMessage` with `wstring()`, but no production Rust emitter was found. See the [DHD declaration and call-site audit](stargate-dhd-state-machine.md#ondhdreply-declaration-and-rust-audit) for source links and verification limits. A decoder and a constant alone do not implement feedback on attempted DHD use.

### `setupStargateInfo` — Initialize Gate Address Lists

Sent when player enters a world with stargates. Provides full gate address state.

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `worldStargateList` | `ARRAY<INT32>` | 4B count + N×4B | All stargates in current world |
| `knownStargateList` | `ARRAY<INT32>` | 4B count + N×4B | Addresses player has discovered |
| `hiddenStargateList` | `ARRAY<INT32>` | 4B count + N×4B | Addresses hidden from player |

**Wire size**: 1B header + 12B minimum (3 empty arrays) to variable

### `updateStargateAddress` — Single Address Update

Sent when player discovers or loses a gate address.

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `addressId` | `INT32` | 4B | Stargate address ID |
| `hasAddress` | `UINT8` | 1B | 1 = player knows this address |
| `hidden` | `UINT8` | 1B | 1 = address is hidden |

**Total wire size**: 1B header + 6B args = **7 bytes**

### `stargateRotationOverride` — Gate Ring Rotation

Controls client-side gate ring animation.

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `yaw` | `FLOAT` | 4B | Rotation angle in radians |

**Total wire size**: 1B header + 4B args = **5 bytes**

### `onStargatePassage` — Gate Travel Complete

Sent when player successfully travels through a stargate.

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `addressId` | `INT32` | 4B | Destination stargate address ID |

**Total wire size**: 1B header + 4B args = **5 bytes**

---

## Server-Only Methods (Not on Wire)

These methods exist in the `.def` but are **not client-exposed** — they are internal server→server calls via BigWorld mailboxes.

### CellMethods (non-Exposed)

| Method | Args | Notes |
|--------|------|-------|
| `giveStargateAddressStr` | `WSTRING AddressId`, `UINT8 Hidden` | GM command — add address by string name |
| `removeStargateAddressStr` | `WSTRING AddressId` | GM command — remove address by string name |
| `closeGatesTo` | `UINT32 AddressId` | Close all gates to a destination |
| `processGateTravel` | `PYTHON userData` | Internal: execute gate travel after validation |

### BaseMethods

| Method | Args | Notes |
|--------|------|-------|
| `processSquadLeaderGateTravel` | `INT32 id`, `PYTHON userData` | Squad leader coordinates group gate travel |
| `processGateTravel` | `PYTHON userData` | Internal gate travel processing |

---

## Properties

All gate travel properties are `CELL_PRIVATE` — none are synced to the client via property updates.

| Property | Type | Notes |
|----------|------|-------|
| `knownStargateAddresses` | `ARRAY<PYTHON>` | Server-side list of known addresses |
| `oldWorldID` | `INT32` | Previous world before gate travel |
| `gateCounter` | `INT32` | Gate usage counter |
| `destinationGate` | `INT32` | Current destination gate ID |
| `destinationGateArrivalTime` | `FLOAT` | When player will arrive at destination |

---

## Implementation Notes

1. **DHD Display**: The `onDisplayDHD` client method is on `SGWPlayer.def` directly (not GateTravel interface), taking `UINT8 PointOfOrigin` (1-38 glyph ID)
2. **Gate address IDs** are indices into a `ref_stargate` reference table (server data, not in `.def`)
3. **Gate travel flow**: Client dials (`onDialGate`) → server validates → server processes travel internally → client receives `onStargatePassage` + map load
4. **Squad gate travel**: Leader's `processSquadLeaderGateTravel` coordinates movement for all squad members through base mailboxes
