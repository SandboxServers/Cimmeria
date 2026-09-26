# Stargate DHD State Machine

> **Date**: 2026-05-13
> **Last updated**: 2026-09-19 (static declaration and Rust call-site audit; no new binary or live-client verification)
> **Type**: Reference
> **Audience**: Engineers implementing or investigating gate travel
> **Phase**: V5 Documentation Campaign — W-content-mech Session 5
> **Confidence**: HIGH for subscriber graph and event inventory; MEDIUM for state ordering and the declaration-derived `onDHDReply` payload; its binary payload and visible UI remain unverified
> **Sources**: Ghidra decompilation of SGW.exe; `EmitNetOut_onDialGate` at `0x00e2e120`; MemberCallback RTTI; cross-reference to `gate-travel-wire-formats.md`

---

## Overview

Gate travel is split across three client classes. `class_GateTravel` (VGateTravel) owns the wire protocol and gate address management. `class_Communicator` (VCommunicator) has the recorded `onDHDReply` subscriber. `class_GameProxyPlayer` (VGameProxyPlayer) owns ring transporter destination lists. There is no single monolithic DHD state machine class; state is implicit in the sequence of events.

The `USeqEvent_Stargate` (`0x0069fba0`) is a Kismet sequence event used for Unreal Engine in-level scripting; it is a presentation layer and does not drive the protocol.

---

## Client Class Subscription Table

### class_GateTravel (VGateTravel) Subscriptions

All confirmed from MemberCallback vfunc_3 RTTI descriptors:

| Event | Direction | MemberCallback vfunc_3 | Notes |
|---|---|---|---|
| `Event_NetIn_setupStargateInfo` | Server→Client | `0x00e2fe10` | Full gate address initialization on world entry |
| `Event_NetIn_updateStargateAddress` | Server→Client | `0x00e2fe90` | Single address add/remove |
| `Event_NetIn_StargateRotationOverride` | Server→Client | `0x00e2ff10` | Gate ring rotation animation |
| `Event_NetIn_StargateTriggerFailed` | Server→Client | `0x00e2ff90` | **New** — not in gate-travel-wire-formats.md |
| `Event_NetIn_onDisplayDHD` | Server→Client | `0x00e2fd90` | Server tells client to show DHD UI |
| `Event_NetIn_onStargatePassage` | Server→Client | `0x00e30010` | Travel complete |
| `Event_Sys_FrameStart` | Internal | `0x00e2fc90` | Per-frame update (animation/state tick) |
| `Event_Cache_ElementReady<DBGateInfo>` | Internal | `0x00e2fd10` | Gate info DB cache warming |
| `Event_World_StargateEvent` | World | `0x00e30090` | Level-scripted gate event |
| `Event_World_DialStargateAddress` | World | `0x00e30110` | Level-scripted dial trigger |
| `Effect_EffectWithUserDataApplied` (XVGateTravel) | Internal | `0x00e30b90` | Effect applied to gate entity |
| `Effect_EffectWithUserDataRemoved` (XVGateTravel) | Internal | `0x00e30c10` | Effect removed from gate entity |

### class_Communicator (VCommunicator) Subscriptions (DHD)

| Event | MemberCallback vfunc_3 | Notes |
|---|---|---|
| `Event_NetIn_onDHDReply` | `0x00cf5440` | Recorded subscriber is VCommunicator; payload and visible UI not established by RTTI |

The MemberCallback RTTI associates `Event_NetIn_onDHDReply` with VCommunicator [SGW 0x00cf5440]. This places the recorded subscriber alongside communication events, separately from the VGateTravel subscriber for `onDisplayDHD` [SGW 0x00e2fd90]. It does not establish the exact UI, prove an NPC dialogue flow, or show that an entity ID selects the subscriber. The `.def` describes the method as feedback on attempted DHD use; see the [declaration and implementation audit](#ondhdreply-declaration-and-rust-audit).

### class_GameProxyPlayer (VGameProxyPlayer) Subscriptions (Ring Transporter)

| Event | MemberCallback vfunc_3 | Notes |
|---|---|---|
| `Event_NetIn_onRingTransporterList` | `0x00df7900` | List of available ring destinations |

---

## Wire Format — Key Confirmed Fields

### `EmitNetOut_onDialGate` Field Layout (confirmed from decompilation at `0x00e2e120`)

```text
Event_NetOut_onDialGate {
    TargetAddressId: INT32   // index resolved from 6-glyph address comparison
    SourceAddressId: INT32   // index resolved from this entity's address
}
```

The emitter at `0x00e2e120` performs:

1. Validates target entity type via `FUN_00e2ba80` + `FUN_00d2d910`
2. Searches `this+0x18`/`this+0x1c` (active address vector) for 6-glyph match via `FUN_00d2d8f0` (reads one glyph per call, 6 iterations)
3. Falls through to `this+0x28`/`this+0x2c` (pending address vector) if not found in active
4. On match: `scalable_malloc(0xC)` → stamps `Event_NetOut_onDialGate::vftable`
5. Sets field `"TargetAddressId"` via `CmeEventSignal_SetField` (`0x0043b850`)
6. Sets field `"SourceAddressId"` via `CmeEventSignal_SetField`
7. Dispatches via `FUN_00e30f20`

The 6-glyph Stargate address is stored as a struct at `this+0x18` offset (INT32 length, pointer-backed vector). Each glyph is a UINT8 read via `FUN_00d2d8f0(addr, glyphIndex)`.

**Confirms `gate-travel-wire-formats.md`:** TargetAddressId and SourceAddressId are both INT32 resolved IDs (indices), not raw 6-glyph addresses. Wire matches the `.def`.

### `EmitNetOut_SetRingTransporterDestination` Field Layout (confirmed from `0x00aeab70`)

```text
Event_NetOut_SetRingTransporterDestination {
    aRegionId:      INT32   // ring transporter region
    aDestinationId: INT32   // destination within region
}
```

Constructor: `EventNetOut_SetRingTransporterDestination_Ctor` at `0x00ae9d70` stamps `NetworkEvent::vftable` then `Event_NetOut_SetRingTransporterDestination::vftable`. Signal object is 0xC bytes. Slash-command path (`SlashCmd_EmitSetRingTransporterDestination` at `0x00c8a830`) reads `"regionId"` and `"destinationId"` from the slash-command event then re-emits as `"aRegionId"` / `"aDestinationId"` — note field name change (no `a`-prefix in slash event, `a`-prefix in wire event). Source path confirmed: `.\Src\SGWTextCommandManager.cpp` lines 0xC35–0xC36.

---

## DHD State Machine (Inferred from Event Sequence)

No persistent client-side state enum was found. The state machine is implicit:

```text
STATE: idle
  │
  │  [Server sends onDisplayDHD (PointOfOrigin: UINT8)]
  ▼
STATE: dhd_visible
  │  Client shows DHD UI; player selects glyphs manually
  │
  │  [Player dials — client resolves 6-glyph → address IDs]
  │  [Client sends: Event_NetOut_onDialGate {TargetAddressId, SourceAddressId}]
  ▼
STATE: dialing
  │
  │  [Server sends: StargateRotationOverride {yaw: FLOAT}]  ← ring animation
  │
  │  On failure:
  │  [Server sends: Event_NetIn_StargateTriggerFailed]  ← NEW event
  │  → back to dhd_visible or idle
  │
  │  On success:
  │  [Server sends: onStargatePassage {addressId: INT32}]
  ▼
STATE: passage
  │  Client performs level load / map transition
  │
  │  [On new world entry: setupStargateInfo]
  │  [setupStargateInfo: {worldStargateList, knownStargateList, hiddenStargateList}]
  ▼
STATE: idle (new world)

Incremental address updates (any time):
  [Server sends: updateStargateAddress {addressId, hasAddress: UINT8, hidden: UINT8}]
  → VGateTravel updates local address set (active+pending vectors)
```

### `onDHDReply` Declaration and Rust Audit

**Static audit: 2026-09-19.** The declaration is known; the binary payload and live-client presentation remain unverified.

| Evidence | What it establishes | Limit |
|---|---|---|
| [SGWPlayer.def](../../../entities/defs/SGWPlayer.def), `ClientMethods/onDHDReply` [DEF SGWPlayer.def:onDHDReply] | One argument: `WSTRING aMessage`; comment: "Give the client feedback on attempted DHD use" | Declared intent and signature, not a verified client payload decoder |
| [Canonical client dispatch table](../../protocol/client-method-dispatch-table.md), SGWPlayer row 100 | `onDHDReply` is client method **100**, with `WSTRING aMessage` | A method index does not establish when the server emits it |
| MemberCallback RTTI [SGW 0x00cf5440] | Recorded `Event_NetIn_onDHDReply` subscriber is VCommunicator | Does not verify field decoding, a displayed text widget, or subscriber selection by entity ID |
| [Rust client-method constants](../../../crates/wire/src/cell/client_methods/player.rs), `ON_DHD_REPLY` | Constant exists with value 100 | No production send call uses it in the audited Rust tree |
| [Wire-log decoder](../../../crates/services/src/wire_log/decoders/generated.rs), `decode_100` | Reads one `wstring()` as `aMessage`; [name table](../../../crates/services/src/wire_log/client_names.rs) names method 100 | Diagnostic decoding is not a production emitter or independent client confirmation |

A search of `crates/` for `onDHDReply` and `ON_DHD_REPLY` finds the constant, a method-index comment in `mercury/mod.rs`, and the wire-log name/decoder. No production Rust emitter was found. The absence of a named call site is a static audit result, not a packet-capture observation. The declaration contains no NPC or gate entity-ID argument, and neither the declaration nor the recorded subscriber establishes that changing the RPC's entity ID would route it to VCommunicator.

`onDisplayDHD` is a separate glyph-selection method and has a VGateTravel subscriber [SGW 0x00e2fd90]. Do not treat `onDHDReply` as a verified gate-state transition or assume it must be sent on every dial success or failure. Confirm the client handler's field consumption and visible feedback in a live session before specifying the missing send path. The companion [wire-format note](gate-travel-wire-formats.md#ondhdreply--dhd-feedback-declared) records the declared argument without claiming a verified byte layout.

---

## Ring Transporter Chain (Extended from gate-travel-wire-formats.md)

```text
[Player approaches ring transporter platform]
       │
       ▼
[Server sends: Event_NetIn_onRingTransporterList]  → VGameProxyPlayer handles
       │  Fields (from .def): list of available destinations
       │
       ▼
[Client shows ring transporter destination UI]
       │
       ▼
[Player selects destination]
[Client sends: Event_NetOut_SetRingTransporterDestination {aRegionId, aDestinationId}]
       │  (confirmed from emitter at 0x00aeab70)
       │
       ▼
[Server processes ring transport — separate from gate travel pipeline]
```

---

## New Findings vs gate-travel-wire-formats.md

### 1. StargateTriggerFailed (new event, not in existing doc)

`Event_NetIn_StargateTriggerFailed` is subscribed by VGateTravel at MemberCallback `0x00e2ff90`. Registration stub: `0x00d88060`. TypedEmitInfo destructor: `0x00d88140` → inner `FUN_00d880e0`. Wire fields unknown — no emitter found for this event (server-side only). Likely sent when: dialing fails (wrong address, gate busy, destination unreachable, or address not in player's known list).

### 2. onDHDReply is VCommunicator, not VGateTravel

The recorded MemberCallback RTTI identifies VCommunicator at `0x00cf5440`, separately from VGateTravel. The [static audit above](#ondhdreply-declaration-and-rust-audit) distinguishes that subscriber evidence from the declared `WSTRING aMessage` and the still-unverified payload decoding and UI behavior.

### 3. onDisplayDHD is VGateTravel

`gate-travel-wire-formats.md` notes `onDisplayDHD` is on `SGWPlayer.def` directly. Confirmed by VGateTravel MemberCallback RTTI at `0x00e2fd90`.

### 4. World events drive level-scripted gate sequences

VGateTravel subscribes to `Event_World_StargateEvent` and `Event_World_DialStargateAddress`. These are Kismet-driven events from UE3 level scripts — used for scripted story sequences where the Stargate is triggered by mission logic rather than player DHD input.

### 5. Gate address is 6-glyph byte array on client

The client stores Stargate addresses as 6-element arrays of UINT8 glyphs. The resolve step in `EmitNetOut_onDialGate` converts the 6-glyph local representation to a server-side INT32 address ID before sending.

---

## Key Addresses

| Address | Symbol | Notes |
|---|---|---|
| `0x00e2e120` | `EmitNetOut_onDialGate` | Sets TargetAddressId + SourceAddressId; 6-glyph→INT32 resolution |
| `0x00aeab70` | `EmitNetOut_SetRingTransporterDestination` | aRegionId + aDestinationId (INT32 pair) |
| `0x00ae9d70` | `EventNetOut_SetRingTransporterDestination_Ctor` | 0xC-byte NetworkEvent ctor |
| `0x00c8a830` | `SlashCmd_EmitSetRingTransporterDestination` | Slash-cmd path; source: SGWTextCommandManager.cpp L0xC35–C36 |
| `0x00e30f20` | Dispatch helper (onDialGate path) | Allocates 0x18-byte emit info; dispatches via subscriber list |
| `0x00d2d8f0` | Glyph accessor | Returns one glyph byte from address struct by index |
| `0x00d2d910` | Entity-type validator | Used by EmitNetOut_onDialGate to check address entity type |
| `0x0069fba0` | `USeqEvent_Stargate__vfunc_0` | Kismet stub; returns 1 |
| `0x006a0a40` | `USeqEvent_Stargate__vfunc_92` | Kismet activation: checks bit0 of `this+0xDC`, fires if gate ID matches |

---

## Open Questions

1. **StargateTriggerFailed wire fields** — event is confirmed present (RTTI + registration stub) but no emitter was found. Likely: a failure reason code (INT8 or INT32) or possibly zero-argument. Needs server-side `.py` or a live packet capture.
2. **Gate address struct layout** — the 6-glyph address is resolved from `this+0x18` (vector of pointers). The pointed-to struct layout is partially known: `FUN_00d2d8f0(ptr, index)` reads one UINT8 glyph. Full struct size unknown.
3. **onDHDReply binary payload and UI** — the declaration is `WSTRING aMessage` at client method 100, but the client decoder and presentation path have not been confirmed from binary or live testing. No production Rust emitter was found in the static audit. Trace the complete client handler and verify visible feedback before implementing a send path.
4. **Pending address vector** (`this+0x28`/`0x2c`) — what populates the pending list vs active list (`this+0x18`/`0x1c`)? Hypothesis: pending = addresses player knows but the local gate can't dial yet (e.g., requires server-side gate to be active). Needs Ghidra cross-reference on `updateStargateAddress` handler.

---

## Related Documents

- [gate-travel-wire-formats.md](gate-travel-wire-formats.md) — wire format tables and the declaration-derived `onDHDReply` signature
- [cme-event-signal.md](cme-event-signal.md) — CME EventSignal pipeline
- [right-click-routing-on-corpse.md](right-click-routing-on-corpse.md) — VGateTravel entity interaction context
