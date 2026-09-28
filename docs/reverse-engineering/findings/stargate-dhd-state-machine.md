# Stargate DHD State Machine

> **Date**: 2026-05-13
> **Last updated**: 2026-09-28 (#1024: `onDHDReply` render-target resolved by static RE — VCommunicator chat cluster, not the DHD window)
> **Type**: Reference
> **Audience**: Engineers implementing or investigating gate travel
> **Phase**: V5 Documentation Campaign — W-content-mech Session 5; #1024 follow-up
> **Confidence**: HIGH for subscriber graph and event inventory; HIGH for `onDHDReply`'s render target (Communicator chat cluster, not the DHD window — static RE + uncooked Lua, no live packet capture); MEDIUM for state ordering
> **Sources**: Ghidra decompilation of SGW.exe (GUI session and `tools/re/ghidra-headless/Probe.java` headless probes); `EmitNetOut_onDialGate` at `0x00e2e120`; MemberCallback RTTI; `entities/defs/interfaces/Communicator.def`; uncooked client Lua at `Content/UI/Core/DHD/DHD.lua`; cross-reference to `gate-travel-wire-formats.md`

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

**Static audit: 2026-09-19.** The declaration is known; the binary payload and live-client presentation were unverified.

**Render target resolved: 2026-09-28 (#1024), by static RE.** `onDHDReply` does not render on the DHD window. It is bound to the same client-side `Communicator` class family that renders the player's chat/system-communication text, not to `GateTravel`'s DHD UI. See [Render-target resolution (#1024)](#ondhdreply-render-target-resolution-1024-2026-09-28) below for the full evidence chain. This is a static-RE conclusion (decompilation + uncooked Lua source), not a live packet-capture or visual observation in a running client.

| Evidence | What it establishes | Limit |
|---|---|---|
| [SGWPlayer.def](../../../entities/defs/SGWPlayer.def), `ClientMethods/onDHDReply` [DEF SGWPlayer.def:onDHDReply] | One argument: `WSTRING aMessage`; comment: "Give the client feedback on attempted DHD use" | Declared intent and signature, not a verified client payload decoder |
| [Canonical client dispatch table](../../protocol/client-method-dispatch-table.md), SGWPlayer row 100 | `onDHDReply` is client method **100**, with `WSTRING aMessage` | A method index does not establish when the server emits it |
| MemberCallback RTTI [SGW 0x00cf5440] | Recorded `Event_NetIn_onDHDReply` subscriber is VCommunicator | RTTI accessor only — doesn't show field decoding or a widget by itself; see the render-target resolution below for the cluster evidence that does |
| [Rust client-method constants](../../../crates/wire/src/cell/client_methods/player.rs), `ON_DHD_REPLY` | Constant exists with value 100 | No production send call uses it in the audited Rust tree |
| [Wire-log decoder](../../../crates/wire-log/src/wire_log/decoders/generated.rs), `decode_100` | Reads one `wstring()` as `aMessage`; [name table](../../../crates/wire-log/src/wire_log/client_names.rs) names method 100 | Diagnostic decoding is not a production emitter or independent client confirmation |

A search of `crates/` for `onDHDReply` and `ON_DHD_REPLY` finds the constant, a method-index comment in `mercury/mod.rs`, and the wire-log name/decoder. No production Rust emitter was found. The absence of a named call site is a static audit result, not a packet-capture observation. The declaration contains no NPC or gate entity-ID argument.

`onDisplayDHD` is a separate glyph-selection method and has a VGateTravel subscriber [SGW 0x00e2fd90]. Do not treat `onDHDReply` as a verified gate-state transition or assume it must be sent on every dial success or failure. The companion [wire-format note](gate-travel-wire-formats.md#ondhdreply--dhd-feedback-declared) records the declared argument without claiming a verified byte layout.

#### `onDHDReply` render-target resolution (#1024, 2026-09-28)

**Question:** does `onDHDReply`'s text render inside the DHD window, in chat, or as a toast?

**Answer: neither the DHD window nor a toast — it is bound to the client's `Communicator` chat/system-message component, the same class that renders `onPlayerCommunication`, `onSystemCommunication`, and `onTellSent`.**

Evidence chain, most specific first:

1. **The DHD CEGUI window has no text-display capability at all.** The uncooked client Lua at `Content/UI/Core/DHD/DHD.lua` (28 lines, read in full) does exactly two things: `DHDMod.onDHDVisibility` shows/hides `DHDWin` and calls `setDHDActive`/`DHD_Movie_Area:deactivate()`; `DHDMod.onCloseClicked` hides it. `DHD_Movie_Area` is bound via `setExternalWindowID(getExternalWindowIDForName("DHD"))` to an **external Scaleform movie** — the glyph-selection UI lives in `UI/Flash/DHD.upk` (compiled Flash/ActionScript, not Lua), confirmed present at `CookedPC/UI/Flash/DHD.upk` alongside the other minigame movies (`Hack.upk`, `Bypass.upk`, `Livewire.upk`, …). There is no CEGUI text widget, no event subscription, and no field in this window capable of consuming a `WSTRING` message. A recursive `grep -r DHDReply` across both the uncooked `Content/` Lua tree and the cooked `CookedPC/` tree (2026-09-28) finds zero hits outside `SGWPlayer.def` and the binary itself — no Lua anywhere references it.
2. **The DHD window's actual show/hide path is a separate, unrelated event chain that never touches `onDHDReply`:** server `onDisplayDHD` (registration stub `register_NetIn_onDisplayDHD` at `0x00d7a3c0`) fans out to the internal `Event_UI_DHDVisibility`, whose `TypedEmitInfo__vfunc_0` sits at `0x00e2fb50`; the UI-side listener is `SGWScriptedWindow_X_UEvent_UI_DHDVisibility___GameEventHandler__vfunc_0` at `0x00ce3290`, which is what ultimately calls into `DHDMod.onDHDVisibility` above. `onDHDReply`'s registration stub (`register_NetIn_onDHDReply` at `0x00d82c60`, confirmed by headless-Ghidra decompile: `return "Event_NetIn_onDHDReply";`) is a completely separate compilation unit from this chain.
3. **`onDHDReply`'s RTTI accessor sits inside a dense, uniformly-spaced (0x80 bytes) cluster of `VCommunicator` `MemberCallback::vfunc_3` accessors that is otherwise entirely chat/system-communication events**, confirmed by the V5 campaign's `worker-5c.checkpoint.json` rename log and re-verified 2026-09-28 by headless-Ghidra decompile of the neighboring accessors:

   | Address | Event | Class |
   |---|---|---|
   | `0x00cf51c0` | `Event_NetIn_onSystemCommunication` | `Communicator` |
   | `0x00cf5240` | `Event_NetIn_onChatJoined` | `Communicator` |
   | `0x00cf52c0` | `Event_NetIn_onNickChanged` | `Communicator` |
   | `0x00cf5340` | `Event_NetIn_onChatLeft` | `Communicator` |
   | `0x00cf53c0` | `Event_NetIn_onTellSent` | `Communicator` |
   | **`0x00cf5440`** | **`Event_NetIn_onDHDReply`** | **`Communicator`** |
   | `0x00cf54c0` | `Event_UI_MinigameText` | `Communicator` |
   | `0x00cf5540`–`0x00cf5ac0` | `SlashCmd_Tell`, `SlashCmd_Petition`, `SlashCmd_ChatJoin/Leave/SetAFKMessage/SetDNDMessage/Ignore/List/Mute/Unmute/Kick` | `Communicator` |

   `0x00cf51c0` and `0x00cf53c0` were independently re-decompiled for this session and confirm the exact same `MemberCallback<NoSubject, Communicator, void(__thiscall Communicator::*)(EventType const*, void*), EventType>::RTTI_Type_Descriptor` shape as `onDHDReply`'s own accessor — same class, same calling convention, differing only in the bound event type. `onDHDReply` sits between `onTellSent` and the minigame text event, not adjacent to anything DHD- or GateTravel-related.
4. **`Communicator` is the client's binary counterpart to a real, documented server interface** — [`entities/defs/interfaces/Communicator.def`](../../../entities/defs/interfaces/Communicator.def) declares exactly `onSystemCommunication`, `onPlayerCommunication`, `onLocalizedCommunication`, `onTellSent`, `onChatJoined`, `onChatLeft`, and `onNickChanged`: the chat system. `onDHDReply` is declared on `SGWPlayer.def` directly rather than on the `Communicator` interface (so it isn't in this list), but its native client-side subscriber is the same `Communicator` component — the developers wired one extra SGWPlayer-only feedback method into the class that already owns chat-text rendering, rather than adding it to the shared interface or to `GateTravel`.
5. Could not find a static, compile-time-constant handler body for `onDHDReply` specifically: per [`cme-event-signal.md`'s `CmeMemberCallback` struct layout](cme-event-signal.md#cmemembercallback-struct-layout), the bound method pointer lives at `+0x8` of a **heap-allocated** MemberCallback instance, set at construction time — it is not baked into any vtable, so it cannot be read from `vfunc_3`/`vfunc_5` alone. Locating the exact `Communicator::onDHDReply(...)` handler body (as opposed to its registration/RTTI scaffolding) is left as future work; it does not change the conclusion above, since the architectural placement (point 3) and the interface identity (point 4) already establish which UI subsystem owns the text.

**Conclusion for #1024:** `onDHDReply` is not a DHD-window message. It renders through the same native component that already renders `onPlayerCommunication` — i.e., functionally the same category of feedback [`dial_feedback`](../../../crates/cell-interactions/src/cell/gate_travel/dial_feedback.rs) already sends, with less certainty about its exact channel/styling (the `.def` gives it one bare `WSTRING`, no speaker or channel argument, unlike `onPlayerCommunication`'s four). Switching to it would trade a verified, byte-exact, already-tested format for an unverified one that appears to land in the same place. **Decision: keep the `onPlayerCommunication` chat line; `onDHDReply` is not used.**

#### Dial-refusal feedback (#727, 2026-09-28; render-target resolved #1024, 2026-09-28)

A refused dial is no longer silent. Every refusal in `handle_dial_gate` ([crates/cell-interactions/src/cell/gate_travel/mod.rs](../../../crates/cell-interactions/src/cell/gate_travel/mod.rs)), and the unrecoverable-arrival refusal in `perform_gate_travel`, sends the dialling player one `onPlayerCommunication("SYSTEM", 0, CHAN_feedback, text)` (client method 28) through `gate_travel::dial_feedback`:

| Refusal | Text |
|---|---|
| Address not in the player's book, or no such gate | `Failed to dial: not a known stargate address` (plus the existing `onErrorCode` 180) |
| No cell entity or space binding | `Failed to dial: you are not in a world yet, try again` |
| Destination is the world the player is on | `Failed to dial: you are already on that world` |
| Destination has no standable arrival | `Failed to dial: the destination gate cannot be reached right now` |

The first row covers both "not held" and "does not exist" with byte-identical traffic, so the answer is not an existence oracle.

**`onDHDReply` is not emitted, and now stays that way.** #1024's static RE ([above](#ondhdreply-render-target-resolution-1024-2026-09-28)) found it does not render on the DHD UI — it is routed through the same `Communicator` chat component as the `onPlayerCommunication` line this module already sends. The chat line remains the 2009 server's intent (`SGWPlayer.onError`, `deprecated/python/cell/SGWPlayer.py:879-884`, with the `Failed to dial: …` wording) on the channel the client shows without a popup. The 2009 call itself used an empty speaker and `CHAN_server` (8), which opens the modal "Server Message" prompt. `onDHDReply` was ruled out, not merely left aside: it is unlikely to be additive (same rendering family as the line already sent) and its own payload is less specific (one bare `WSTRING`, no channel or speaker) than the already-verified `onPlayerCommunication` call this module makes today.

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

The recorded MemberCallback RTTI identifies VCommunicator at `0x00cf5440`, separately from VGateTravel. **#1024 (2026-09-28) resolved what that means**: `0x00cf5440` sits inside a dense `VCommunicator` chat/system-communication accessor cluster (`onSystemCommunication`, `onChatJoined`, `onNickChanged`, `onChatLeft`, `onTellSent`, **`onDHDReply`**, `UI_MinigameText`, then the `SlashCmd_Chat*` family), and `Communicator` is the client counterpart of the documented [`Communicator.def`](../../../entities/defs/interfaces/Communicator.def) chat interface. `onDHDReply` renders through that chat component, not the DHD window — see [the render-target resolution](#ondhdreply-render-target-resolution-1024-2026-09-28) for the full evidence chain.

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
| `0x00cf51c0` | MemberCallback vfunc_3: **VCommunicator** × onSystemCommunication | Adjacent to onDHDReply in the chat cluster (#1024) |
| `0x00cf53c0` | MemberCallback vfunc_3: **VCommunicator** × onTellSent | Adjacent to onDHDReply in the chat cluster (#1024) |
| `0x00cf5440` | MemberCallback vfunc_3: **VCommunicator** × onDHDReply | Chat-cluster member, not a DHD-window handler (#1024) |
| `0x00d82c60` | `register_NetIn_onDHDReply` | Returns `"Event_NetIn_onDHDReply"`; decompiled 2026-09-28 |
| `0x00d7a3c0` | `register_NetIn_onDisplayDHD` | Feeds `Event_UI_DHDVisibility`, the DHD window's real show/hide chain — unrelated to onDHDReply (#1024) |
| `0x00e2fb50` | `Event_UI_DHDVisibility` TypedEmitInfo vfunc_0 | Internal event that shows/hides the DHD window |
| `0x00ce3290` | `SGWScriptedWindow` × `Event_UI_DHDVisibility` GameEventHandler vfunc_0 | Calls into `DHD.lua`'s `DHDMod.onDHDVisibility` |

---

## Open Questions

1. **StargateTriggerFailed wire fields** — event is confirmed present (RTTI + registration stub) but no emitter was found. Likely: a failure reason code (INT8 or INT32) or possibly zero-argument. Needs server-side `.py` or a live packet capture.
2. **Gate address struct layout** — the 6-glyph address is resolved from `this+0x18` (vector of pointers). The pointed-to struct layout is partially known: `FUN_00d2d8f0(ptr, index)` reads one UINT8 glyph. Full struct size unknown.
3. ~~**onDHDReply binary payload and UI**~~ — **Resolved 2026-09-28 (#1024) for the render target**: it is a `Communicator` chat-cluster event, not a DHD-window message; see the [render-target resolution](#ondhdreply-render-target-resolution-1024-2026-09-28). Still open: the exact `Communicator::onDHDReply(...)` handler body address (the bound method pointer lives in a heap instance, not a vtable — see point 5 of the resolution) and a live packet-capture/visual confirmation of the on-screen text. Neither is needed to answer #1024 (DHD window vs. not), but both would raise this from static-RE to observed-behavior confidence.
4. **Pending address vector** (`this+0x28`/`0x2c`) — what populates the pending list vs active list (`this+0x18`/`0x1c`)? Hypothesis: pending = addresses player knows but the local gate can't dial yet (e.g., requires server-side gate to be active). Needs Ghidra cross-reference on `updateStargateAddress` handler.

---

## Related Documents

- [gate-travel-wire-formats.md](gate-travel-wire-formats.md) — wire format tables and the declaration-derived `onDHDReply` signature
- [cme-event-signal.md](cme-event-signal.md) — CME EventSignal pipeline
- [right-click-routing-on-corpse.md](right-click-routing-on-corpse.md) — VGateTravel entity interaction context
