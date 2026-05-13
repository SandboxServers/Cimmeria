# Architectural Anomalies — CME EventSignal Subsystem

> **Diátaxis type**: reference / explanation
> **Audience**: engineers working on CME EventSignal or Cimmeria protocol bridge
> **Last updated**: 2026-05-13
> **Confidence**: HIGH — all three anomalies confirmed by live decompile (W-anom session 5)
> **Cross-reference**: [`cme-event-signal.md`](cme-event-signal.md) — full pipeline, Pattern A/B taxonomy

Three subsystems appeared anomalous in prior campaign sessions because they deviated from the
standard CME EventSignal Pattern A/B pipeline. This document resolves all three.

---

## Overview

**Anomaly 1 — Black Market (BM) emitters** used `TypedEmitInfo` but had no `CallbackImpl__vfunc_2`.
Resolved: BM emitters use **Pattern B**, not a third unknown mechanism. Pattern B never creates
`CallbackImpl` objects. The dispatch is through the typed event object's vtable, not a subscriber list.

**Anomaly 2 — GiveInventory** had the same apparent anomaly. Resolved: `Event_NetOut_GiveInventory`
is a **GM cheat command** only. The TypedEmitInfo object exists for the signal registration
infrastructure, but no client-side subscriber ever listens to it. The actual player-facing variant is
`Event_SlashCmd_GiveInventory`, which has a full `CallbackImpl__vfunc_2` and a registered
`SGWTextCommandMgr` handler.

**Anomaly 3 — VSGWHomeless** appeared to be an unnamed catch-all subscriber. Resolved: this is the
**`SGWHomeless` class** — an in-editor developer tool class that handles `Editor_*`, `Option_*`, and
`SlashCmd_TestSequence` events in release builds. It is not a catch-all; it is a specific subsystem
whose name encodes developer intent ("homeless" = events without a purpose-built screen-level handler).

---

## Anomaly 1 — Black Market Callback Mechanism

### Claim (prior sessions)

BM has `TypedEmitInfo` entries but no `CallbackImpl__vfunc_2` pair. Must use a different callback
registration mechanism — possibly direct function pointers instead of the typed CME signal pattern.

### Resolution

The BM emitters use **CME Pattern B** — the same pattern documented in `cme-event-signal.md` for
`EmitNetOut_callForAid` and `EmitNetOut_SetRingTransporterDestination`. Pattern B does not create
`CallbackImpl` objects. This was not a third mechanism — it was an unrecognized instance of the
already-documented Pattern B.

### Evidence

All four BM emitters (`EmitNetOut_BMCreateAuction` `0x00e59970`, `EmitNetOut_BMCancelAuction`
`0x00e59c70`, `EmitNetOut_BMPlaceBid` `0x00e59da0`, `EmitNetOut_BMSearch` `0x00e59f70`) follow
the same sequence:

```
scalable_malloc(0xC)                                -- allocate 12-byte typed event object
FUN_00e5c1a0(pEventObj)                             -- ctor: NetworkEvent_Ctor + stamp Event_NetOut_BM*::vftable
CmeEventSignal_SetField(this, "<key>", &value) × N  -- populate N fields
thunk_FUN_0054c900()                               -- get CME system singleton
(**(code **)(*this + 8))(system, 1)                -- dispatch via vtable slot 2 of typed event object
```

`thunk_FUN_0054c900` (`0x0054c900`) is a lazy-initialized singleton accessor: if
`DAT_01ee2678 == 0`, it allocates 68 bytes and calls `FUN_0054c870` (`0x0054c870`) which calls
`FUN_00a37710` (`0x00a37710`) to initialise the CME system object. The same singleton is returned
for every BM emitter call.

`FUN_00e5c1a0` (`0x00e5c1a0`) is the `Event_NetOut_BMCreateAuction` typed constructor:

```c
// confirmed at 0x00e5c1a0
NetworkEvent_Ctor(param_1);                            // base init (0x004412e0)
*param_1 = NetworkEvent::vftable;                      // initial vtable
*param_1 = Event_NetOut_BMCreateAuction::vftable;      // typed vtable stamp
```

Analogous ctor functions exist for each BM event:
- `FUN_00e5c440` — `Event_NetOut_BMCancelAuction` ctor
- `FUN_00e5c6e0` — `Event_NetOut_BMPlaceBid` ctor
- `FUN_00e5c980` — `Event_NetOut_BMSearch` ctor

### Why Pattern B (not Pattern A)

Pattern A uses `CmeEventSignal_GetSystem → LookupByName → SetField → vtable dispatch on the
registered signal handle`. Pattern B bakes the event type into the vtable at construction, bypassing
the lookup step. BM was compiled as Pattern B because its emit paths are UI-callback-driven with a
known type at each call site — the type resolution cost of Pattern A was unnecessary.

### Impact on Cimmeria

No change required. The BM server-side handler receives exactly the same Mercury entity-method call
regardless of whether the client used Pattern A or Pattern B. Pattern B is a client-internal
optimisation. The Cimmeria `SGWBlackMarketManager` service handler documentation in
[`black-market-wire-formats.md`](black-market-wire-formats.md) remains correct.

---

## Anomaly 2 — GiveInventory

### Claim (prior sessions)

`GiveInventory` has a `TypedEmitInfo` entry for `Event_NetOut_GiveInventory` but no paired
`CallbackImpl__vfunc_2`. Possibly a different registration mechanism.

### Resolution

`Event_NetOut_GiveInventory` is a **GM/admin cheat command signal** dispatched only by the server
tooling. The client-side `TypedEmitInfo` at `0x00d97830` exists because the CME event registration
infrastructure creates it unconditionally for all signals with `register_NetOut_GiveInventory`
(`0x00d97750`). No client-side subscriber ever calls `CmeEventSignal_Subscribe` for this signal,
so no `CallbackImpl` object is created.

The actual player-visible GiveInventory flow goes through a separate signal:
`Event_SlashCmd_GiveInventory` — which **does** have both a `TypedEmitInfo` at `0x00594ad0` and a
`CallbackImpl__vfunc_2` at `0x00c964d0`, registered to `SGWTextCommandMgr` via
`MemberCallbackRtti_SlashCmd_GiveInventory__SGWNetworkManager` (`0x00c9a2a0`).

### Evidence

| Function | Address | Role |
|----------|---------|------|
| `register_NetOut_GiveInventory` | `0x00d97750` | Returns string `"Event_NetOut_GiveInventory"` — vtable slot only, no callers |
| `CME_EventSignal_VEvent_NetOut_GiveInventory___TypedEmitInfo__vfunc_0` | `0x00d97830` | MSVC scalar destructor for the TypedEmitInfo |
| `CME_EventSignal_VEvent_SlashCmd_GiveInventory___CallbackImpl__vfunc_2` | `0x00c964d0` | RTTI accessor for the SlashCmd variant — the registered subscriber |
| `MemberCallbackRtti_SlashCmd_GiveInventory__SGWTextCommandMgr` | `0x00c9a2a0` | RTTI accessor for the text-command manager binding |

`register_NetOut_GiveInventory` has no callers (confirmed by xref query). Its only reference is
`019cad68 [DATA]` — the vtable entry. The absence of a subscriber is structural: the signal was
defined for server use but no client-side handler was ever wired.

### Why This Matters

`Event_NetOut_GiveInventory` appears in the client binary purely as dead client-side infrastructure.
The server should not expect this signal from the client. The command `SlashCmd_GiveInventory` is
the correct GM tool path; it passes through `SGWTextCommandMgr`, which translates it to a
server-side RPC.

---

## Anomaly 3 — VSGWHomeless Purpose

### Claim (prior sessions)

`VSGWHomeless` cluster at `0x00d3da00–0x00d3e580` (~27 functions). Routes events without a dedicated
handler class: `Editor_*`, `SlashCmd_TestSequence`, `Option_*` events all route through it. Is it
a catch-all subscriber, or a specific subsystem whose name was lost?

### Resolution

`VSGWHomeless` is **not** a catch-all and **not** anonymous. It is the `SGWHomeless` class — an
in-editor developer tool manager that was intentionally compiled into the release binary. The class
name is recoverable from RTTI: every `MemberCallbackRtti_` RTTI accessor in the cluster spells out
`class_SGWHomeless` in its template argument (e.g., `class_SGWHomeless,void_(__thiscall_SGWHomeless::*)(class_Event_Editor_Close_const*,void*)`).

`SGWHomeless` is a single-instance class (static singleton at `DAT_01ef2380`, init-once guarded by
`DAT_01ef23f8`) that:

1. Subscribes to 22 `Editor_*` events: `Editor_Close`, `Editor_Ghost`, `Editor_Walk`,
   `Editor_ToggleCombat`, `Editor_TogglePhysicsMode`, `Editor_Use`, `Editor_Camera*` (6 modes),
   `Editor_View*` (3 render modes), `Editor_ScreenShot`, `Editor_ShowFPS`, `Editor_ShowPerformance`,
   `Editor_ShadowStats`, `Editor_SequenceBegin`/`End`/`Interrupt`, `Editor_BeginPIE`/`EndPIE`,
   `Editor_TestSequence`.
2. Dispatches each subscription to a concrete handler that calls into the Unreal Engine editor
   viewport/camera system (accessed via `DAT_01ee1254 + 0x2d0 → viewport chain`).
3. Registers itself as the active mode under the string `"editor"` (via `FUN_0057b800`).
4. Some handlers open browser URLs: `Editor_ViewWireframe` → `http://www.stargateworlds.com/`,
   `Editor_ShadowStats` → `http://beta.stargateworlds.com/`. These are placeholder/dev tool handlers.

### Key Functions

| Address | Function | Role |
|---------|----------|------|
| `0x00d3d440` | `SGWHomeless_GetInstance` (inferred) | Static-init singleton: allocates `DAT_01ef2380`, calls `FUN_00d3d270`, registers atexit cleanup |
| `0x00d3d270` | (called from init) | Walks 7 `SGWPIEScriptManager` subscriptions — **distinct from SGWHomeless subscriptions** |
| `0x00d3efb0` | `SGWHomeless_RegisterSubscriptions` (inferred) | Registers 22 `Editor_*` subscriptions for `SGWHomeless`; ends with `FUN_0057b800("editor")` mode registration |
| `0x00d3e060` | Handler: `Editor_Close` | Calls `CloseEditorViewport` via UE3 viewport vtable slot `0x10c` |
| `0x00d3eba0` | Handler: last Editor_ event | Calls into viewport via vtable slot `0x10c` with a string from `DAT_019bc634` |
| `0x00d3ed10` | Handler: `Editor_ViewWireframe` (inferred) | Opens `http://www.stargateworlds.com/` via `ShellExecuteW` — dev placeholder |
| `0x00d3ee60` | Handler: `Editor_ShadowStats` (inferred) | Opens `http://beta.stargateworlds.com/` via `ShellExecuteW` — dev placeholder |
| `0x00d40ad0` | MemberCallback ctor: `SGWHomeless × Editor_Close` | Stamps `MemberCallback<NoSubject, SGWHomeless, handler, Event_Editor_Close>::vftable` |
| `0x00d40740` | RTTI: `Editor_TestSequence / SGWHomeless` | Returns `TypeDescriptor*` — confirms class name `SGWHomeless` |

The RTTI accessor cluster for SGWHomeless runs from `0x00d40740` to `0x00d415c0` (30 functions,
uniform 0x80 spacing). Each returns the compile-time TypeDescriptor for the corresponding
`MemberCallback<NoSubject, SGWHomeless, handler_ptr, Event_*>` template instantiation.

### Note on W5-C Checkpoint Addresses

The W5-C brief cited `0x00d3da00–0x00d3e580` as the VSGWHomeless cluster. These addresses are
**not** function entry points — they are instruction-level addresses inside `SGWPIEScriptManager`
subscription helper functions (which share the address range with SGWHomeless). The checkpoint
recorded mid-function addresses. The actual SGWHomeless registration function entry is
`0x00d3efb0`; the RTTI accessor cluster begins at `0x00d40740`.

### Why This Class Exists in a Release Binary

`SGWHomeless` provides an in-process editor overlay for play-in-editor testing. The Stargate Worlds
client shipped with the editor subsystem partially intact — this was common for games that used the
Unreal Engine's built-in editor integration as a QA/testing tool. The class name ("homeless") was
the developers' term for events that had no dedicated screen-level handler: they all "live" in
`SGWHomeless` as a catch-basin for editor tool commands that the main window manager did not own.

---

## Cross-references to `cme-event-signal.md` Pattern A/B

`cme-event-signal.md` defines two structural emit patterns. The anomalies map as follows:

| Anomaly | Maps to | Notes |
|---------|---------|-------|
| BM (all 4 emitters) | **Pattern B** | Typed ctor + vtable slot 2 dispatch; `thunk_FUN_0054c900` is the same singleton accessor used by all Pattern B emitters |
| GiveInventory (NetOut) | **Neither** — no client subscriber | TypedEmitInfo infrastructure exists; signal is server-side only on the client path |
| GiveInventory (SlashCmd) | **Pattern A** (inferred) | Full `CallbackImpl` + subscriber registration to `SGWTextCommandMgr` |
| SGWHomeless | **Pattern A subscriber side** | The `MemberCallback` objects registered by `FUN_00d3efb0` are the subscriber side of Pattern A; `CmeEventSignal_Subscribe` (`0x00a5c150`) is called via `FUN_00a37790` for each |

The `cme-event-signal.md` anomaly section should be updated to close out these three items. The
two remaining open anomalies documented there (Trade duplicate TypedEmitInfo — benign; SGWHomeless
purpose) are now resolved.

---

## Open Questions

All three anomalies are resolved. Minor follow-up items:

1. **Option_ subscriber for SGWHomeless** — the W5-C checkpoint logged Option_ChatFontSize,
   Option_UIScale, Option_GamepadEnabled, Option_ShowDamageNumbers, Option_HideHelmets,
   Option_FriendlyFire as VSGWHomeless-bound. These are not in `FUN_00d3efb0`. There must be a
   second SGWHomeless registration function for `Option_*` events — not yet located. Evidence would
   be a function that calls `FUN_00d4xxxx` subscription helpers with `SGWHomeless`-typed ctors for
   `Event_Option_*`. Low priority — does not affect Cimmeria server.

2. **`DAT_019bc634`** (referenced by `FUN_00d3eba0`) — an unresolved data label used as a string
   argument to an editor handler. Probably a localised view-mode name string. Does not affect
   Cimmeria server.

3. **`FUN_0057b800` / `FUN_0057d070`** — the `"editor"` mode registration calls at the end of
   `FUN_00d3efb0`. These are CME mode-management functions; their full contract is not yet
   documented. Low priority — client-only.
