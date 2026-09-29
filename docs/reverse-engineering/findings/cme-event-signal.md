# CME EventSignal — Emit Pipeline & Class Anatomy

> **Diátaxis type**: reference
> **Audience**: engineers working on protocol decompilation or Cimmeria's CME-bridge surface
> **Last updated**: 2026-09-28
> **Confidence**: HIGH (decompiled across W1 + W2 + W3 of V5 Documentation Campaign session 1)

Every `CME::EventSignal` emit on the client side follows a fixed five-call pipeline; every `TypedEmitInfo__vfunc_0` is the MSVC scalar destructor (not a name accessor); every `CallbackImpl__vfunc_2` is the RTTI type-name accessor (returning a `TypeDescriptor*`, not a name string).

## Discovery context

Surfaced and cross-confirmed by W1 + W2 + W3 of the V5 Documentation Campaign session 1 (2026-05-12). W1's `EmitNetOut_DebugMinigameInstance` (`0x00c79120`) — the only non-stub function in W1's scope — exposed the full pipeline. W2 + W3 independently confirmed the destructor and RTTI-accessor findings across 363 TypedEmitInfo + CallbackImpl functions. See [`../v5-campaign/CAMPAIGN_STATUS.md`](../v5-campaign/CAMPAIGN_STATUS.md) for the per-worker reports.

## The emit pipeline

Every client-side `Event_NetOut_*` emit funnels through the addresses below. Two structural patterns exist (confirmed W-emit-A + W-emit-B, session 3):

| Address | Name | Role |
|---------|------|------|
| `0x0155f790` | `CmeEventSignal_GetSystem` | Returns the event registry singleton. See [the 2026-09-28 correction](#correction-2026-09-28-the-registry-is-an-event-factory-not-a-subscriber-api). |
| `0x00a5c0f0` | `CmeEventSignal_LookupByName` | Creates a new event object from its class name (`const std::string&`). See the correction. |
| `0x0043b850` | `CmeEventSignal_SetField` | Set a key/value field on a signal object. |
| `0x00cb1f00` | `CmeEventSignal_SetFieldHelper` | SetField wrapper: acquires handle via `FUN_004410d0`, calls SetField, releases handle. Used by emitters that do not hold the handle directly. |
| `0x00a5c150` | `CmeEventSignal_Subscribe` | **Not a subscribe.** `count(name)` on the event registry: returns whether a class name is registered and inserts nothing. See the correction. |
| `0x005783b0` | `CmeEventData_GetField` | Extract a named field from an event data object (receiving side / emitters copying fields between signals). |
| `0x00c79120` | `EmitNetOut_DebugMinigameInstance` | Canonical 154-line emitter exemplar — only non-stub in W1's V5 scope. |

Call sequence for an emit (Pattern A):

1. **Get the system singleton** — `CmeEventSignal_GetSystem()` returns the `CME::EventSignalSystem*` for this process.
2. **Look up the signal handle by name** — `CmeEventSignal_LookupByName(system, "Event_NetOut_<Name>")` returns the signal object for the named event.
3. **Populate fields** — call `CmeEventSignal_SetField(signal, "<key>", &value)` once per field on the signal's data object.
4. **Dispatch via vtable** — call the signal's primary virtual emit slot to run the bound subscribers.

`SGWNetworkManager` is the canonical NetOut subscriber; it converts the populated signal into a Mercury entity-method call through the universal RPC dispatcher at `0x00c6fc40` (see [`combat-wire-formats.md`](combat-wire-formats.md)).

## Correction (2026-09-28): the registry is an event factory, not a subscriber API

Disassembly of the QA `SGW.exe` and a headless-Ghidra decompile show that the "system" these names describe is an **event factory registry**, and that nothing at these addresses subscribes to anything.

- `0x0155f790` returns `0x01f11fc4`, a function-local static `std::map<std::string, factory>`. The first call builds it empty (constructor `0x00a5c1d0`) and registers its destructor with `atexit`. `CMERegistry__RegisterAllEventEmitHandlers` (`0x005c75d0`) fills it at startup, keyed by full event class name (`Event_Action_MouseClick` at `0x01840754`, `Event_Action_MouseLook`, …).
- `0x00a5c0f0` is `__thiscall(registry, const std::string& name) -> event*`, `ret 4`. It runs the map's `find` (`FUN_0158ea90`, which compares `std::string` keys: size at `+0x14`, capacity at `+0x18`, inline below 16), and on a hit calls the stored factory at `node+0x28` and returns the new event object. On a miss it returns 0.
- `0x00a5c150` is `__thiscall(registry, const std::string& name) -> bool`, `ret 4`: `find(name) != end()`. It inserts nothing.
- `Client_NetIn_EntityMethodDispatch` (`0x00c6f8f0`) calls `0x00a5c0f0` for every inbound entity method it routes, with the event name stored in the method's handler-map node, then fills the arguments and fires the event through the event's vtable slot 2. That makes `0x00a5c0f0` the positive half of the dispatch oracle whose negative half is the drop callee `0x01590f30`.
- Real native subscriptions construct a `MemberCallback` (for example the constructor at `0x00d34cb0`, called from `FUN_00d351d0` and `FUN_00d35260`) and pass it to `FUN_00a37790` or `FUN_00a374a0`. Those two are the subscribe candidates; neither is named or verified yet.

Consequence for the telemetry DLL: its CME subscriber install called `0x00a5c0f0` with a C string where the function takes a `std::string`, then called `0x00a5c150` as "subscribe". At best the lookup found nothing (the registry is still empty when the DLL boots) and nothing was subscribed; at worst the key compare read a garbage pointer out of the string bytes. The install was removed on 2026-09-28 and replaced with an inline hook on `0x00a5c0f0` (`client.cme.event`), which names every event the client creates through the registry. See [client-instrumentation-hookpoints.md](client-instrumentation-hookpoints.md) and `crates/client-telemetry/src/hooks/inline_hooks/cme_event_factory.rs`.

The claim below that Pattern A emitters "look up the signal handle by name" should be read as "create the event object by name".

## Pattern B emitters

A second structural pattern was confirmed by W-emit-A (session 3) for a subset of emitters, primarily in the lower binary half (`0x00400000–0x00b00000`) and in a dense 200+ function cluster at `0x00573d70–0x005aXXXX`:

**Pattern B vs Pattern A:**

| Aspect | Pattern A | Pattern B |
|--------|-----------|-----------|
| Signal resolution | Dynamic: `GetSystem + LookupByName` at emit time | Static: event type baked into vtable at construction |
| Object lifecycle | Caller reuses a system-registered handle | Caller allocates 12-byte object via `scalable_malloc(0xC)` |
| Base constructor | Not called | `NetworkEvent_Ctor` (`0x004412e0`) always called first |
| Vtable stamping | N/A (handle already typed) | Caller stamps `Event_NetOut_<Name>::vftable` after base ctor |
| Field population | `CmeEventSignal_SetField` (0x0043b850) | Same — `CmeEventSignal_SetField` |
| Dispatch | Signal vtable slot | `*pObj[+0x8]` (vtable slot 2 of the typed object) |

**Pattern B construction sequence:**
```
scalable_malloc(0xC)                        -- 12-byte signal object
NetworkEvent_Ctor(pObj)                     -- base init (0x004412e0)
*pObj = &Event_NetOut_<Name>::vftable       -- stamp typed vtable
CmeEventSignal_SetField(pObj, key, &val)   -- populate fields
(*pObj->vtable[2])(...)                     -- dispatch
```

Confirmed Pattern B emitters (W-emit-A + W-emit-B, session 3):
- `EmitNetOut_callForAid` (`0x00aea880`) — field: respawnerID
- `EmitNetOut_SetRingTransporterDestination` (`0x00aeab70`) — fields: aRegionId, aDestinationId
- 200+ typed ctors in `0x00573d70–0x005aXXXX` (one per `Event_NetOut_*` / `Event_UI_*` type)

The dense ctor cluster calls `NetworkEvent_Ctor` with uniform spacing (~0x30 bytes per function), covering the full `Event_NetOut_*` + `Event_UI_*` + `Event_SlashCmd_*` namespace.

## The class anatomy

### `TypedEmitInfo`

`TypedEmitInfo` is the per-event type-info object attached to every `Event_NetOut_*` / `Event_NetIn_*` signal. Vtable slot 0 (`TypedEmitInfo__vfunc_0`) is the **MSVC scalar destructor** (`~TypedEmitInfo()`), NOT a name-string accessor as previously assumed.

Body shape:

1. Call the per-event cleanup function (releases owned field data).
2. Conditionally `scalable_free(pThis)` if `bDeallocate & 1` — the standard MSVC scalar-destructor heap-cleanup contract.

**Score ceiling: `analyze_function_completeness` caps at ~78 for these.** The deduction is structural: `void* this` in MSVC `__thiscall` cannot be retyped via the Ghidra MCP API, so the analyzer can never close the residual gap. Accept this as a known limitation, not a worker error.

Confirmed across the entire NetIn TypedEmitInfo family (187 functions, W3) plus 17 NetOut TypedEmitInfo (W2 + W3). The 57 functions W1 applied trimmed V5 to in session 1 are part of this family and need a session-2 rescore with the destructor plate.

### `CallbackImpl`

`CallbackImpl` is the per-event subscriber-glue object. Vtable slot 2 (`CallbackImpl__vfunc_2`) is the **RTTI type-name accessor**: it returns the compile-time `TypeDescriptor*` for the bound type, NOT a name string. Confirmed across 17 CallbackImpl functions by W3.

Cluster ranges (mirrored in [`../address-map.md`](../address-map.md)):

| Address range | Cluster |
|---------------|---------|
| `0x00d43e30 – 0x00d44c80` | NetOut CallbackImpl__vfunc_2 RTTI type descriptor accessors (uniform 0x10-spacing) |
| `0x00e11cb0 – 0x00e11cd0` | NetIn store CallbackImpl cluster (`onStoreOpen` / `onStoreUpdate` / `onStoreClose`) |
| `0x00e219b0 – 0x00e21a10` | NetIn inventory CallbackImpl cluster (`onContainerInfo` through `CashChanged`) |
| `0x00e24810` | LootDisplay CallbackImpl — isolated from inventory cluster (~0x2E00 gap), suggests separate compile unit |

## Architectural anomalies (open)

Two subsystems break the otherwise-uniform pattern and warrant follow-up:

- **Black Market** (`BMCreateAuction`, `BMCancelAuction`, `BMPlaceBid`, `BMSearch`) has `TypedEmitInfo` entries but **no paired `CallbackImpl__vfunc_2`**. Suggests a different callback registration mechanism — possibly direct function pointers instead of the typed CME signal pattern. See [`black-market-wire-formats.md`](black-market-wire-formats.md).
- **`GiveInventory`** has the same anomaly — `TypedEmitInfo` present, `CallbackImpl` absent. See [`inventory-wire-formats.md`](inventory-wire-formats.md).

Both deserve a targeted decompile to find the alternate registration path. Until that's done, do not assume the universal emit pipeline above applies to either subsystem.

A third (benign) anomaly: **Trade has duplicate `TypedEmitInfo` instances** at `0x00d2ad10`/`0x00d2aa70` and `0x00e266c0`/`0x00e26700` for the same event names (`TradeRequestCancel`, `TradeLockState`). W2 confirmed these are legitimately separate signal objects for different subsystems handling the same wire event — not a duplication bug. See [`trade-wire-formats.md`](trade-wire-formats.md).

## CmeMemberCallback struct layout

The `CmeMemberCallback` struct (created in Ghidra category root, session 3) represents
the 12-byte heap object allocated for every bound subscriber registration. It is the
`this` argument received by `CmeEventSignal_InvokeMemberCallback` (vfunc_5).

```c
struct CmeMemberCallback {         // size: 0x0C (12 bytes)
    void* pVtable;     // +0x00  vtable ptr — one of 10 MemberCallback vtable arrays
    void* pSubscriber; // +0x04  subscriber object — loaded into ECX before dispatch
    void* pMethodPtr;  // +0x08  bound method pointer — the concrete handler body
};
```

Ghidra struct: `CmeMemberCallback` (12 bytes, created W-cleanup session 3, 2026-05-13).
Evidence: `0x00e04583: MOV ECX, [EAX+0x4]` (subscriber load), `0x00e0457f: MOV EDX, [EAX+0x8]` (method ptr load).

## Invoke dispatch

`0x00e04570` — `CmeEventSignal_InvokeMemberCallback` — is the shared vfunc_5 body
loaded into slot 5 of every `MemberCallback` vtable. It is the last leg of the emit
pipeline: the point at which the stored method pointer is actually called.

### What it does

The function is 11 instructions long (`0x00e04570` – `0x00e04588`, `RET 0x8`).
It performs a single indirect dispatch through the method pointer stored at `this+0x8`
on the `MemberCallback` object. Before the call it loads the subscriber object pointer
from `this+0x4` into ECX so the handler receives the correct `__thiscall` receiver.

`MemberCallback` object layout (ECX on entry) — see `CmeMemberCallback` struct above:

| Offset | Contents |
|--------|----------|
| `+0x0` | vtable ptr — points to one of the 10 shared MemberCallback vtable arrays |
| `+0x4` | subscriber* — the bound object whose method will be called (loaded into ECX before dispatch) |
| `+0x8` | method ptr — function pointer to the concrete handler body |

Argument order reversal: the bound handler receives `(pSubjectArg, pCallbackThis)` —
the subject/entity pointer becomes arg1, the event-data copy becomes arg2. This is
the standard MemberCallback calling convention for `GameEntitySubject` event handlers.

### Shared across 10 vtable instantiations

xref_count=12, but 2 of the 12 are non-vtable DATA references (coincidental alignment
in a data block adjacent to strings "commit transaction" and "GameEntityFactory.cpp").
The 10 true vtable slots span the following subscriber class / event pairings,
confirmed via RTTI `MemberCallback__vfunc_3` accessors:

| Vtable address | Subscriber class | Event |
|----------------|-----------------|-------|
| `0x019ba728` | `VSequenceManager` | `AppearanceJob_Completed` |
| `0x019ba744` | `VSequenceManager` | `AppearanceJob_Completed` (second vtable instance) |
| `0x019bbd74` | `VCharacterCreation` | `Event_Cache_ElementReady` |
| `0x019d5e68` | `VGameProxyPlayer` | `AppearanceJob_Completed` |
| `0x019d687c` | `VGameBeing` | `NetIn_TimerUpdate` |
| `0x019d6898` | `VGameBeing` | `Entity_Destroyed` |
| `0x019d68b4` | `VGameBeing` | `Entity_PawnGiven` |
| `0x019de66c` | `VCoverInfo` | `Entity_ProxyPlayerCellCreated` |
| `0x019e7e38` | `VGameAppearanceManager` | `AppearanceJob_Completed` |
| `0x019e7e54` | `VGameAppearanceManager` | `AppearanceJob_Completed` (second vtable instance) |

The shared dispatch body exists because the underlying dispatch pattern is identical
for all GameEntitySubject-derived MemberCallback types — only the bound method pointer
at `+0x8` differs per subscriber registration.

### Key instruction

```
00e0457f: MOV EDX, dword ptr [EAX+0x8]   ; load bound method ptr from MemberCallback+0x8
00e04583: MOV ECX, dword ptr [EAX+0x4]   ; load subscriber object into ECX (__thiscall this for handler)
00e04586: CALL EDX                         ; DISPATCH — handler body varies per subscriber class
```

### Position in the pipeline

```
CmeEventSignal_GetSystem        (0x0155f790)  get singleton
  -> CmeEventSignal_LookupByName (0x00a5c0f0)  resolve signal handle
    -> CmeEventSignal_SetField    (0x0043b850)  populate fields
      -> vtable dispatch (signal vtable)         fire the signal
        -> CmeEventSignal_InvokeMemberCallback (0x00e04570)  [THIS FUNCTION]
          -> bound handler body (varies per subscriber class)
```

## `FreeCallback` and finding NetIn subscribers from RTTI

`MemberCallback` is not the only subscriber shape. `CME::EventSignal::FreeCallback<Subject, Context, Fn>` binds a plain function instead of a method: `Fn` is a `__cdecl` function pointer, and the bound `Context` value is passed to it as the last argument. The two shapes show up side by side in the client's RTTI type-name strings (`.?AV?$MemberCallback@...` and `.?AV?$FreeCallback@...`), which is the quickest static way to find native, non-Lua subscribers to a `NetIn` event: search the defined strings for the event name and read the template arguments.

Two subscribers found this way (2026-09-27, headless Ghidra, [tools/re/ghidra-headless/](../../../tools/re/ghidra-headless/README.md)):

| Event | RTTI type-name string | Subscriber (demangled by hand) | Finding |
|---|---|---|---|
| `Event_NetIn_TradeResults` | `0x01e5e800` (TypeDescriptor `0x01e5e7f8`) | `MemberCallback<NoSubject, Trade, void (__thiscall Trade::*)(Event_NetIn_TradeResults const*, void*)>` | [trade-result-client-handling.md](trade-result-client-handling.md#a-native-trade-subscriber-exists-rtti-2026-09-27) |
| `Event_NetIn_onErrorCode` | `0x01e20da8` | `FreeCallback<NoSubject, Communicator*, void (__cdecl*)(Event_NetIn_onErrorCode const*, void*, Communicator*)>` | [ability-trainer-ui.md §2](ability-trainer-ui.md#a-native-subscriber-exists-rtti-2026-09-27) |

Two limits on the method. An RTTI string proves the callback type is compiled in, and so near-certainly subscribed, but it does not give you the handler's address: that is the pointer stored in the callback object (`+0x8` for a `MemberCallback`), which you read from live memory or reach through the vtable once a full RTTI analysis resolves it. And in this binary a raw pointer scan for the Complete Object Locator is **not** a reliable way to that vtable: the `Trade` scan led to `0x019d8928`, which belongs to an unrelated `MemberCallback` specialization. Treat such hits as leads to check, not results.

## Naming convention correction (Session 3)

Sessions 1 and 2 applied annotation scripts that named `MemberCallback` vtable slot 2 implementations as
`OnEvent_<Event>__<Subscriber>`. This prefix was incorrect: the `On` prefix implies an event handler, but
slot 2 is the RTTI type-name accessor — it returns a `TypeDescriptor*` for the subscriber class at
compile time and has no runtime event-handling logic.

Session 3 (W-rename, 2026-05-13) corrected all 489 affected functions, replacing the `OnEvent_` prefix
with `MemberCallbackRtti_`. The corrected naming schema:

```
MemberCallbackRtti_<EventName>__<SubscriberClass>
```

Examples:
- `MemberCallbackRtti_NetOut_RepairItem__SGWNetworkManager` (`0x00d46ce0`)
- `MemberCallbackRtti_Net_Connected__Detail_CookedKismetEventSetData` (`0x00426860`)
- `MemberCallbackRtti_UI_WorldChanged__SGWScriptedWindow` (`0x00cc83a0`)

The 20 functions at `0x00d46ce0–0x00d47660` had mangled C++ template names rather than the scripted
`OnEvent_` prefix; their event names were extracted from the template argument and they received the
`MemberCallbackRtti_NetOut_*__SGWNetworkManager` pattern directly.

NamingConventions.java issues advisory warnings for this convention (underscore-containing, non-PascalCase,
unrecognized verb prefix) but the validator is non-blocking: all 489 renames applied without rejection.
This is a known and accepted deviation — the compound template-derived names do not fit the single-verb
PascalCase mold; clarity of provenance outweighs mechanical PascalCase compliance here.

Slot-to-role table for `MemberCallback<E, S>` vtable:

| Slot | Name pattern | Role |
|------|-------------|------|
| 0 | (destructor) | MSVC scalar destructor |
| 2 | `MemberCallbackRtti_<E>__<S>` | RTTI accessor — returns `TypeDescriptor*` |
| 5 | `CmeEventSignal_InvokeMemberCallback` (`0x00e04570`) | Actual dispatch — calls bound method at `this+0x8` |

## Cross-references

- [`../address-map.md`](../address-map.md) — canonical address registry; CME pipeline + CallbackImpl cluster tables live there.
- [`combat-wire-formats.md`](combat-wire-formats.md) — universal RPC dispatcher (`0x00c6fc40`) that turns populated signals into Mercury entity-method calls.
- [`inventory-wire-formats.md`](inventory-wire-formats.md) — `GiveInventory` anomaly context.
- [`contact-list-wire-formats.md`](contact-list-wire-formats.md) — cyclic-shift name-misassignment example in a TypedEmitInfo block.
- [`black-market-wire-formats.md`](black-market-wire-formats.md) — Black Market anomaly context.
- [`trade-result-client-handling.md`](trade-result-client-handling.md) and [`ability-trainer-ui.md`](ability-trainer-ui.md) — the native `TradeResults` and `onErrorCode` subscribers found from RTTI.
- [`../../../tools/re/ghidra-headless/README.md`](../../../tools/re/ghidra-headless/README.md) — the headless-Ghidra probe script used for RTTI string searches.
- [`../../engine/cme-framework.md`](../../engine/cme-framework.md) — CME framework overview (PropertyNode, EventSignal, Atrea scripts).
- [`../v5-campaign/CAMPAIGN_STATUS.md`](../v5-campaign/CAMPAIGN_STATUS.md) — V5 campaign aggregator; per-worker reports for W1/W2/W3.
