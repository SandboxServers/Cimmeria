---
title: "Address Map — Key Locations in SGW.exe"
type: reference
audience: contributors doing RE
last_updated: 2026-05-27
---

# Address Map — Key Locations in SGW.exe

> **Last updated**: 2026-05-13
> **Binary**: SGW.exe (32-bit x86 PE, MSVC 8.0 / VC80)
> **Image base**: `0x00400000`

---

## Overview

Key virtual addresses, vtables, global variables, and important functions discovered during reverse engineering. All addresses are virtual addresses as loaded by Ghidra.

## Important Globals

| Address | Name | Type | Notes |
|---------|------|------|-------|
| `0x01ef244c` | `g_EntityManager` | `EntityManager*` | Singleton — set in `BW_client_entity_manager` constructor |
| `0x01ea5778` | `GMalloc` | `FMalloc*` | UE3 global allocator — points to FMallocCME instance |
| `0x01ee1254` | `GEngine` | `UEngine*` | UE3 engine singleton (RVA `0x01AE1254`) |
| `0x01ef134c` | `GEditor` | `UEditorEngine*` | Set when GIsEditor=1 (RVA `0x01AF134C`) |
| `0x01ef2e74` | `GApp` | `void*` | Application pointer (RVA `0x01AF2E74`) |
| `0x01eadbc0` | `FName::GNames` | `TArray<FNameEntry*>*` | Global name table (RVA `0x01ACADE0`; add image base `+0x400000` for note below) |
| `0x01edc69c` | `UObject::GObjObjects` | `TArray<UObject*>*` | Global object array (RVA `0x01ADC69C`) |
| `0x01ead7ac` | `GIsEditor` | `UBOOL` | Editor mode flag |
| `0x01ead7b0` | `GIsUCC` | `UBOOL` | Commandlet mode flag |
| `0x01ead7bc` | `GIsClient` | `UBOOL` | Client mode flag |
| `0x01ead7c0` | `GIsServer` | `UBOOL` | Server mode flag |
| `0x01eb0830` | `GIsGame` | `UBOOL` | Game mode flag (CME addition, not in stock UE3 2004) |
| `0x01ee435c` | BigWorld package init guard | `DWORD` | Bit flags: bit 0=package created, bit 1=callback installed |
| TODO | `g_ConnectionModel` | `ServerConnection*` | Main server connection object |
| TODO | `g_ScriptManager` | `ScriptManager*` | Python script engine |

## Key Vtables

Vtables identified via RTTI (script 01) or manual analysis.

| Address | Class | vfunc Count | Notes |
|---------|-------|-------------|-------|
| See `BW_client_entity_manager` | `EntityManager` | — | Dual vtable: ServerMessageHandler + FCallbackEventDevice |
| `0x018014f4` | `UBWNetDriver` | 3 ifaces | Primary + UObject iface (0x018014ec) + FNetObjNotify (0x018014d8) |
| `0x0180167c` | `UBWConnection` | 2 ifaces | Primary + UObject iface (0x01801670) |
| `0x01895e3c` | `ABigWorldEntity` | — | Extends AActor (0x0183c40c), size 0x1C4 |
| `0x018caea4` | `UBigWorldInfo` | — | Extends UObject, size 0x44 |
| TODO | `ServerConnection` | — | Mercury connection to server |
| TODO | `Entity` | — | Base entity class (BigWorld) |
| TODO | `CME::EventSignal` | — | CME event dispatch (client-side UI bus only) |

## Core Architecture Functions — Phase 2 Decompiled

### Universal RPC Dispatcher

| Address | Function | Notes |
|---------|----------|-------|
| `0x00c6fc40` | Universal RPC dispatcher | ALL outgoing entity method calls route here |
| `0x00dd6a60` | `ServerConnection_startEntityMessage` | Writes cell method header: `methodID \| 0x80` |
| `0x00dd6980` | `ServerConnection_startProxyMessage` | Writes base method header: `methodID \| 0xC0` |

### Entity Creation

| Address | Function | Notes |
|---------|----------|-------|
| `0x00dddca0` | `ServerConnection_createBasePlayer` | 4B entityID + 2B typeID + property stream |
| `0x00dda2e0` | `ServerConnection_createCellPlayer` | 4B skip + 4B spaceID + 12B Vec3 pos + property stream |

### Entity Manager (entity_manager.cpp — CME modified BW 1.9.1)

| Address | Function | Notes |
|---------|----------|-------|
| `0x00dd3330` | `EntityManager::EntityManager` | Constructor — dual vtable (ServerMessageHandler + FCallbackEventDevice), singleton at `0x01ef244c` |
| `0x00dd2270` | `EntityManager::onEntityCreate` | Line 480/529: creates BW entity, scales position ×100, calls RadiansToRotator |
| `0x00dd1650` | `EntityManager::onEntityMoveWithError` | Scales pos/vel ×100, converts rotation, handles sentinel "use current" values, delegates to ApplyTransform |
| `0x00dd09e0` | `EntityManager::createEntity` | Line 2619: allocates entity, calls GameEntityBase::Init (0x00e685e0) |
| `0x00dd0b00` | `EntityManager::PostLoadMap` | Callback 0x32: fires Event_Level_PostLoad |
| `0x00dd3150` | `EntityManager::LevelRemovedFromWorld` | Callback 0x30: fires Event_Map_Unloaded |
| `0x00dd0d00` | `BW_client_entity_manager_1` area | Entity method dispatch |
| `0x00dd2900` | `BW_client_entity_manager_5` | Entity leave AoI — decrements refcount, cleanup |
| `0x00dd27f0` | `BW_client_entity_manager_4` area | Entity enter AoI — increments refcount, enterWorld |
| `0x00dd1b10` | `BW_client_entity_manager_6` | Entity position/movement update |
| `0x00dd1d00` | `EntityManager::enterWorld` | Entity enter world callback |

### EntityDescription Parsing

| Address | Function | Notes |
|---------|----------|-------|
| `0x01593cd0` | `EntityDescription_parse` | Opens .def file, handles Parent recursion |
| `0x01593600` | `EntityDescription__unknown_01593600` | Parse dispatch: Implements→Properties→Methods |
| `0x015924a0` | `EntityDescription_parseProperties` | Property ID assignment (sequential, excludes EDITOR_ONLY) |
| `0x01594f60` | `MethodDescription_parse` | Method signature parsing (Args, ArgNames, Exposed) |
| `0x015974a0` | `DataDescription_parse_2` | Property type + flags + default value parsing |
| `0x015959c0` | `DataDescription_parse_1` | Property flag string → bitmask conversion |
| `0x01593420` | `EntityDescription_ParseClientMethods` | Extracts 'ClientMethods' child, delegates to ParseMethodsSection |
| `0x015934c0` | `EntityDescription_ParseCellMethods` | Extracts 'CellMethods' child, delegates to ParseMethodsSection |
| `0x01593560` | `EntityDescription_ParseBaseMethods` | Extracts 'BaseMethods' child, delegates to ParseMethodsSection |

### EntityDescription Vector Helpers (W-entity-desc-A, 2026-05-13)

| Address | Function | Notes |
|---------|----------|-------|
| `0x0158e060` | `DataDescriptionParseVec_GetSize` | `(end-begin)/0x110`; fields at `this+4/+8` |
| `0x0158e080` | `MethodDescriptionVec_GetSize` | `(end-begin)/0x50`; fields at `this+4/+8` |
| `0x0158e0a0` | `DataDescriptionParseVec_AllocN` | `scalable_malloc(n*0x110)` with overflow guard |
| `0x0158e110` | `MethodDescriptionVec_AllocN` | `scalable_malloc(n*0x50)` with overflow guard |
| `0x0158e180` | `DataDescriptionParseVec_GetSizeAlt` | Size using `this+0x10/+0x14` offsets |
| `0x0158e1a0` | `DataDescriptionParseVec_GetAt` | Bounds-checked `begin + idx*0x110` |
| `0x0158e1e0` | `MethodDescription_CopyCtorSEH` | SEH-guarded MethodDescription copy-construct |
| `0x0158e230` | `DataDescription_PartialInitSEH` | SEH-guarded DataDescription_PartialInit (0x40-byte form) |
| `0x0158e280` | `MethodDescriptionVec_CopyRangeToOffset` | Copies range to `src+offset` using CopyAssign |
| `0x0158e310` | `DataDescriptionParseVec_ForEachFindMax` | Functor-per-element with running max |
| `0x0158e460` | `MethodDescriptionVec_ReserveN` | Init with N capacity (0x50-byte, max 0x3333333) |
| `0x0158e4b0` | `DataDescriptionVec_ReserveN` | Init with N capacity (0x40-byte, max 0x3ffffff) |
| `0x0158e500` | `DataDescriptionVec_UninitCopyRange` | SEH range copy, 0x40-byte stride |
| `0x0158e5c0` | `MethodDescriptionVec_UninitCopyRange` | SEH range copy, 0x50-byte stride |
| `0x0158e650` | `EntityDescriptionMap_LowerBound` | MSVC xtree lower_bound on method ID map |
| `0x0158e710` | `EntityDescription_FindMethodIdByName` | Returns uint16 method ID; 0xffff=not-found; called by RPC dispatcher |
| `0x0158e780` | `EntityDescription_FindAndWritePropertyByName` | Scans DataDescVec by name, calls WriteClientData on match |
| `0x0158e840` | `EntityDescriptionMap_InsertOrFind` | MSVC xtree insert-or-find on method ID map |
| `0x0158ea00` | `MethodDescriptionVec_UninitCopyRangeThunk` | 5-arg thunk for MethodDescriptionVec_UninitCopyRange |
| `0x0158ea30` | `DataDescriptionVec_UninitCopyRangeThunk` | 5-arg thunk for DataDescriptionVec_UninitCopyRange |
| `0x0158ea60` | `DataDescriptionVec_UninitCopyRangeThunk2` | 3-arg thunk for DataDescriptionVec_UninitCopyRange |

### Property Change

| Address | Function | Notes |
|---------|----------|-------|
| `0x015652d0` | `FNetworkPropertyChange__vfunc_0` | Writes property change to stream (4B header + values) |

### Event Signal Registration (CME — client-side only)

| Address | Event Name | Notes |
|---------|------------|-------|
| `0x00cb7d90` | `register_NetOut_UseAbility` | Returns string "Event_NetOut_UseAbility" |
| `0x00d771e0` | `register_NetIn_onEffectResults` | Returns string "Event_NetIn_onEffectResults" |
| `0x00d7f520` | `register_NetIn_TimerUpdate` | Returns string "Event_NetIn_TimerUpdate" |
| `0x00d86620` | `register_NetIn_onStatUpdate` | Returns string "Event_NetIn_onStatUpdate" |
| `0x00d86c10` | `register_NetIn_onStatBaseUpdate` | Returns string "Event_NetIn_onStatBaseUpdate" |
| `0x00d7d300` | `register_NetIn_onContainerInfo` | Returns string "Event_NetIn_onContainerInfo" |

**Note**: Event registration functions simply return a name string. They do NOT contain serialization logic. The actual network serialization is handled by the universal RPC dispatcher at `0x00c6fc40`. See `docs/reverse-engineering/findings/combat-wire-formats.md` for details.

## Weapon / Ammo Subsystem (W-weap session, 2026-05-13)

See [`findings/weapon-ammo-pipeline.md`](findings/weapon-ammo-pipeline.md) for full analysis. Key finding: issue #168 — `handle_reload` sent propId=7 (AccessLevel) instead of propId=3 (AmmoTypeId).

| Address | Function | Notes |
|---------|----------|-------|
| `0x00e078a0` | `EmitNetOut_RequestReload` | Direct emitter; Pattern B (12-byte scalable_malloc). Called by `FUN_00ad7880` after `GamePlayer` RTTI cast. |
| `0x00ad7880` | `FUN_00ad7880` | Gate: RTTI-casts local entity to `GamePlayer`, then calls `EmitNetOut_RequestReload(reloadType)`. |
| `0x00c889a0` | `EmitNetOut_RequestReload_TextCmd` | SGWTextCommandManager handler variant; reads "reloadType" from CME event; source: `SGWTextCommandManager.cpp:0xB4B`. |
| `0x00c8a5c0` | `EmitNetOut_RequestAmmoChange_TextCmd` | SGWTextCommandManager handler for ammo change; reads "ammoType", sets ItemId+AmmoType; source: `SGWTextCommandManager.cpp:~0xC09`. |
| `0x00cbcda0` | `NetworkEvent_RequestReload_Ctor` | 12-byte NetworkEvent ctor for `Event_NetOut_RequestReload` (Pattern B). Sets `aReloadType` field. |
| `0x00caf850` | `CmeEventSignal_Emit_Subscribe` | CME emit dispatcher called by both RequestReload emitters. Wraps into a 0x18-byte container via `FUN_00c92b20`. |
| `0x00cbce00` | `register_NetOut_RequestReload` | Returns string `"Event_NetOut_RequestReload"`. Stub only. |
| `0x019bfe54` | String: `"onEntityProperty"` | Data xref from `RegisterBulkNetOutSignals` at `0x00db9f12`. |
| `0x019b409c` | String: `"Event_NetOut_RequestReload"` | Data xref from `RegisterBulkNetOutSignals` at `0x00db882c`. |
| `0x019b430c` | String: `"Event_NetOut_RequestAmmoChange"` | Companion to RequestReload. |
| `0x019aed18` | String: `"aReloadType"` | Field name in `Event_NetOut_RequestReload` event object. |
| `0x019af444` | String: `"ammoType"` | Input field read from CME event in RequestAmmoChange handler. |
| `0x019af4ac` | String: `"AmmoType"` | SetField output key in RequestAmmoChange NetworkEvent. |

### EEntityPropertyType constants (from enumerations.xml — no binary address needed)

| propId | Name | Used in |
|--------|------|---------|
| 1 | `GENERICPROPERTY_TrainingPoints` | world entry, progression |
| 2 | `GENERICPROPERTY_AppliedSciencePoints` | world entry, progression |
| **3** | **`GENERICPROPERTY_AmmoTypeId`** | **handle_reload fix (#168), requestAmmoChange, bandolier slot swap, world entry** |
| 4 | `GENERICPROPERTY_PvPFlag` | world entry |
| 5 | `GENERICPROPERTY_PetOwnerId` | AoI enter |
| 6 | `GENERICPROPERTY_MobAggression` | spawn |
| 7 | `GENERICPROPERTY_AccessLevel` | world entry |
| 8 | `GENERICPROPERTY_Gender` | world entry |
| 9 | `GENERICPROPERTY_DatabaseId` | AoI enter (speaker ID) |

## CME EventSignal pipeline (sessions 1–3)

Surfaced by W1 + W2 + W3 of the V5 Documentation Campaign session 1 (2026-05-12); extended in sessions 2–3. See [`findings/cme-event-signal.md`](findings/cme-event-signal.md) for the consolidated finding.

| Address | Name | Role |
|---------|------|------|
| `0x00c79120` | `EmitNetOut_DebugMinigameInstance` | Canonical CME emitter (only non-stub in W1's scope). 154 lines — shows the full `GetSystem → LookupByName → SetField × N → vtable dispatch` pattern. |
| `0x005783b0` | `CmeEventData_GetField` | Extract named field from event data object. |
| `0x0155f790` | `CmeEventSignal_GetSystem` | Singleton accessor for CME EventSignal system. |
| `0x00a5c0f0` | `CmeEventSignal_LookupByName` | Resolve signal handle by name string. |
| `0x0043b850` | `CmeEventSignal_SetField` | Set key/value field on a signal object. |
| `0x00cb1f00` | `CmeEventSignal_SetFieldHelper` | SetField wrapper: acquires CME handle via FUN_004410d0, calls SetField, releases handle. Used where callers don't hold the handle directly. |
| `0x00a5c150` | `CmeEventSignal_Subscribe` | Subscriber insertion: registers a callback object into a signal's subscriber set. Returns true if newly inserted. Distinct from LookupByName (0x00a5c0f0) by 0x60 bytes and different call contract. |
| `0x00e04570` | `CmeEventSignal_InvokeMemberCallback` | Shared vfunc_5 body for all MemberCallback vtables. Loads method ptr from `this+0x8`, subscriber from `this+0x4` (ECX), dispatches. 10 vtable instantiations. |
| `0x004412e0` | `NetworkEvent_Ctor` | Base constructor for Pattern B NetworkEvent objects (12-byte, `scalable_malloc(0xC)`). Called by 200+ typed ctors before vtable stamp. |
| `0x00db3390` | `RegisterBulkNetOutSignals` | Bulk startup registration sweep: calls LookupByName for 40+ Event_NetOut_* signals. Previously misnamed `register_NetOut_onStrikeTeamResponse` by annotation script 04. |

## CallbackImpl RTTI accessor clusters (session 1)

`CallbackImpl__vfunc_2` is the RTTI type-name accessor (returns a compile-time `TypeDescriptor*`, NOT a name string). Cluster ranges identified by W2 + W3.

| Address range | Cluster |
|---------------|---------|
| `0x00d43e30 – 0x00d44c80` | NetOut CallbackImpl__vfunc_2 RTTI type descriptor accessors (uniform 0x10-spacing) |
| `0x00e11cb0 – 0x00e11cd0` | NetIn store CallbackImpl cluster (`onStoreOpen` / `onStoreUpdate` / `onStoreClose`) |
| `0x00e219b0 – 0x00e21a10` | NetIn inventory CallbackImpl cluster (`onContainerInfo` through `CashChanged`) |
| `0x00e24810` | LootDisplay CallbackImpl — isolated from inventory cluster (~0x2E00 gap), suggests separate compile unit |

See [`findings/cme-event-signal.md`](findings/cme-event-signal.md) for the architectural anomalies (Black Market + GiveInventory lack this pattern). All three anomalies are now resolved — see [`findings/architectural-anomalies.md`](findings/architectural-anomalies.md).

## Architectural anomalies (W-anom session 5)

All three anomalies confirmed resolved by live decompile 2026-05-13. See [`findings/architectural-anomalies.md`](findings/architectural-anomalies.md) for the full findings.

### Black Market Pattern B emitters

| Address | Name | Role |
|---------|------|------|
| `0x00e59970` | `EmitNetOut_BMCreateAuction` | Pattern B emitter — 4 fields, guarded by inventory ownership check via `FUN_00e1c450` |
| `0x00e59c70` | `EmitNetOut_BMCancelAuction` | Pattern B emitter — 1 field (sequenceId), no guard |
| `0x00e59da0` | `EmitNetOut_BMPlaceBid` | Pattern B emitter — 2 fields, guarded by bid-amount check at `entity+0x60` |
| `0x00e59f70` | `EmitNetOut_BMSearch` | Pattern B emitter — 11 fields (largest BM emitter) |
| `0x00e5c1a0` | `Event_NetOut_BMCreateAuction::ctor` | Pattern B ctor: `NetworkEvent_Ctor` + stamp `Event_NetOut_BMCreateAuction::vftable` |
| `0x00e5c440` | `Event_NetOut_BMCancelAuction::ctor` | Pattern B ctor for BMCancelAuction |
| `0x00e5c6e0` | `Event_NetOut_BMPlaceBid::ctor` | Pattern B ctor for BMPlaceBid |
| `0x00e5c980` | `Event_NetOut_BMSearch::ctor` | Pattern B ctor for BMSearch |
| `0x0054c900` | `CME_GetOrCreateSystem` (inferred) | Lazy-init CME system singleton: allocates 68-byte object if `DAT_01ee2678 == 0`; used by all BM Pattern B emitters as dispatch target |
| `0x0054c870` | `CME_InitSystem` (inferred) | Inner init: stores singleton pointer at `DAT_01ee2678`, calls `FUN_00a37710` |
| `0x00a37710` | `CME_System_Ctor` (inferred) | Inits CME system object: sets `+0x4/+0x8/+0xC` to 0, calls `FUN_00570310` for `+0x14`, `FUN_00a38d50` for internal list at `+0x1c` |

### GiveInventory anomaly

| Address | Name | Role |
|---------|------|------|
| `0x00d97750` | `register_NetOut_GiveInventory` | Returns string `"Event_NetOut_GiveInventory"` — vtable name accessor, zero callers; signal is server-side only from client perspective |
| `0x00d97830` | `CME_EventSignal_VEvent_NetOut_GiveInventory___TypedEmitInfo__vfunc_0` | MSVC scalar destructor for GiveInventory TypedEmitInfo; no client subscriber ever registered |
| `0x00c964d0` | `CME_EventSignal_VEvent_SlashCmd_GiveInventory___CallbackImpl__vfunc_2` | RTTI accessor for the **SlashCmd** variant — the active path; bound to `SGWTextCommandMgr` |
| `0x00c9a2a0` | `MemberCallbackRtti_SlashCmd_GiveInventory__SGWTextCommandMgr` | RTTI accessor confirming `SGWTextCommandMgr` is the subscriber for the slash-command form |

### SGWHomeless class

| Address | Name | Role |
|---------|------|------|
| `DAT_01ef2380` | `g_SGWHomeless` (inferred) | Static singleton object — 0x70 bytes (`_eh_vector_constructor_iterator_` spans 2 elements of 0x38 each); owns `+0x70` self-pointer |
| `DAT_01ef23f8` | `g_SGWHomeless_initFlag` (inferred) | Init-once guard (bit 0 set after first call to `FUN_00d3d440`) |
| `0x00d3d440` | `SGWHomeless_GetInstance` (inferred) | Static-init pattern: checks bit 0 of `DAT_01ef23f8`, calls `FUN_00d3d270`, registers atexit via `FUN_012375cb` |
| `0x00d3efb0` | `SGWHomeless_RegisterSubscriptions` (inferred) | Registers 22 `Editor_*` subscriptions for `SGWHomeless`; calls `thunk_FUN_0054c900` + `FUN_00d41dXX` per event; ends with `FUN_0057b800("editor")` mode registration |
| `0x00d3e060` | `SGWHomeless_Handle_Editor_Close` (inferred) | Calls `CloseEditorViewport` string → UE3 viewport vtable `+0x10c` |
| `0x00d3ed10` | `SGWHomeless_Handle_Editor_ViewWireframe` (inferred) | `ShellExecuteW(L"http://www.stargateworlds.com/")` — dev placeholder |
| `0x00d3ee60` | `SGWHomeless_Handle_Editor_ShadowStats` (inferred) | `ShellExecuteW(L"http://beta.stargateworlds.com/")` — dev placeholder |
| `0x00d40740` – `0x00d415c0` | SGWHomeless RTTI accessor cluster | 30 `MemberCallbackRtti_` functions (0x80 spacing), each returning TypeDescriptor* for `MemberCallback<NoSubject, SGWHomeless, handler, Event_*>` |
| `0x00d40ad0` | `MemberCallback<SGWHomeless, Editor_Close>::ctor` | Stamps typed vtable for `Event_Editor_Close` / SGWHomeless binding; confirms class name `class_SGWHomeless` |
| `0x00a37790` | `CmeEventSignal_Subscribe_impl` (inferred) | Inner subscribe: calls vtable slot 2 (`(*param_2 + 8)`) for RTTI lookup, then `FUN_00a39170`/`FUN_00a38950` to insert into signal's subscriber set |

## Undocumented client→server telemetry (session 1)

Two client→server telemetry pushes surfaced by W0 that are not in any current protocol doc. Cimmeria server should handle gracefully (no-op or log).

| Address | Name | Role |
|---------|------|------|
| `0x00d9cc40` | `SystemOptions` | Client sends hardware/performance info. |
| `0x00d9cee0` | `PerfStats` | Client sends FPS/latency metrics. |

## FMallocCME — Custom Memory Allocator

CME's 32-bit adaptation of Epic's FMallocTBB (64-bit only in stock UE3).
Wraps Intel TBB `scalable_malloc` with 24-byte overhead for manual 16-byte alignment.
Hard-asserts on OOM (`check(Ptr)` at FMallocCME.h:37) instead of returning NULL.

| Address | Function | Notes |
|---------|----------|-------|
| `0x004198f0` | `FMallocCME::Malloc` | `scalable_malloc(size+0x18)` + 16-byte align, `check(Ptr)` on OOM |
| `0x004198b0` | `FMallocCME::Free` | Reads raw ptr from alignment header → `scalable_free(raw)` |
| `0x00419810` | `FMallocCME::Realloc` | Does `Malloc + memcpy + Free` (doesn't use `scalable_realloc`) |
| `0x00416660` | Allocator bootstrap | Allocates 4B via CRT malloc, stamps FMallocCME vtable, skips thread-safe proxy (TBB is thread-safe) |
| `0x017f8e8c` | FMallocCME vtable | 20 slots: 3 real (Malloc/Free/Realloc), Init returns TRUE, rest stubs |
| `0x00419950` | `FMallocThreadSafeProxy` | Wraps any FMalloc with CRITICAL_SECTION (0x24 bytes, vtable 0x017F8F7C). Skipped for TBB. |
| `0x00419923` | OOM assert location | `check(Ptr)` — patched by Atrea MallocOOMSoftFail to return NULL |

### Allocation Layout (24-byte overhead per alloc)
```
scalable_malloc(size + 0x18) returns [raw_ptr]
                                      |
[  ...padding...  ][orig_size][raw_ptr][  aligned user data  ]
                    at -0x8    at -0x4   <-- returned to caller
                                         (16-byte aligned via AND 0xFFFFFFF0)
```

### TBB Import Thunks
| Address | Function | IAT Slot |
|---------|----------|----------|
| `0x00457e00` | `scalable_malloc` thunk | `JMP [0x017f0394]` (tbbmalloc.dll) |
| `0x00457dfa` | `scalable_free` thunk | `JMP [0x017f0390]` (tbbmalloc.dll) |
| `0x0150c7ec` | `scalable_realloc` thunk | `JMP [0x017f0398]` (tbbmalloc.dll) |
| `0x004162f0` | `scalable_free` trampoline | `JMP 0x00457dfa` (redirect to thunk) |

## World Entry Pipeline

Recovered in session 4b-world-entry (2026-05-13). Full findings in
`docs/reverse-engineering/findings/world-entry-pipeline.md`.

### Message Handlers (server → client)

| Address | Function | Wire Message | Notes |
|---------|----------|--------------|-------|
| `0x00dddca0` | `ServerConnection_CreateBasePlayer` | CREATE_BASE_PLAYER (0x05) | reads entityId u32 + typeId u16 |
| `0x00dda2e0` | `ServerConnection_CreateCellPlayer` | CREATE_CELL_PLAYER (0x06) | 32-byte payload; Y/Z rotation swap via `FUN_015846a0` |
| `0x00dda6c0` | `ServerConnection_SpaceViewportInfo` | SPACE_VIEWPORT_INFO (0x08) | CONSTANT_LENGTH=13 |
| `0x00dd9ee0` | `ServerConnection_ForcedPosition` | FORCED_POSITION (0x31) | CONSTANT_LENGTH=49 |
| `0x00dda0e0` | `PurgeAndRebuildEntityStateLists` | RESET_ENTITIES (0x04) | resets entity lists; calls BroadcastEntityActivation |
| `0x00df27f0` | `GameProxyPlayer_HandleOnClientMapLoad` | onClientMapLoad (method 117, sub=56) | fields: areaName, mapPath, WorldID, Location, Direction |
| `0x00c71a20` | `GameWorldConstants_HandleSetupWorldParameters` | setupWorldParameters (method 122, sub=61) | loads BW_TO_UE3_SCALE=100.0f and physics constants |

### Message Senders (client → server)

| Address | Function | Wire Message | Notes |
|---------|----------|--------------|-------|
| `0x00dd9280` | `BroadcastEntityActivation` | ENABLE_ENTITIES (base method 1) | sets bEntitiesEnabled @ ServerConnection+0x316 |
| `0x00449b20` | `Event_NetOut_versionInfoRequest_vfunc_3` | versionInfoRequest | post-terrain-load trigger |

### Entity Activation Paths

| Address | Function | Notes |
|---------|----------|-------|
| `0x00c6ed70` | `VoiceHandlerGated_ActivateEntitiesAndSetServerConnection` | secondary path; only fires when voice handler set |

### CME Signal Infrastructure

| Address | Symbol | Notes |
|---------|--------|-------|
| `0x00de04c0` | `register_NetIn_AccountLoginSuccess` | Phase 2 auth subscriber registration (Pattern A) |
| `0x00ddfd00` | `register_NetIn_ServerSelectSuccess` | Phase 2 shard-select subscriber registration |
| `0x00dde8d0` | `register_NetIn_LoginFailure` | login failure subscriber registration |
| `0x00d93d80` | `register_NetOut_ClientReady` | ClientReady signal registration |
| `0x00d45be0` | `MemberCallbackRtti_ClientReady__SGWNetworkManager` | RTTI accessor; vtable ref @ 0x019c5f2c |
| `0x00df7b80` | CME RTTI: `GameProxyPlayer + Event_World_Loaded` | subscriber vtable @ 0x019d5d94 |
| `0x00df6e80` | CME RTTI: `GameProxyPlayer + Event_Level_PostLoad` | subscriber vtable @ 0x019d5abc |
| `0x00e9a480` | CME RTTI: `GameAppearanceManager + Event_Level_PostLoad` | |
| `0x00e2af30` | CME RTTI: `Minimap + Event_World_Loaded` | |

### Post-Entry UE3 Callbacks

| Address | Function | Notes |
|---------|----------|-------|
| `0x00dd0b00` | `EntityManager_PostLoadMap` | UE3 CALLBACK_PostLoadMap (index 0x32) → fires Event_Level_PostLoad |
| `0x00de8670` | `GameProxyPlayer_HandlePlayerPawnCreated` | Event_Player_PawnCreated handler; sets up target indicator, ground target, player controller |

### Key Data Addresses

| Address | Symbol | Value / Notes |
|---------|--------|---------------|
| `DAT_01ef2500` | ENABLE_ENTITIES message descriptor | size field = **1 byte** (stock BW keepBase u8; NOT 8 bytes as previously hypothesized) |
| `DAT_018cad90` | `BW_TO_UE3_SCALE` | `0x42C80000` = 100.0f; loaded by setupWorldParameters |
| `DAT_01ee2b6c` | world-loaded guard flag | set after Event_World_Loaded fires; prevents double-emit |
| `DAT_01ee2684` | global `WorldInfo*` | read by Event_World_Loaded trigger thunk |
| `DAT_01eb082c` | editor/replay mode flag | if set, PostLoad handler skips transform copy |
| `0x017bae02` | ENABLE_ENTITIES descriptor init site | `MOV DWORD PTR [struct], 1` — confirms 1-byte payload |
| `0x019c2828` | `"onClientReady"` | CME signal name string |
| `0x019cf548` | enableEntities debug log | `"ServerConnection::enableEntities: Enabling entities %d\n"` |
| `0x019d09f8` | `"resetEntities"` | message name string; ref @ 0x017bb210 in msg table |
| `0x019d26c8` | `"areaName"` | onClientMapLoad field name (assert string) |
| `0x019d26d0` | `"mapPath"` | onClientMapLoad field name (assert string) |
| `0x019d2684` | `"WorldID"` | onClientMapLoad field name (assert string) |
| `ServerConnection+0x316` | `bEntitiesEnabled` | u8; toggled by BroadcastEntityActivation and PurgeAndRebuildEntityStateLists |

### World Entry — Handlers Resolved (W-misc-gaps, 2026-05-13)

| Address | Function | Notes |
|---------|----------|-------|
| `0x00de8660` | `GameProxyPlayer_HandleEvent_Level_PostLoad` | primary PostLoad handler; wrapper over FUN_00de8430 |
| `0x00de8430` | `FUN_00de8430` | PostLoad body: assigns UE3 PlayerController to mPlayerController, sets input mode, copies transform |
| `0x00de9e60` | `LAB_00de9e60` | alternate PostLoad handler (account-disconnect code path only) |
| `0x00df4270` | `FUN_00df4270` | main GameProxyPlayer callback registration fn (35 event subscriptions) |
| `0x005541a0` | `FUN_005541a0` | Event_World_Loaded emitter; fires after all sub-levels ready and entities settled |
| `0x007100d0` | `FUN_007100d0` | Event_World_Loaded trigger thunk; reads DAT_01ee2684; no static callers |
| `0x00d43dc0` | `SGWNetworkManager_EventHandler_ClientReady_invoke` | ClientReady wire-send; calls RouteOutgoingEntityRpc |
| `0x00d57030` | `FUN_00d57030` | SGWNetworkManager::EventHandler<ClientReady> ctor |
| `0x00d45b70` | `FUN_00d45b70` | MemberCallback<ClientReady, SGWNetworkManager> ctor |

### Timer System — Extended Subscriber Table (W-misc-gaps, 2026-05-13)

| Address | Function | Notes |
|---------|----------|-------|
| `0x00dec9e0` | `SGWBeing_onBigWorldTimeComplete` | Event_NetIn_TimerUpdate type 14 (0x0E); reads BigWorldTimeComplete double + SourceID; emits float countdown |
| `0x00d26380` | `FUN_00d26380` | DialogController Event_NetIn_TimerUpdate handler; type 6 (0x06); drives NPC interaction timer UI |
| `0x00d26ee0` | `FUN_00d26ee0` | MemberCallback<DialogController, Event_NetIn_TimerUpdate> ctor |
| `0x00d26850` | `FUN_00d26850` | DialogController constructor; registers handler FUN_00d26380 |
| `0x00e47800` | `FUN_00e47800` | SGW::Crafting Event_NetIn_TimerUpdate handler; type 16 (0x10); drives crafting timer UI |
| `0x00e45c70` | `FUN_00e45c70` | MemberCallback<SGW::Crafting, Event_NetIn_TimerUpdate> ctor |
| `0x00e49850` | `FUN_00e49850` | SGW::Crafting constructor; registers handler FUN_00e47800 |
| `0x00c68110` | `FUN_00c68110` | GameEntityManager Event_NetIn_TimerUpdate handler; type 1 (entity arrival/AoI pre-announce) |
| `0x00c6aaa0` | `FUN_00c6aaa0` | MemberCallback<GameEntityManager, Event_NetIn_TimerUpdate> ctor (3 data params, 0x10 bytes) |
| `0x00c69120` | `FUN_00c69120` | GameEntityManager constructor; registers handler FUN_00c68110 |
| `0x019bb3a0` | vtable | MemberCallback<DialogController, Event_NetIn_TimerUpdate> |
| `0x019db3ac` | vtable | MemberCallback<SGW::Crafting, Event_NetIn_TimerUpdate> |
| `0x019aad4c` | vtable | MemberCallback<GameEntityManager, Event_NetIn_TimerUpdate> |

### CRT Import IAT Slots (for allocator replacement)
| IAT Address | Function | DLL |
|-------------|----------|-----|
| `0x017ef990` | `malloc` | MSVCR80.dll |
| `0x017ef994` | `free` | MSVCR80.dll |
| `0x017efa58` | `realloc` | MSVCR80.dll |

## UE3 Engine Initialization

| Address | Function | Notes |
|---------|----------|-------|
| `0x00416010` | `GuardedMain` | Top-level engine entry, routes to wxWidgets or game loop |
| `0x004185e0` | `LaunchEngineLoop` | GIs* flag init, engine class selection, callback setup |
| `0x00417fe0` | Engine class selection | `if(!GIsEditor)` → GameEngine vs EditorEngine |
| `0x00418af0` | GIs* flag-setting code | 5 consecutive `MOV [addr], reg` instructions |
| `0x00486000` | `appFailAssert` | Assertion handler — called by `check()` macro |

### GIs* Flag Values by Mode
| Mode | GIsClient | GIsServer | GIsEditor | GIsUCC | GIsGame |
|------|-----------|-----------|-----------|--------|---------|
| Game | 1 | 0 | 0 | 0 | 1 |
| Editor | 1 | 1 | 1 | 0 | 0 |
| UCC | 0 | 0 | 0 | 1 | 0 |

## UE3 Core Functions

| Address | Function | Notes |
|---------|----------|-------|
| `0x004e06a0` | `UObject::ProcessEvent` | vtable index 58 (0x3A) |
| `0x0140fe54` | `UObject::VTable` | Base UObject vtable |
| `0x00ad4530` | `WxUnrealEdApp::OnInit` | Editor wxWidgets init |
| `0x00722e60` | `UEditorEngine::Init` | Editor engine init (transaction system, brush builders) |
| `0x000c43a0` | `FFileManager::MoveFile` | File manager — has bug where moving to same path deletes file |

## BWNetDriver / BWConnection — UE3↔BigWorld Network Bridge

CME replaced UE3's standard IpNetDriver with thin wrappers around BigWorld's Mercury networking.
Class hierarchy: `UObject → UNetDriver → UBWNetDriver` (IpDrv), `UObject → UNetConnectionBase → UBWConnection` (Engine).
UBWConnection adds NO extra data members — it's a pure UE3 wrapper; all real state lives in ServerConnection/Mercury::Channel.

### UBWNetDriver
| Address | Function | Notes |
|---------|----------|-------|
| `0x00480510` | `UBWNetDriver::UBWNetDriver` | Ctor: creates UBWConnection, calls htons(0) for Winsock init |
| `0x004807d0` | `UBWNetDriver::~UBWNetDriver` | Dtor chain → `0x004800c0` |
| `0x00480650` | `UBWNetDriver::StaticClass` | UE3 class registration (IpDrv package) |
| `0x00666540` | `UBWNetDriver::InitListen` | Returns FALSE — BW clients never listen |

### UBWConnection
| Address | Function | Notes |
|---------|----------|-------|
| `0x00480ad0` | `UBWConnection::UBWConnection` | Ctor: calls UNetConnectionBase ctor (0x005e0790) |
| `0x00480b30` | `UBWConnection::~UBWConnection` | Dtor chain → `0x0047fbe0` |
| `0x00480a00` | `UBWConnection::StaticClass` | UE3 class registration (Engine package, 0xB8 bytes) |

### ServerConnection (BigWorld native C++ — not a UObject)
| Address | Function | Notes |
|---------|----------|-------|
| `0x00ddf580` | `ServerConnection::logOnBegin` (first) | SOAP/curl to LoginApp: POST /SGWLogin/UserAuth |
| `0x00ddf9f0` | `ServerConnection::logOnBegin` (reconnect) | Mercury-level reconnect via existing channel |
| `0x00dd8ec0` | `ServerConnection::logOn` | Finalizes login: validates BaseApp addr, sends authenticate |
| `0x00dd8930` | `ServerConnection::send` | Sends current bundle through Mercury::Nub |
| `0x00dd86b0` | `ServerConnection::processInput` | Main packet processing (catch at 0x00dd87c6) |
| `0x00dd9280` | `ServerConnection::enableEntities` | Enables entity streaming after login |
| `0x00dd8630` | `ServerConnection::disconnect` | Destroys channel, clears handler |
| `0x00dd8c20` | `ServerConnection::loggedOff` | Handler for server disconnect notification |
| `0x00dd6130` | `ServerConnection::isConnected` | `return pChannel_ != NULL` |

### LoginReplyHandler / BaseAppLoginHandler
| Address | Function | Notes |
|---------|----------|-------|
| `0x00dded60` | LoginReplyHandler ctor (SOAP/curl) | curl_easy_init + curl_multi for async HTTP |
| `0x00dde380` | LoginReplyHandler ctor (Mercury) | Direct Mercury-level login for reconnection |
| `0x00de10b0` | LoginReplyHandler::handleTimeout | Drives curl_multi_perform at 100ms intervals |
| `0x00ddec40` | BaseAppLoginHandler::onBaseAppReply | Channel swap on success, retry on timeout |
| `0x00de4bf0` | BaseAppLoginHandler ctor | Sends "baseAppLogin" via Mercury to BaseApp |

### Login Flow
1. UE3 creates `UNetPendingLevel` (0x008ccc30) → constructs `UBWNetDriver` → constructs `UBWConnection`
2. `ServerConnection::logOnBegin` — curl SOAP POST to `/SGWLogin/UserAuth` (gSOAP namespace: `sgwlogin`)
3. Response parsed: `SGWLoginSuccess` contains BaseApp address, ticket, shard list
4. `BaseAppLoginHandler` creates Mercury::Channel to BaseApp, sends `baseAppLogin`
5. BaseApp replies → channel transferred to `ServerConnection::pChannel_`
6. `ServerConnection::logOn` sends `authenticate` message
7. `ServerConnection::enableEntities` enables entity streaming

### BaseAppExtInterface Message Table (at 0x019d086c)
Messages FROM client TO BaseApp:

| Name | Purpose |
|------|---------|
| `baseAppLogin` | Initial login to BaseApp |
| `authenticate` | Authentication after login |
| `avatarUpdateImplicit/Explicit` | Player position updates |
| `avatarUpdateWardImplicit/Explicit` | Ward entity position updates |
| `switchInterface` | Switch to different interface |
| `requestEntityUpdate` | Request entity data refresh |
| `enableEntities` | Enable entity streaming |
| `setSpaceViewportAck` | Ack space viewport change |
| `setVehicleAck` | Ack vehicle assignment |
| `restoreClientAck` | Ack client restore |
| `disconnectClient` | Client-initiated disconnect |
| `entityMessage` | Generic entity method call |

### ClientInterface Messages (FROM BaseApp TO client)
Key messages: `authenticate`, `bandwidthNotification`, `updateFrequencyNotification`, `setGameTime`, `resetEntities`, `createBasePlayer`, `createCellPlayer`, `spaceData`, `spaceViewportInfo`, `createEntity`, `updateEntity`, `entityInvisible`, `leaveAoI`, `tickSync`, `setSpaceViewport`, `setVehicle`, 24 `avatarUpdate*` variants, `detailedPosition`, `forcedPosition`, `controlEntity`, `loggedOff`, `restoreClient`, `resourceFragment`, `voiceData`.

### SOAP Login Types (gSOAP/curl)
SOAP namespace: `http://www.stargateworlds.com/xml/sgwlogin`
Auth endpoint: `/SGWLogin/UserAuth` | Server select: `/SGWLogin/ServerSelection`

Key RTTI types: `SGWLoginRequest`, `SGWLoginResponse`, `SGWLoginSuccess`, `AccountInfo`, `SGWShardListResp`, `SGWSelectServerRequest`, `SGWServerLocationResponse`, `UserPendingBaseAppMgrRequest/Response`, `TicketType`, `BaseAppAddress`, `ServerNameType`, `SessionKeyType`, `ga__GlobalAuthReq/Res`.

## Auth / Encryption (W-auth session, 2026-05-13)

Full analysis in [`findings/mercury-protocol-internals.md`](findings/mercury-protocol-internals.md) — "Cipher Key Derivation (Session 5 Verification)" section.

### PacketEncrypter

| Address | Name | Notes |
|---------|------|-------|
| `0x01603a70` | `PacketEncrypter_ctor` | Constructor; called from `register_NetIn_ServerSelectSuccess` with 32-byte session key |
| `0x01b27374` | `PacketEncrypter::vftable` | 12 slots; slots 1 & 2 are send/recv |
| `0x01604ac0` | `PacketEncrypter__vfunc_0` | Destructor |
| `0x01603b80` | `PacketEncrypter__send` | Encrypt outgoing packet (vfunc_1) |
| `0x01603fa0` | `PacketEncrypter__recv` | Decrypt incoming packet (vfunc_2) |
| `0x016043a0` | `PacketFilter_base_init` | Inits `Mercury::PacketFilter` vtable; base of PacketEncrypter hierarchy |

### Crypto++ Primitives (confirmed via RTTI vtable stamps)

| Address | Function | CryptoPP type |
|---------|----------|---------------|
| `0x0040e030` | AES-256 encryptor init | `CryptoPP::BlockCipherFinal<0, Rijndael::Enc>` |
| `0x0040d000` | CBC-Encryption mode init | `CryptoPP::CipherModeFinalTemplate_ExternalCipher<CBC_Encryption>` |
| `0x0040d0b0` | CBC-Decryption mode init | `CryptoPP::CipherModeFinalTemplate_ExternalCipher<CBC_Decryption>` |
| `0x01604d00` | HMAC-MD5 init | `CryptoPP::HMAC<CryptoPP::Weak1::MD5>` |
| `0x004089b0` | StreamTransformationFilter ctor | `CryptoPP::StreamTransformationFilter`; param=4 → PKCS_PADDING |
| `0x00414720` | HashFilter ctor | `CryptoPP::HashFilter`; wraps HMAC-MD5 for MAC output |
| `0x00a587f0` | IV buffer init | Copies 16 bytes (zero-filled) into `PacketEncrypter+0x18` |

### Session Key / KDF

| Address | Function | Notes |
|---------|----------|-------|
| `0x00ddfd00` | `register_NetIn_ServerSelectSuccess` | Entry point; allocates PacketEncrypter, passes 32-byte key |
| `0x015eb940` | gSOAP `SessionKeyType` deserializer | Dispatched as case 0x26 from `0x015ed300`; reads `xsd:hexBinary` → raw bytes |
| `0x015ed300` | gSOAP type dispatch | case 0x26 = `sgwLogin:SessionKeyType` |

**No KDF**: 64-char hex `SessionKey` in SOAP → gSOAP hex decoder → 32 raw bytes → AES key = HMAC key (same buffer, no transformation).

## ABigWorldEntity / UBigWorldInfo — UE3↔BigWorld Entity Bridge

ABigWorldEntity is a custom AActor subclass (size 0x1C4, extends AActor 0x1A8 + 0x1C bytes).
UBigWorldInfo is a UObject holding BigWorld connection parameters (size 0x44).
Both registered in the "Engine" package from `.\Src\BigWorldEntity.cpp`.

### Coordinate Conversion Constants
| Address | Name | Value | Notes |
|---------|------|-------|-------|
| `0x018cad90` | `BW_TO_UE3_SCALE` | `100.0f` | BW meters → UE3 centimeters |
| `0x018cafcc` | `RAD_TO_URU` | `10430.378f` | Radians → UE3 rotation units (65536/2π) |
| `0x018cafd0` | `NEG_RAD_TO_URU` | `-10430.378f` | Negated for axis swap |
| `0x018cae9c` | `URU_TO_RAD` | `9.58738e-05f` | UE3 rotation units → radians (2π/65536) |

### ABigWorldEntity
| Address | Function | Notes |
|---------|----------|-------|
| `0x0077ed90` | Constructor | Sets vtable to 0x01895e3c |
| `0x0077ed20` | Destructor | / `0x0077edf0` (scripted) |
| `0x0084b040` | `ABigWorldEntity::StaticClass` | Registration at BigWorldEntity.cpp line 85 |
| `0x0084adc0` | `ABigWorldEntity::AttachComponent` | **Key override**: disables UE3 collision for BW entities (sets `CollisionResponseFlags = 0xFFFFC004`) then calls `AActor::AttachComponent` (0x006e6c10) |
| `0x00527800` | `execAttachComponent` | UScript native thunk |

### UBigWorldInfo
| Address | Function | Notes |
|---------|----------|-------|
| `0x0084b110` | Constructor | Calls UObject base |
| `0x0084b160` | Destructor | / `0x0084b1b0` |
| `0x0084ad00` | `UBigWorldInfo::StaticClass` | Registration at BigWorldEntity.cpp line 86 |
| `0x0084a880` | `UBigWorldInfo::Init` | Virtual at vtable offset 0x10c — **empty stub** |
| `0x00533E90` | `execInit` | UScript native thunk |

### BigWorld Package Registration (0x0084a8f0)
1. Creates UE3 package "BigWorld" via `FUN_0049e960(L"BigWorld", 1, 1)`
2. Creates `UnrealMessageCallback` (0x0084b210) bridging BW debug→UE3 logging
3. Registers native functions for both classes
4. Hooks into CME EventSignal system

### ABigWorldEntity Collision Behavior
When a `USkeletalMeshComponent` is attached, ABigWorldEntity sets `CollisionResponseFlags = 0xFFFFC004` to disable UE3 collision — collision is handled by BigWorld's spatial system instead.

### GameEntityBase (.\Src\GameEntityBase.cpp) — The Actual Bridge Object

GameEntityBase is the CME class that wraps a BigWorld entity and maintains the connection to its UE3 actor. ABigWorldEntity is a thin AActor; GameEntityBase does the heavy lifting.

**Field layout:**
| Offset | Type | Name | Notes |
|--------|------|------|-------|
| `+0x04` | `CacheData*` | `mCacheData` | Position cache for entities not yet in world |
| `+0x08` | `AActor*` | `mActor` | The UE3 ABigWorldEntity actor |
| `+0x0C` | `EntityID` | `mEntityID` | |
| `+0x10` | `EntityTypeID` | `mEntityType` | |
| `+0x14` | `SmartPointer` | `mAppearance` | |
| `+0x20` | `double` | `mLastUpdateTime` | |
| `+0x24` | `SpaceID` | `mSpaceID` | |
| `+0x28` | `bool` | `mIsVolatile` | |
| `+0x2C` | `EntityID` | `mVehicleID` | |
| `+0x30` | `bool` | `mInWorld` | |

**CacheData layout (0x1C bytes):** Position.XYZ (3×f32), Velocity.XYZ (3×f32), bIsVolatile (u8)

| Address | Function | Notes |
|---------|----------|-------|
| `0x00e68a30` | `GameEntityBase::ApplyTransform` | Writes BW position/rotation to UE3 Actor→Location/Rotation |
| `0x00e68670` | `GameEntityBase::EnterWorld` | |
| `0x00e685e0` | `GameEntityBase::Init` | Allocates CacheData, sets up bridge |
| `0x00e685c0` | `GameEntityBase::GetPosition` | |
| `0x00e69150` | Entity enterWorld handler | Checks appearance readiness, schedules AppearanceJob |

### Coordinate Axis Swap

Position axes are swapped during BW→UE3 conversion (not just scaled):
```
UE3_X = BW_Z * 100.0    BW_Z → UE3_X
UE3_Y = BW_X * 100.0    BW_X → UE3_Y
UE3_Z = BW_Y * 100.0    BW_Y → UE3_Z
```

Rotation conversion functions:
| Address | Function | Notes |
|---------|----------|-------|
| `0x0084a8a0` | `RotatorToRadians` | UE3 FRotator → BW radians (pitch/roll negated for handedness) |
| `0x0084a9d0` | `RadiansToRotator` | BW radians → UE3 FRotator (pitch/roll negated for handedness) |

### Debug HUD
| Address | Function | Notes |
|---------|----------|-------|
| `0x00c739e0` | `DrawBigWorldDebugInfo` | Displays BW position (meters), BW rotation (radians), UE3 position |

### Config
- `Engine.BigWorldInfo.DefaultBigWorld` — INI key for default BW connection parameters (at `0x008f5c21`)
- `Editor.EditorEngine.BWResDirectory` — BigWorld resource directory path (at `0x0084ae70`)

## Cooked Data Pipeline (W-cooked session 5, 2026-05-13)

Full findings in [`findings/cooked-data-pipeline.md`](findings/cooked-data-pipeline.md).

Key finding: the client registers exactly 21 `ServerSource` categories (IDs 1–21). Category 0 does NOT exist
on the client. The server's `resource.cpp` table starting at 0 is inconsistent with the client.

### CacheLibrary / LibCategory

| Address | Name | Notes |
|---------|------|-------|
| `0x00420074` | `CookedData_RegisterAllLibCategories` | Startup init: reads SourceCachePath INI, registers all 21 LibCategory objects with CacheLibrary |
| `0x004786c0` | `LibCategoryBase_Ctor` | Sets vtable at `+0x0`, category ID at `+0x4` |
| `0x004786f0` | `CacheLibrary_GetSingleton` | Lazy-init: 0xC-byte CacheLibrary at `DAT_01ea56d8`; shutdown guard at `DAT_01ea56dc` |
| `0x00478840` | `CacheLibrary_Ctor` | SEH wrapper → calls `FUN_0157ce00` (body ctor) |
| `0x00437650` | `CacheLibrary_RegisterCategory` | Inserts LibCategory into internal red-black tree map keyed by category ID |
| `0x0044c800` | `LibCategory_ServerSource_Ctor_cat1_KismetSeqEvent` | Template ctor for category 1; wires 5 CME event subscriptions |
| `0x01ea56d8` | `g_pCacheLibrary` | Global: CacheLibrary singleton pointer (12 bytes) |
| `0x01ea56dc` | `g_CacheLibraryInitialized` | Global: init/shutdown state byte |

### CME MemberCallback Constructors (per event type)

| Address | Name | Event |
|---------|------|-------|
| `0x004267f0` | `CME_MemberCallback_Ctor_ServerSource_NetConnected` | `Event_Net_Connected` |
| `0x004268f0` | `CME_MemberCallback_Ctor_ServerSource_NetDisconnected` | `Event_Net_Disconnected` |
| `0x00426970` | `CME_MemberCallback_Ctor_ServerSource_onVersionInfo` | `Event_NetIn_onVersionInfo` |
| `0x004269f0` | `CME_MemberCallback_Ctor_ServerSource_NetProxyData` | `Event_Net_ProxyData` (RESOURCE_FRAGMENT delivery) |
| `0x00426a70` | `CME_MemberCallback_Ctor_ServerSource_onCookedDataError` | `Event_NetIn_onCookedDataError` |

### CME Subscription Wrappers

| Address | Name | Notes |
|---------|------|-------|
| `0x0042a7b0` | `CME_Subscribe_ServerSource_NetConnected` | alloc 0xC + ctor + Subscribe |
| `0x0042a840` | `CME_Subscribe_ServerSource_NetDisconnected` | alloc 0xC + ctor + Subscribe |
| `0x0042a8d0` | `CME_Subscribe_ServerSource_onVersionInfo` | alloc 0xC + ctor + Subscribe |
| `0x0042a960` | `CME_Subscribe_ServerSource_NetProxyData` | alloc 0xC + ctor + Subscribe |
| `0x0042a9f0` | `CME_Subscribe_ServerSource_onCookedDataError` | alloc 0xC + ctor + Subscribe |
| `0x00a37790` | `CME_EventSignal_Subscribe` | Core subscribe: RTTI type resolution via vfunc_2, then rb-tree insert |

### ServerSource Event Handlers (cat-6 template instantiation)

| Address | Name | Notes |
|---------|------|-------|
| `0x00441630` | `ServerSource_onVersionInfo_Handler_cat6` | Reads CategoryId/RequiredUpdates/InvalidateAll/Version; fires elementDataRequest per pending entry |
| `0x00441aa0` | `ServerSource_onCookedDataError_Handler_cat6` | Reads categoryID/elementKey; decrements RequiredUpdates; fires Event_Cache_ElementError |

### ZipStorage / PAK Archive

| Address | Name | Notes |
|---------|------|-------|
| `0x00479340` | `ZipStorageBase_OpenArchive` | Opens PAK ZIP; creates cache dir; source: ZipStorage.cpp:130 |
| `0x00479930` | `ZipStorageBase_WriteStreamToFile` | Writes ostream to named ZIP entry; source: ZipStorage.cpp |
| `0x00479e10` | `ZipStorageBase_WriteMetaDataVersion` | Writes 4-byte version stamp to PAK "MetaData" ZIP entry |
| `0x00479e90` | `ServerSource_SetVersion` | Stores server version at `this+0x24` → calls WriteMetaDataVersion |
| `0x0043bdb0` | `ServerSource_RequestElement` | Cache-miss check → fires `Event_NetOut_elementDataRequest(cat, key)` |
| `0x013a1620` | `CZipStorage_Dtor` | Destroys wstring at `+0xC`, CZipAutoBuffer at `+0x5C` |

### Category→PAK Mapping (confirmed from binary — all ServerSource, DEFLATE ZIP)

| ID | PAK File | C++ DataType |
|----|----------|--------------|
| 1 | `CookedDataKismetSeqEvent.pak` | `CookedKismetEventSequenceData` |
| 2 | `CookedDataAbilities.pak` | `Ability` |
| 3 | `CookedDataMissions.pak` | `Mission` |
| 4 | `CookedDataItems.pak` | `DBInvItem` |
| 5 | `CookedDataDialogs.pak` | `Dialog` |
| 6 | `CookedDataKismetSetEvent.pak` | `CookedKismetEventSetData` |
| 7 | `CookedCharCreation.pak` | `CookedCharCreationData` (special case — char creation category) |
| 8 | `CookedInteractionSet.pak` | `InteractionSet` |
| 9 | `CookedDataEffects.pak` | `DBEffect` |
| 10 | `TextStrings.pak` | `CookedTextType` |
| 11 | `ErrorStrings.pak` | `CookedErrorTextType` |
| 12 | `CookedWorldInfo.pak` | `SGW::WorldInfo` |
| 13 | `CookedDataStargates.pak` | `DBGateInfo` |
| 14 | `CookedDataContainers.pak` | `DBInvContainer` |
| 15 | `CookedBlueprints.pak` | `SGW::Blueprint` |
| 16 | `CookedSciences.pak` | `SGW::AppliedScience` |
| 17 | `CookedDisciplines.pak` | `SGW::Discipline` |
| 18 | `CookedParadigm.pak` | `SGW::RacialParadigm` |
| 19 | `SpecialWords.pak` | `CookedSpecialWordsType` |
| 20 | `CookedInteractions.pak` | `SGW::InteractionData` |
| 21 | `CookedBehaviorEvents.pak` | `BehaviorEventData` |

## Mercury Protocol Functions

| Address | Function | Notes |
|---------|----------|-------|
| `0x015841d0` | `Mercury::Nub::Nub` | Constructor: creates "NetworkThread for ExternalNub", two tbb::concurrent_queue |
| `0x01581ab0` | `Mercury::Nub::processPendingEvents` | NetworkThread: recvfrom loop → ClientMessage → concurrent_queue |
| `0x01576f90` | `Mercury::Nub::sendInternal` | Called from ServerConnection::send |
| TODO | `Mercury::Channel::send` | Outgoing message dispatch |
| TODO | `Mercury::Channel::processMessage` | Incoming message processing |
| TODO | `Mercury::Bundle::startMessage` | Begin constructing a message |
| TODO | `Mercury::Bundle::addBlob` | Add raw data to message |

### Mercury::Nub Threading Model
- Background thread ("NetworkThread for ExternalNub") handles raw UDP I/O via `processPendingEvents`
- Received packets wrapped as `Mercury::ClientMessage` pushed to `tbb::concurrent_queue` at Nub offset +0x138
- Game thread pops messages during `UGameEngine::Tick` (0x008f6930) → `TickDispatch`
- Socket error handling: WSAETIMEDOUT (0x274D), WSAECONNRESET (0x2751), WSAECONNREFUSED (0x2746) → NubException

## Entity Property Functions

| Address | Function | Notes |
|---------|----------|-------|
| `0x015652d0` | `FNetworkPropertyChange__vfunc_0` | Property change serialization |
| `0x015924a0` | `EntityDescription_parseProperties` | Property ID assignment |
| `0x015974a0` | `DataDescription_parse_2` | Property flag/type parsing |
| TODO | `Entity::readCellData` | Deserialize cell entity data |
| TODO | `Entity::readBaseData` | Deserialize base entity data |

## UE3 Editor Integration (Atrea patches)

Atrea (`AtreaLoader.config.xml`) applies runtime patches to enable editor/UCC modes.

| Patch | RVA | Description |
|-------|-----|-------------|
| EditorMode | `0x00018AF0` | Rewrites GIs* flags for editor mode |
| EditorCallbacks | `0x000186D2` | Installs editor callback devices (FCallbackEventDeviceEditor) |
| EditorCallbackVMT | `0x0198F52C` | Replaces game VMT pointers with editor VMT pointers |
| EditorSettings | `0x001757BA` | Inverts EDITOR command-line check (`setz` → `setnz`) |
| EditorCurrentPackage | `0x0198F4A0` | Changes current package from L"Launch" to L"UnrealEd" |
| EditorChunkLimit | `0x007FDA41` | Removes 100-chunk limit for streaming maps (`JLE` → `JMP`) |
| EditorMyGamesDir | `0x0008D1E8` | Skips %USERPROFILE%\My Games redirect (`CALL` → `NOP NOP`) |
| DisablePrefabSerialize | `0x001CE8E1` | Skips prefab serialization on load (`JGE` → `NOP+JMP`) |
| MallocOOMSoftFail | `0x00019923` | Returns NULL on OOM instead of asserting |

## Reference Source Code

| Source | Path | Match Quality |
|--------|------|--------------|
| BigWorld 1.9.1 | `F:\Stargate Worlds-QA\Reference\BigWorld-Engine-1.9.1\` | HIGH — BW ≥1.8.1, VC2005, 1:1 Mercury match |
| UE3 Early (2004) | `F:\Stargate Worlds-QA\Reference\UE3-2004\` | MED-HIGH — same era, VC80.sln, core architecture |
| UE3 CodeRed (2013) | `F:\Stargate Worlds-QA\Reference\UE3-CodeRed\` | MEDIUM — has FMallocTBB, later build |

### Key Source File Matches
| SGW.exe Embedded Path | Reference Source |
|----------------------|-----------------|
| `..\..\..\..\Server\bigworld\src\client\entity_manager.cpp` | `BigWorld-1.9.1/bigworld/src/client/entity_manager.cpp` |
| `..\..\..\..\Server\bigworld\src\common\servconn.cpp` | `BigWorld-1.9.1/bigworld/src/common/servconn.cpp` |
| `nub.cpp` | `BigWorld-1.9.1/src/lib/network/nub.cpp` |
| `LaunchEngineLoop.cpp` | `UE3-2004/Development/Src/Launch/Src/LaunchEngineLoop.cpp` |
| `UnObj.cpp` | `UE3-2004/Development/Src/Core/Src/UnObj.cpp` |
| `FMallocCME.h` | CME custom — derived from `UE3-CodeRed/.../FMallocTBB.h` |

### CME Custom Source Files (no reference source available)
| File | Assertion Refs | Domain |
|------|---------------|--------|
| `BWNetDriver.cpp` | 1+ | UE3↔BigWorld network driver |
| `BWConnection.cpp` | 1+ | UE3↔BigWorld connection wrapper |
| `BigWorldEntity.cpp` | 4+ | UE3 actor for BW entities |
| `FMallocCME.h` | 1 (line 37) | TBB allocator (32-bit backport of FMallocTBB) |
| `BaseAppearanceJob.cpp` | 1+ | Character appearance system |

## Inventory State (W-inventory-state session, 2026-05-13)

See [`findings/inventory-state-machine.md`](findings/inventory-state-machine.md) for full analysis.

Key finding: there is **no separate equip wire message**. Equip and unequip are implemented as `moveItem` (method 38) targeting equipment container IDs 4–14. The `/equipitem` slash command routes through `FUN_00e1f420` (equip-by-name on the client Inventory model), which in turn calls `EmitNetOut_MoveItem`.

### Inventory Class Init + Subscriptions

| Address | Function | Notes |
|---------|----------|-------|
| `0x00e20da0` | `Inventory_Init` | Initializer: registers all 14 CME event signal subscriptions; evidence link in MemberCallback chain |
| `0x00e1f6b0` | `Inventory_HandleOnContainerInfo` | S→C handler: reads Bags[] + Items[] FIXED_DICT arrays; called on world entry and after bag ops |
| `0x00e1fb20` | `Inventory_HandleOnActiveSlotUpdate` | S→C handler: reads BagId + SlotId; updates active bandolier slot in client model |
| `0x00e1fd30` | `Inventory_HandleOnUpdateItem` | S→C handler: reads ItemUpdates[] FIXED_DICT array; 600+ lines decompiled C; creates/updates item objects |
| `0x00e1da00` | `Inventory_HandleOnRemoveItem` | S→C handler: removes item from client model by itemId |
| `0x00e1db80` | `Inventory_HandleOnRefreshItem` | S→C handler: marks item stale or triggers re-fetch (exact semantics unconfirmed) |
| `0x00e1dcc0` | `Inventory_HandleOnClearOrgVaultInventory` | S→C handler: clears org vault (team/command bank containers) |
| `0x00e21ce0` | MemberCallback ctor for onUpdateItem subscriber | Traced as anchor to find Inventory_Init |
| `0x00e224e0` | Intermediate setup function | Caller of `0x00e21ce0`; called by `Inventory_Init` |

### Inventory Emit Functions

| Address | Function | Notes |
|---------|----------|-------|
| `0x00e1e340` | `EmitNetOut_MoveItem` | Pattern A; fields: ItemId, TargetBag, TargetSlot, Quantity; method 38 |
| `0x00e1ef70` | `EmitNetOut_RequestActiveSlotChange` | Pattern A; fields: BagId (hardcoded 3), SlotId; method 41 |
| `0x00e1f420` | `Inventory_EquipByName` | Client-side equip-by-name; looks up item, determines equipment slot, calls `EmitNetOut_MoveItem` |
| `0x00e1f480` | `Inventory_UnequipByName` | Client-side unequip-by-name; calls `EmitNetOut_MoveItem` targeting INV_MAIN |

### SGWTextCommandMgr Inventory Handlers

| Address | Function | Notes |
|---------|----------|-------|
| `0x00c8d0f0` | `SGWTextCommandMgr_Ctor` | Constructor: registers ~130 slash command handlers including equip/unequip/bandolier |
| `0x00c96db0` | `MemberCallback_EquipItem_SGWTextCmdMgr_Ctor` | MemberCallback ctor for `/equipitem` subscriber; vtable `0x019b174c` |
| `0x00c9d8c0` | Intermediate registration fn | Called by `SGWTextCommandMgr_Ctor`; calls `MemberCallback_EquipItem_SGWTextCmdMgr_Ctor` |
| `0x00c73da0` | `SGWTextCmdMgr_HandleEquipItem` | Reads `ItemName`, calls `Inventory_EquipByName` (`0x00e1f420`) |
| `0x00c73ee0` | `SGWTextCmdMgr_HandleUnequipItem` | Reads `ItemName`, calls `Inventory_UnequipByName` (`0x00e1f480`) |
| `0x00c74d20` | `SGWTextCmdMgr_HandleActivateBandolierSlot` | Reads `SlotNum`, calls `EmitNetOut_RequestActiveSlotChange(BagId=3, SlotNum)` |

### Container ID → Equipment Slot Mapping

| Container ID | Rust Constant | Body Slot |
|---|---|---|
| 3 | `INV_BANDOLIER` | Active weapon slots (4 slots) |
| 4 | `INV_HEAD` | Head |
| 5 | `INV_FACE` | Face |
| 6 | `INV_NECK` | Neck |
| 7 | `INV_CHEST` | Chest |
| 8 | `INV_HANDS` | Hands |
| 9 | `INV_WAIST` | Waist |
| 10 | `INV_BACK` | Back |
| 11 | `INV_LEGS` | Legs |
| 12 | `INV_FEET` | Feet |
| 13 | `INV_ARTIFACT1` | Artifact 1 |
| 14 | `INV_ARTIFACT2` | Artifact 2 |

### MemberCallback RTTI — Inventory Cluster

| Address range | Cluster |
|---------------|---------|
| `0x00e219b0 – 0x00e21a10` | NetIn inventory CallbackImpl cluster: `onContainerInfo` through `onCashChanged` (6 entries, 0x10 spacing) |
| `0x019b174c` | `MemberCallbackRtti_SlashCmd_EquipItem__SGWTextCommandMgr` vtable — confirms EquipItem slash command subscriber RTTI |
| `DebugCommunication.cpp` | 5 | Debug/telemetry system |
| `SGWTestIpDrv.cpp` | 1+ | Network testing utility |
| `TcpNetDriver.cpp` | 2+ | TCP fallback network driver |
| `ZipStorage.cpp` | 10 | Zip archive handling |

## CME Custom Modules

### BaseAppearanceJob.cpp
| Address | Function | Notes |
|---------|----------|-------|
| `0x00eb7450` | Assertion location | State machine for async character appearance loading |
| — | BaseAppearanceJob | Abstract base: DoWork → PostProcess → Cleanup virtual dispatch |
| — | PawnAppearanceJob | Subclass for character models |
| — | CompositedAppearanceJob | Subclass for composited appearances |
| — | StaticMeshAppearanceJob | Subclass for static meshes |

Fires `Event_AppearanceJob_Completed` consumed by: GameAppearanceManager, GameBeing, GameProxyPlayer, SequenceManager, CharacterCreation, PortraitManager.

### CompositedAppearanceProxy.cpp — entity+0x3D2 write site
| Address | Function | Notes |
|---------|----------|-------|
| `0x00ec0840` | `CompositedAppearanceProxy::ApplyToPawn` | **THE WRITER of entity+0x3D2**: writes weapon-category byte from proxy+0x34. Debug: `"Applying CompositedAppearanceProxy to pawn"`. Calls `GameBeing_UpdateCombatStanceWeaponSet` after write. |
| `0x00ec08e5` | write site | `MOV [entity+0x3D2], al` — single byte write in entire binary to this offset. |
| `0x00ebe840` | proxy configurator | Sets proxy+0x34 = job[0x1e] (weapon category from BeingAppearance ComponentList). Sets proxy+0x38 = job[0x30]. |
| `0x00eb4be0` | `IComposingProcessContinuation::Process` | Invoked on CompositedAppearanceJob completion; gets pawn from entity listener; calls ApplyToPawn. |
| `0x00ec0680` | `CompositedAppearanceProxy` ctor | Initializes proxy+0x34 = 0, proxy+0x38 = 0. |

### GameBeing appearance pipeline (BeingAppearance → entity+0x3D2)
| Address | Function | Notes |
|---------|----------|-------|
| `0x00e01360` | `GameBeing::HandleNetIn_BeingAppearance` | CME subscriber; reads BodySet + ComponentList; calls setAppearance. |
| `0x00e00bc0` | `GameBeing::setAppearance` | Debug string: `"GameBeing::setAppearance"`. Schedules appearance compositing job. |
| `0x00e69150` | `GameAppearanceManager::scheduleAppearanceJob` | Debug: SCHEDULING JOB / HOLD FOR TRANSACTION / ENTITY NOT READY. Calls FUN_00e998e0 to enqueue. |
| `0x00ebdb50` | `CompositingProcess_main` | Async TBB compositing task. Source: `CompositingProcess_main.cpp`. |
| `0x00e69070` | EntityListenerEntry→pawn | Returns `*(param_1+8)` as pawn/GameEntity pointer. |

### DebugCommunication.cpp
| Address | Function | Notes |
|---------|----------|-------|
| `0x0047c2f0` | Assertion 1 | FDebugReceiver / FDebugSender (FRunnable subclasses) |
| `0x0047c980` | Assertion 2 | Lock-free SPSC ring buffer for inter-thread comms |
| `0x0047c3e0` | Assertion 3 | UDP telemetry on ports 13500/13502 |

Packet format: `'S' 'T' + name + payload` (~4KB max). Configurable via `DebugCommunication.UDP` INI section.

### TcpNetDriver.cpp
| Address | Function | Notes |
|---------|----------|-------|
| `0x0047ec40` | Assertion 1 | UTcpNetDriver (0x14C bytes, extends UNetDriver) |
| `0x0047efb0` | Assertion 2 | UTcpipConnection (0x4F74 bytes / ~20KB, extends UNetConnection) |
| `0x0047b860` | `RegisterIpDrvClasses` | Registers all 5 IpDrv classes |

Custom bool properties: `AllowPlayerPortUnreach`, `LogPortUnreach`.

### ZipStorage.cpp
| Address | Function | Notes |
|---------|----------|-------|
| `0x00479fa0` | Assertion/log 1 | `Detail::ZipStorageBase` with 7 template instantiations |
| `0x00479340` | Assertion/log 2 | Kismet events, missions, inventory, dialogs, interactions |
| `0x00479930` | Assertion/log 3 | Uses CZipArchive for zip I/O, versioned cache invalidation |
| `0x00478f90` | Assertion/log 4 | Subscribes to 6 CME events (Connected, Disconnected, etc.) |

Uses Apache log4cxx for structured logging with source location info.

### SGWTestIpDrv.cpp
| Address | Function | Notes |
|---------|----------|-------|
| `0x0047dd40` | Assertion | UTestIpDrv (0x178 bytes), UObject-derived network testing utility |

## Animation System (session 4 — W-anim)

See [`findings/animation-system.md`](findings/animation-system.md) for the full evidence trail.

### Anim Notify Classes (CME custom subclasses of UAnimNotify)

| Address | Function | Notes |
|---------|----------|-------|
| `0x00e974b0` | `SGWAnimNotifyEvent_Emit` | CME emitter: reads actor name → LookupByName → sets SequenceName/CancelOnMovement/PlaybackType/HaltAnimTree → dispatches. Source: `SGWAnimNotify_Event.cpp:25`. Vtable-only (no direct callers). |
| `0x00e97070` | `USGWAnimNotify_Event::Notify` (unnamed) | Actual Notify virtual override. Validates mesh component, checks bone socket, dispatches via entity vtable `+0xE8`/`+0xEC`. Source: `SGWAnimNotify_Event.cpp`. |
| `0x00e97290` | `USGWAnimNotify_Event__vfunc_0` | MSVC scalar destructor stub (`return 1`). |
| `0x00e97ae0` | `USGWAnimNotify_JumpEvent__vfunc_0` | MSVC scalar destructor stub (`return 1`). Source: `SGWAnimNotify_JumpEvent.cpp`. |
| `0x00e96f60` | `USGWAnimNotify_Script__vfunc_0` | MSVC scalar destructor stub (`return 1`). Source: `SGWAnimNotify_Script.cpp`. |

### RTTI type descriptors for SGW anim notify classes

| Address | String |
|---------|--------|
| `0x01e6bb60` | `.?AVUSGWAnimNotify_Script@@` |
| `0x01e6bb84` | `.?AVUSGWAnimNotify_Event@@` |
| `0x01e6bba8` | `.?AVUSGWAnimNotify_JumpEvent@@` |

### USGWAnimController

| Address | Function | Notes |
|---------|----------|-------|
| `0x00e95b60` | `USGWAnimController__vfunc_0` | MSVC scalar destructor stub (`return 1`). |
| `0x00e95d10` | `USGWAnimController::StaticClass` | Allocates 0x178 bytes; UScript class size 0xD4; Engine package. Source: `SGWAnimController.cpp:0xC2`. |

UScript-native methods (dispatch table at `0x01df10d0`): `execClearSecondAnim`, `execClearAnim`, `execPlayAnimNode`, `execPlaySecondNamedAnim`, `execPlayNamedAnim`.

### USGWAnim_TransitionByStance — combat stance / holster / draw / crouch transitions

| Address | Function | Notes |
|---------|----------|-------|
| `0x00e966d0` | `USGWAnim_TransitionByStance__vfunc_0` | MSVC scalar destructor stub. Source: `SGWAnim_TransitionByStance.cpp`. |
| `0x00e96720` | Stance lookup (unnamed) | Searches 0x14-byte table at `this+0x12C` (count at `this+0x130`) matching 3-char weapon code + 3-char posture code. Returns index or -1. AnimMap.xml index lookup. |
| `0x00e96810` | Anim sequence trigger (unnamed) | Fires vtable `+0x13C` for each sequence in the matched entry, resets to -1 when complete. |
| `0x00e968d0` | Tick/update (unnamed, vfunc_70 override) | Reads `entity+0x3D0` (combat stance code), calls lookup, on change fires trigger. **entity+0x3D0 is the 3+3 char weapon+posture code.** |
| `0x00e96940` | TickAnim (unnamed, vfunc_72 override) | Calls base `UAnimNodeSequence::vfunc_72(deltaTime)`, then trigger if flag bit 1 at `this+0xC0` is clear. |
| `0x00e96c50` | Ctor (unnamed) | Calls `FUN_00804390` (UAnimNodeBlendBase ctor), stamps `USGWAnim_TransitionByStance::vftable`. |
| `0x00e969d0` | `USGWAnim_TransitionByStance::StaticClass` | Class size 0x138 (312 bytes); Engine package. |

**Key `USGWAnim_TransitionByStance` field layout:**

| Offset | Type | Name | Notes |
|--------|------|------|-------|
| `+0x11C` | `char[4]` | `currentStanceCode` | Last seen stance code (e.g. `"1HS"`) |
| `+0x120` | `int` | `currentAnimIndex` | Current anim table index; -1 = none |
| `+0x124` | `int` | `sequenceCounter` | Sequence playback counter |
| `+0x128` | `entity*` | `pEntity` | Entity whose combat state drives transitions |
| `+0x12C` | `SGWAnimTransitionEntry*` | `pAnimTable` | AnimMap.xml entries |
| `+0x130` | `int` | `animTableCount` | Entry count |
| `+0xC0 bit 1` | `bool` | `suppressTrigger` | If set: suppresses anim trigger during TickAnim |
| `+0xCC` | `USkeletalMeshComponent*` | `pMesh` | Mesh for scale validation |

### USGWAnim_BlendByPosture / USGWAnim_BlendByWeapon

| Address | Function | Notes |
|---------|----------|-------|
| `0x00e92170` | `USGWAnim_BlendByPosture::StaticClass` | Class size 0xF0; Engine package. Source: `SGWAnim_BlendByPosture.cpp`. |
| `0x00e92240` | `USGWAnim_BlendByPosture` ctor | Calls `FUN_00e8a6e0` (SGWAnim base ctor), stamps `USGWAnim_BlendByPosture::vftable`. |
| `0x00e91f40` | `USGWAnim_BlendByPosture__vfunc_0` | MSVC scalar destructor stub. |
| `0x00e925a0` | `USGWAnim_BlendByWeapon::StaticClass` | Class size 0xF4; Engine package. Source: `SGWAnim_BlendByWeapon.cpp`. |
| `0x00e92520` | `USGWAnim_BlendByWeapon__vfunc_0` | MSVC scalar destructor stub. |
| `0x00e92670` | `USGWAnim_BlendByWeapon` ctor | Calls `FUN_00e8a6e0`, stamps `USGWAnim_BlendByWeapon::vftable`. |

### USGWAnim_BlendByPosture — posture-driven blend (session 5)

| Address | Function | Notes |
|---------|----------|-------|
| `0x00e92350` | `SGWAnimBlendByPosture_TickAnim` | Maps entity posture enum (entity+0x3D2) to blend child: 1=crouch→child 2, 2=stand→child 1, 3=partial→child 3/4 (sub-state at entity+0x3D1), 5-8=special. Source: `SGWAnim_BlendByPosture.cpp`. |

### USGWAnim_BlendByWeapon — weapon-type blend selector (session 5)

| Address | Function | Notes |
|---------|----------|-------|
| `0x00e92530` | `SGWAnimBlendByWeapon_IsWeaponEquipped` | Checks whether current actor's weapon is the BlendByWeapon base class. |
| `0x00e92570` | `SGWAnimBlendByWeapon_OnChildGroupChange` | Caches weapon type index (via `FUN_00d404b0`) in `this+0xF0` on group change. |
| `0x00e928e0` | `SGWAnimBlendByWeapon_TickAnim` | Selects blend child with highest weapon priority (via `FUN_007ff790`); ties broken by anim completion fraction. Calls `SetActiveChild` via vtable +0x17C. Source: `SGWAnim_BlendByWeapon.cpp`. |

### USGWAnim_TransitionByPosture — crouch/stand transition (session 5)

| Address | Function | Notes |
|---------|----------|-------|
| `0x00e92db0` | `SGWAnimTransitionByPosture_OnChildGroupChange` | Dynamic-casts entity to GameBeing; queries FUN_00dff3f0 for crouch state; updates this+0xF0 bit 1. |
| `0x00e92e30` | `SGWAnimTransitionByPosture_SetActiveChild` | Validates incoming child is playable; sets transition pending flag; selects blend weight 0 (snap) or computed weight. |
| `0x00e92ef0` | `SGWAnimTransitionByPosture_TickAnim` | Polls entity crouch state each tick; fires SetActiveChild(1) for stand→crouch, SetActiveChild(2) for crouch→stand. Cached state in this+0xF0 bit 0. Source: `SGWAnim_TransitionByPosture.cpp`. |

**USGWAnim_TransitionByPosture field layout:**

| Offset | Type | Name | Notes |
|--------|------|------|-------|
| `+0x3C`  | `actor*` | `pOwnedActor` | The actor this node is attached to |
| `+0xB4`  | `ptr` | `pChildList` | UAnimNodeBlendList child array base |
| `+0xD4`  | `int` | `nActiveChild` | Current active child index |
| `+0xF0 bit 0` | `bool` | `bLastCrouchState` | Cached crouch state from previous tick |
| `+0xF0 bit 1` | `bool` | `bTransitionPending` | Set while transition animation plays |
| `actor+0x1B4` | `entity*` | `pEntity` | Entity ptr stored on the actor |
| `child+0xC0 bit 1` | `bool` | `bAnimStillActive` | UAnimNodeBlend active flag (used to detect transition completion) |

### USGWAnim_BlendByCover — cover lean blend (session 5)

| Address | Function | Notes |
|---------|----------|-------|
| `0x00e90d80` | `SGWAnimBlendByCover_IsInCover` | Checks whether actor's class is in cover base class hierarchy. |
| `0x00e90dc0` | `SGWAnimBlendByCover_TickAnim` | Iterates entity cover slots via FUN_00e71880/FUN_00e718b0; finds max cover side (slot+0x14); sets blend child = maxCoverSide+1. Source: `SGWAnim_BlendByCover.cpp`. |

### USGWAnim_SequenceListPlayer — scripted multi-step animation (session 5)

| Address | Function | Notes |
|---------|----------|-------|
| `0x00e960c0` | `SGWAnimSequenceListPlayer_IsSequenceActive` | Checks actor's class vs sequence player base class. |
| `0x00e96140` | `SGWAnimSequenceListPlayer_PlayNextSequence` | Validates list non-empty; calls SetAnimSequence for current entry; checks match, advances index/loop counter. Source: `SGWAnim_SequenceListPlayer.cpp:0x25`. |
| `0x00e96200` | `SGWAnimSequenceListPlayer_StartSequenceList` | Resets playback to entry 0; calls FUN_008027d0 to set first sequence. Source: `SGWAnim_SequenceListPlayer.cpp:0x38`. |
| `0x00e962d0` | `SGWAnimSequenceListPlayer_TickAnim` | After base tick: if animation finished and no loops pending, advances to next sequence in list (stride 0x0C per entry). Source: `SGWAnim_SequenceListPlayer.cpp`. |

**SequenceListEntry layout (stride 0x0C):**

| Offset | Type | Name | Notes |
|--------|------|------|-------|
| `+0x00 byte 0` | `byte` | flags | bit 0 = loop, bit 1 = blend flag |
| `+0x04` | `int` | sequenceNameIdx | FName index for sequence name |
| `+0x08` | `int` | groupNameNum | FName number for group |
| this+0x11C | `ptr` | pSequenceList | List base pointer |
| this+0x120 | `int` | nSequenceCount | Total entries |
| this+0x128 | `int` | nLoopCountdown | Remaining loop repeats |
| this+0x12C | `int` | nCurrentIdx | Current playback position |

### USGWAnim_MaskChooser — upper/lower body layer mask (session 5)

| Address | Function | Notes |
|---------|----------|-------|
| `0x00e93330` | `SGWAnimMaskChooser_IsChildOfClass` | Class hierarchy check for MaskChooser. |
| `0x00e93400` | `SGWAnimMaskChooser_TickAnim` | Checks aim/override state (this+0xE8 via FUN_00e93370); selects child 1 (upper body override) when aim active, child 0 (lower body) otherwise. Blend time from this+0x118. Source: `SGWAnim_MaskChooser.cpp`. |

### Reload / Event-Set Pipeline (issue #210)

| Address | Function | Notes |
|---------|----------|-------|
| `0x00cbcda0` | `Event_NetOut_RequestReload` ctor | Pattern B: `NetworkEvent_Ctor` + `Event_NetOut_RequestReload::vftable`. Field: `aReloadType` (int). |
| `0x00c889a0` | SGWTextCommandManager reload handler | `SGWTextCommandManager.cpp:0xB4B`. Reads `reloadType` from CME event, creates `Event_NetOut_RequestReload`, dispatches via `FUN_00caf850`. |
| `0x00cbd610` | `Event_NetOut_LoadBehavior` ctor | Pattern B: `NetworkEvent_Ctor` + `Event_NetOut_LoadBehavior::vftable`. |
| `0x00c891b0` | SGWTextCommandManager behavior-event-set handler | `SGWTextCommandManager.cpp:0xB84`. Reads `behaviorEventSetId`, creates `Event_NetOut_LoadBehavior`, field: `aBehaviorEventSetId`. |
| `0x00e6fd20` | `GameEntity::onKismetEventSetUpdate` handler | `GameEntity.cpp:0x149`. Reads `kismetEventSetId`, stores at `this+0x98`, calls `FUN_00d29c90` for ZipStorage lookup. **CRITICAL: server must send kismetEventSetId = item archetype's ItemEventSet entry ID, not ability event set ID.** |
| `0x00d88d80` | `register_NetIn_onKismetEventSetUpdate` | Returns `"Event_NetIn_onKismetEventSetUpdate"`. |
| `0x015d47c0` | Item archetype serializer | `FUN_015d36f0(param_1, "ItemEventSet", -1, param_4+0xC)` — `ItemEventSet` is at archetype `+0x0C`. This is read-time data; server must use this to select the correct kismetEventSetId. |

### CME Event Registration — Animation-related NetIn signals

| Address | Name | Role |
|---------|------|------|
| `0x00d88d80` | `register_NetIn_onKismetEventSetUpdate` | Returns `"Event_NetIn_onKismetEventSetUpdate"` |

## Respawn Lifecycle (W-respawn-lifecycle, session 7)

See [`findings/respawn-lifecycle.md`](findings/respawn-lifecycle.md) for full evidence trail.

### Death + BSF_Dead Handling

| Address | Function | Notes |
|---------|----------|-------|
| `0x00e01c90` | `GameBeing_OnStateFieldUpdate` | XOR-delta dispatch; routes BSF_Dead change to OnDeadStateChanged (documented W-state) |
| `0x00e6e330` | `GameBeing_OnDeadStateChanged` | BSF_Dead handler: toggles interaction type, fires Event_Entity_InteractionUpdate |
| `0x00dff610` | `GameBeing_GetInteractionConfig` | Returns 4-slot interaction descriptor; BSF_Dead (pThis+0x158 bit 0) gates loot vs. live interaction |
| `0x00e791d0` | `GameBeing_ApplyDeadInteraction` | UI only: applies death color tint to CharacterName widget via "Color" field |
| `0x00e68570` | `GameBeing_RightClickGatePredicate` | Right-click gate; reads pThis+0x158 bit 0 (documented W-right-click-routing) |

Key data globals:
- `0x0185d37c` — loot/dead interaction descriptor handle (returned when BSF_Dead is set)
- `0x017f7ea0` — default/talk interaction descriptor handle
- `0x0185d374` — secondary NPC interaction handle (case 2 in entity-class switch)

**Corpse model confirmed**: NO separate corpse entity spawned. Original entity transitions in-place via BSF_Dead flag flip. Ragdoll via kismet Entity_Death sequence (event 5001).

### onBeginAidWait — Defeat Window Parse

| Address | Function | Notes |
|---------|----------|-------|
| `0x00cc2eb0` | `SGWScriptedWindow_ParseBeginAidWaitEvent` | Parses onBeginAidWait: reads TimeToAid + respawner array (respawnerID + respawnerName per entry), pushes to Lua |
| `0x00cea4a0` | `SGWScriptedWindow_OnBeginAidWait_Dispatch` | Dispatch thunk: reads pThis+0x8/0xc/0x10, calls ParseBeginAidWaitEvent |

Field name strings embedded in binary:
- `0x019B4670` — `"TimeToAid"`
- `0x019B467C` — `"respawners"`
- `0x019B4688` — `"respawnerID"`
- `0x019B4694` — `"respawnerName"`

Wire format for `onBeginAidWait` (method 98):
```
[INT32  TimeToAid]       // server sends 30 seconds
[UINT32 array_count]
([INT32 respawnerID] [WSTRING respawnerName]) × array_count
```

### Client → Server Respawn Messages

| Address | Function | Notes |
|---------|----------|-------|
| `0x00aa1c00` | `Lua_callForAid` | Lua C binding (__cdecl, returns int); validates 2 args, reads respawnerID, calls EmitNetOut_callForAid |
| `0x00aea880` | `EmitNetOut_callForAid` | Pattern-B emitter; field "respawnerID" → "aRespawnerMobID"; cell method 67 (CALL_FOR_AID) |
| `0x00c6fc40` | `RouteOutgoingEntityRpc` | Universal outgoing RPC router (documented W-world-entry) |

Full wire path: `PlayerDefeat.lua::callForAid(respawnerID)` → `Lua_callForAid` → `EmitNetOut_callForAid` → `Event_NetOut_callForAid` CME → `RouteOutgoingEntityRpc` → cell method 67.

Auto-respawn (method 70 RESPAWN): client sends with no args after TimeToAid expires; same server code path with respawner_id = -1.

### Server → Client GiveRespawner

| Address | Function | Notes |
|---------|----------|-------|
| `0x00c81430` | `EmitNetOut_GiveRespawner` | Pattern-B emitter; reads "RespawnerMobId" from entity data; sets "aRespawnerMobID"; dispatches via FUN_00cace50 |
| `0x00cb7ee0` | `EventNetOut_GiveRespawner_Ctor` | 12-byte event ctor; installs Event_NetOut_GiveRespawner::vftable (0x019B37E0); double-overwrite MSVC pattern |
| `0x019B37E0` | `Event_NetOut_GiveRespawner::vftable` | Final vtable for GiveRespawner event objects |

### Respawner Selection and Execution

No binary evidence of per-player respawner unlock gating found in SGW.exe (issue #233 open). Current implementation uses global `Vec<RespawnerDef>` filtered only by `world_name`.

Server-side priority (from `crates/services/src/cell/cell_methods/player/combat/respawn.rs`):
1. Explicit `respawner_id > 0` from CALL_FOR_AID
2. First `RespawnerDef` matching entity's world
3. Castle default: `"Castle_CellBlock"` at `[-334.231, 73.472, -228.026]`
4. In-place at current position (warn log)

Same-world: `CellToBaseMsg::ReanchorPlayer` — no RESET_ENTITIES; stats reset, flags cleared, `CREATE_BASE_PLAYER` + viewport + `CREATE_CELL_PLAYER` + `FORCED_POSITION` + appearance replay.

Cross-world: `CellToBaseMsg::GateTravel` — full instance teardown.

## Build Environment

| Field | Value |
|-------|-------|
| QA Build Path | `c:\BUILD\QA\SGW\Working\Development\Src\Core\Inc\FMallocCME.h` |
| Perforce Workspace | `F:\perforce3\SGW\` (from Launcher.exe strings) |
| Compiler | MSVC 8.0 (Visual C++ 2005) |
| Static Libraries | libcurl 7.17.0, gSOAP 2.7, OpenSSL, Intel TBB |
| PE Characteristics | LARGE_ADDRESS_AWARE (0x0122), image base 0x00400000 |

## String Table Locations

| Address Range | Content | Count |
|---------------|---------|-------|
| TODO | Event_NetOut_* strings | 479 |
| TODO | Event_NetIn_* strings | 496 |
| TODO | RTTI type descriptors | ~9,700 |
| TODO | BigWorld source paths | ~200+ |
| TODO | CME:: debug strings | ~100+ |

---

## Cover System

See [`findings/cover-system.md`](findings/cover-system.md) for full analysis.

### Cover Weight Event Handlers (Client → Server)

| Address | Function | Notes |
|---------|----------|-------|
| `0x00c87430` | `SGWTextCommandMgr_OnChangeCoverWeight` | `/changecoverweight` handler — reads `distance,defCover,offCover,move,crossPath,cover` floats, constructs `Event_NetOut_ChangeCoverWeight` (0x0C bytes), dispatches via CME. Source: `SGWTextCommandManager.cpp` L0xB15–0xB1A |
| `0x00c87d00` | `SGWTextCommandMgr_OnChangeCoverStanceWeight` | `/changecoverstanceweight` — same 6 floats + `stance` wstring. Source: L0xB32–0xB38 |
| `0x00cbcaa0` | `register_NetOut_ChangeCoverWeight` | Returns string `"Event_NetOut_ChangeCoverWeight"` |
| `0x00cbcc50` | `register_NetOut_ChangeCoverStanceWeight` | Returns string `"Event_NetOut_ChangeCoverStanceWeight"` |
| `0x00cbc8f0` | `register_NetOut_RegenerateCoverLinks` | Returns string `"Event_NetOut_RegenerateCoverLinks"` |
| `0x00cbca40` | `Event_NetOut_ChangeCoverWeight_Ctor` | 0x0C-byte NetworkEvent ctor (Pattern B), stamps `Event_NetOut_ChangeCoverWeight::vftable` |

### CoverInfo Client Object

| Address | Function | Notes |
|---------|----------|-------|
| `0x00e73280` | `CoverInfo__vfunc_0` | MSVC scalar destructor for `CoverInfo` |
| `0x00e71c30` | `CoverInfo_Dtor` | Dtor body — unsubscribes FCallbackEventDevice slots 0x2f (`Entity_Destroyed`) and 0x30 (`Entity_ProxyPlayerCellCreated`); cleans up spatial tree list at `this+0x4` |
| `0x00e71710` | `CoverInfo_UpdateCover` | Core cover update — checks if player moved > threshold `DAT_018fde24`, queries CoverSpace spatial tree at `this+0x10`, dispatches sorted candidate list |
| `0x00aebd30` | `CoverInfo_LuaUpdateCoverImpl` | Gets player's `GameBeing` via entity manager, calls `CoverInfo_UpdateCover` |
| `0x00aa0d10` | `Lua_updateCover` | Lua C binding for `updateCover()` — 2 args, returns bool |
| `0x00e72ab0` | `CoverInfo_GetSpatialNode` | Queries CoverSpace tree node by entityID and chunk |

### Cover Node Loading

| Address | Function | Notes |
|---------|----------|-------|
| `0x010556a0` | `CoverNodeXmlLoader` | Parses `SGWCoverSet` XML — keys: `entity`, `SGWCoverSet`, `coverSet`, `coverNodes`, `coverNode`, `transform`, `coverheight`, `coverquality`, `coverwidth`, `orientation`; orientation converted by `* DAT_018f41c0 * DAT_01816ac8` |
| `0x00904d80` | `USGWCoverNodeComponent_SpawnCoverNode` | Spawns UE3 actor for a cover node prefab; loads `"SGW_Cover.CoverNode"` static mesh; CoverNodePrefabData stride = 0x18 bytes; coverHeight enum switch → float constants at `DAT_018f41d4/d0/cc/c8` |
| `0x00903a10` | `USGWCoverNodeComponent__vfunc_0` | Returns 1 (stub/identity) |
| `0x01605ef0` | `CME_FindCoverNodeCentroid__vfunc_0` | Cover node centroid finder |
| `0x01605f10` | `CME_CoverNodeVariance__vfunc_0` | Cover node variance calculator |

### ACoverLink UE3 Functions

| Address | Function | Notes |
|---------|----------|-------|
| `0x00704be0` | `ACoverLink__vfunc_183` | `GetSlotActions` — enumerates slots (stride 0x9c), collects FireLinks, DefinedPaths, CoverLinks into pointer array |
| `0x006ff840` | `ACoverLink__vfunc_73` | `GetSlotViewPoint` — returns slot position from `this+0xdc/e0/e4`; adds PawnOwner adjustment via vtable +0x380 if present |
| `0x00700800` | `ACoverLink__vfunc_205` | `HasFireLinkTo` / `IsValidClaim` — slot = `param_1*0x9c + this+0x28c`; linear scan DefinedPaths for matching target+dir; returns `pathCount > 0` |

## Ability Resolution Pipeline (W-abilities session, 2026-05-13)

See [`findings/ability-resolution-pipeline.md`](findings/ability-resolution-pipeline.md) for full analysis.

### Call Trace: Button Press → Network Emit

| Address | Function | Notes |
|---------|----------|-------|
| `0x00aa2910` | Lua `useAbility` thunk | Entry point from Lua VM → forwards to AbilitySet_InvokeAbility. Error string at `0x01940b70`: `"#ferror in function 'useAbility'"` |
| `0x00d2a000` | `AbilitySet_GetSlotByIndex` | Looks up AbilitySlot pointer by zero-based index; returns null if out of range |
| `0x00d2ae40` | `AbilitySet_EmitUseAbilityOrGroundTarget` | Branch on targetType (slot+0x48): 3=TargetGround → reticle flow; else → Pattern B emit for `Event_NetOut_UseAbility` |
| `0x00dea330` | `AbilitySet_ActivateGroundTargetReticle` | Asserts TCM_AERadius==2 AND TargetGround==3; shows AE reticle; subscribes to `Event_Player_GroundTargetingEnd` |
| `0x00d29d40` | `AbilityInfo_GetAERadius` | Returns float at param_1+0xa0; asserts TCM==TCM_AERadius(2). Effective score 87. |

### AbilityType Struct — PAK/Serialized Layout

| Address | Function | Notes |
|---------|----------|-------|
| `0x015d51c0` | `AbilityType_DeserializePak` | Reads PAK stream into AbilityType. Key offsets: +0x34=WarmupSec, +0x38=CooldownSec, +0x44=TCM, +0x48/4C=TCM_Param1/2, +0x54/58=Flags, +0x60/64=Min/MaxRange. EffectIds at param_4+4 via FUN_015d3e60. |
| `0x00adb670` | `AbilityType_GetLuaAbilityInfo` | Runtime Lua layout. Offsets differ from PAK: +0x50=warmUp, +0x54=coolDown, +0x60=icon, +0x94=TCM, +0x98=flags (bit0=weapon, bit1=deploy, bit16=pet). |

### ETargetCollectionMethod (TCM) and ETargetType Enum Values (confirmed from asserts)

| Value | Enum | Confirmed at |
|-------|------|--------------|
| 2 | `TCM_AERadius` | Assert in `AbilityInfo_GetAERadius` (0x00d29d40) |
| 3 | `ETargetType::TargetGround` | Assert in `AbilitySet_ActivateGroundTargetReticle` (0x00dea330) |

### onEffectResults Handler

| Address | Function | Notes |
|---------|----------|-------|
| `0x00eb1630` | `CombatQueue_HandleOnEffectResults` | Subscriber to `Event_NetIn_onEffectResults`. Source: `Src\CombatQueue.cpp` lines 0x2b-0x54. Wire: 21+7×N bytes. Visibility filter: skips if neither SourceID nor TargetID is local player/target. |

### Active Effect Query

| Address | Function | Notes |
|---------|----------|-------|
| `0x00aec290` | `EffectType_GetLuaEffectInfo` | Lua query for active effect slot (1-based index). Writes Name/Description/IconLoc/TCM/Beneficial/Hidden/Channeled/Flags/TotalTime/TimeRemaining. `Channeled` bool from FUN_00d2d0e0 — drives cast-bar cancel UI. |

### Channeled Ability Cancellation

| Address | Function | Notes |
|---------|----------|-------|
| `0x00c8c820` | `SGWTextCommandMgr_HandleConfirmEffect` | Reads EffectId(int)+Response(byte). Emits `Event_NetOut_ConfirmEffect` with fields `"aEffectId"` (int) and `"aAccepted"` (bool: Response==1). Only path for channeled ability cancel. |

### Timer Handlers — Event_NetIn_TimerUpdate

All five handlers subscribe to the same `Event_NetIn_TimerUpdate` signal; dispatch by the `Type` byte.

| Address | Function | Timer Types | Notes |
|---------|----------|-------------|-------|
| `0x00ea6af0` | `CooldownManager_HandleOnTimerUpdate` | 0,1,2,3 | Gate: SourceID != entityId → return. Calls FUN_00ea62b0 which fires `Event_UI_AbilityCooldown`. UI subscriber at `0x01e0c458`. |
| `0x00e09160` | `EffectSet_HandleOnTimerUpdate` | 5 | Reads SecondaryId+BigWorldTimeComplete+TotalTime; updates active-effect duration window. |
| `0x00d18a30` | `MissionSet_HandleOnTimerUpdate` | 9,10,11 | Types: `\t`=reset, `\n`=progress (walks this+0x58), `\v`=completion (walks this+0x64). Final: FUN_00d16dd0. |
| `0x00e02380` | `GameBeing_HandleOnTimerUpdate_Reload` | 12,13 | `\f`(12)→Event_UI_EntityReload; `\r`(13)→Event_UI_EntityReloadDeployment. Remaining=BigWorldTimeComplete−FUN_00c6e220(). |
| Unknown | GameProxyPlayer timer handler? | 4,6,7,8? | 5th subscriber not yet identified. |

### Globals (pending W0 rename)

| Address | Proposed name | Evidence |
|---------|---------------|---------|
| `0x01e0c458` | `g_AbilityCooldown_UISubscriber` | Subscriber to `Event_UI_AbilityCooldown` fired by CooldownManager; observed in xref trace from FUN_00ea62b0 |
| `0x00708e10` | `ACoverLink__vfunc_161` | `PostEditChange` / cover link validation; logs warning string at `0x018830bc` if slot has no CoverType; checks `ForceCoverType` |

### CoverSpace Spatial Tree

| Address | Function | Notes |
|---------|----------|-------|
| `0x01608c50` | `CoverSpace_LogTreeStats` | Logs tree statistics: nodes, K, MaxDepth, MaxCoverNodes, Branches, Leaves, Overlap %. Uses std::list iteration. |
| `0x01608820` | `CoverSpace_GetFactory` | (inferred) Creates/gets CoverSpace singleton |
| `0x0160c3b0` | `CoverSpace_GetChunkTree` | (inferred) Gets spatial tree for given chunk |
| `0x01609760` | `CoverSpace_QueryAtPosition` | (inferred) Queries candidates near a 3D position |

### Cover-Related RTTI / String Addresses

| Address | Value | Notes |
|---------|-------|-------|
| `0x018f3f1c` | `"SGW_Cover.CoverNode"` | Static mesh path for cover node actor |
| `0x017f9bb8` | `"covernodes_local.pak"` | Cover node archive filename |
| `0x017f9b80` | `"MyCoverNodeArchive"` | Cover node archive class name |
| `0x019d2ca4` | `"Entity: %d is moving to cover"` | Debug string for movement type 0 (referenced in `FUN_00deb660`) |
| `0x01b1a5f4` | `"publicReservationData"` | Property name for `SGWCoverSet.publicReservationData` |
| `0x018830bc` | `"%s Slot %d has not CoverType"` | Warning when slot has no cover type assigned |

---

## NPC Movement / Pathfinding (Session 4 — W-path)

See [`findings/npc-movement-pathfinding.md`](findings/npc-movement-pathfinding.md) for full analysis.

Key finding: No dedicated CME move-emitter functions exist. Server streams position via BigWorld avatarUpdate wire (msg 0x10–0x2F / 0x30–0x31). Client receives via `onEntityMoveWithError`, converts BW→UE3 (×100 + axis swap), and renders via `GameEntityBase::ApplyTransform` with optional physics interpolation.

**Server gap (Cimmeria)**: `npc_ai_leash()` snaps NPC to spawn instantly without sending `movementType=2` + waypath. `npc_movement_tick()` sends raw AoI position updates without `movementType=1` (CombatAdvance) + path payload.

### Wire-to-UE3 conversion (confirmed in 0x00dd1650)

| Constant | Address | Value |
|----------|---------|-------|
| `BW_TO_UE3_SCALE` | `0x018cad90` | `100.0f` (BW meters → UE3 cm) |
| Position sentinel | `DAT_019d1a44` | FLT_MAX / ∞ ("use current component") |

Axis swap: `UE3_X = BW_Z × 100`, `UE3_Y = BW_X × 100`, `UE3_Z = BW_Y × 100`

### AI Movement FSM (7 states — jump table 0x00dec018 inside 0x00deb660)

| State | Index | Name |
|-------|-------|------|
| CoverAdvance | 0 | Move toward cover node |
| CombatAdvance | 1 | Move toward combat target |
| Leash | 2 | Return to spawn point |
| Patrol | 3 | Follow patrol route |
| Follow | 4 | Follow player/squad leader |
| Wander | 5 | Random idle wander |
| Avoid | 6 | Obstacle/collision avoidance |

### NPC Movement Functions

| Address | Function | Notes |
|---------|----------|-------|
| `0x00dd1650` | `EntityManager::onEntityMoveWithError` | Wire → UE3 coordinate conversion; BW→UE3 scale + axis swap + sentinel handling; delegates to ApplyTransform |
| `0x00dd19e0` | `GameEntityManager_UpdateControlledEntityTransform` | Player-controlled entity transform push |
| `0x00deb660` | `MovementTypeSwitch` | 7-state AI FSM dispatcher; 610 instructions; jump table at `0x00dec018`. Too large to decompile (timeout). Key debug strings inside: "is moving to cover", "is making a combat advance", "is leashing" |
| `0x00dec018` | AI movement jump table | Cases 0–6 for the 7 movement states |
| `0x00deaaf0` | `onPositionUpdate` | BigWorld position update callback. Also **allocates new UE3 actors** for path waypoint visualization (not just updates position) |
| `0x00dec040` | `PathDestroy` | Fires on path completion/cancellation; destroys path-visualization actors by `wcsicmp` name match |
| `0x00dec6d0` | `onSquadList` | Squad-member path data receiver |
| `0x00dec9e0` | `onBigWorldTimeComplete` | BigWorld time-sync callback |
| `0x00dedf30` | `TickUpdate` | Per-tick movement advance (advances entity along waypath) |
| `0x00def320` | `ApplyTargetChange` | Target acquisition / heading update |
| `0x00df08c0` | `TargetIDReceiver` | CME NetIn target-id event receiver |
| `0x00df3550` | `RegionUpdate` | BigWorld space/region change callback |
| `0x00df3ab0` | `SGWBeing_RegisterCallbacks` | Registers TickUpdate + onPositionUpdate + MovementTypeSwitch + PathDestroy + RegionUpdate for SGWBeing |
| `0x00df3cc0` | `SGWMob_RegisterCallbacks` | Identical callback set to SGWBeing (confirmed by decompile) |
| `0x00e68a30` | `GameEntityBase::ApplyTransform` | Routes to: Path A (direct write, force flag), Path B (vehicle interpolator at entity+0xe4), Path C (physics interpolator at entity+0x1d0) |
| `0x00e688c0` | `EntityVisibilityManager` | Distance-cull / LOD management for entities |
| `0x00e69690` | `EntityInterpolatorUpdate` | Physics interpolator dispatch: calls `FUN_0049ffb0` then `(*interpolator + 0xe8)` |


## Character Creation Pipeline (Session 4b — W-character-creation)

See [`findings/character-creation-pipeline.md`](findings/character-creation-pipeline.md) for full analysis.

### CreateCharacter — Client → Server emit path

| Address | Function | Notes |
|---------|----------|-------|
| `0x00d32ce0` | `EmitNetOut_CreateCharacter` | Main emit: CharName, ExtraName, CharDefId, SkinTintColorID (this+0x40), VisualChoiceList |
| `0x00d37010` | `Event_NetOut_CreateCharacter_Ctor` | 12-byte Pattern B ctor; stamps `Event_NetOut_CreateCharacter::vftable` |
| `0x00d328f0` | `BuildVisualChoiceList` | Iterates VisualGroup vector (stride=0x34, 0xD uint32s) → VisGroupId+ChoiceId pairs in CME PropertyList |
| `0x00d34d30` | `VisualChoiceVector_GetAt` | Array-of-structs accessor, stride=0x24; used by BuildVisualChoiceList |
| `0x00d39160` | `AppearanceChain_LookupRaceNode` | Race-keyed BST lookup; inserts new node if not found |
| `0x00d388d0` | `AppearanceChain_LookupArchetypeNode` | Archetype-keyed BST lookup (same pattern) |
| `0x00d37f70` | `AppearanceChain_LookupVisualGroupNode` | VisualGroup-keyed BST lookup; node sentinel at +0x71 |
| `0x00d32370` | `CreateCharacter_PostEmitReset` | Post-dispatch cleanup: iterates and fires pending subscription list, resets linked-list sentinel |
| `0x00d67970` | `SGWNetworkManager_VEvent_NetOut_CreateCharacter___EventHandler__vfunc_0` | SGWNetworkManager dispatch stub → FUN_00d55bd0 → MemberCallback |
| `0x00d37070` | `register_NetOut_CreateCharacter` | Returns `"Event_NetOut_CreateCharacter"` |

### onCharacterList — Server → Client receive path

| Address | Function | Notes |
|---------|----------|-------|
| `0x00e74060` | `GameAccount_HandleNetIn_CharacterList` | Processes onCharacterList: builds 0xC0-byte CharacterInfo structs, deduplicates by "name-extraName" key |
| `0x00d78980` | `register_NetIn_CharacterList` | Returns `"Event_NetIn_CharacterList"` |
| `0x00d78ec0` | `register_NetIn_CharacterCreateFailed` | Returns `"Event_NetIn_CharacterCreateFailed"` |

### onCharacterVisuals — Server → Client receive path

| Address | Function | Notes |
|---------|----------|-------|
| `0x00e74f50` | `GameAccount_HandleNetIn_CharacterVisuals` | Processes onCharacterVisuals: reads BodySet, Components[], primaryTint/secondaryTint/skinTint (0xRRGGBB00 packed) → RGBA float tuples at CharacterInfo+0x90/+0xA0/+0xB0 |
| `0x00e6f8b0` | `GameEntity_ApplySkinTintColors` | Reads primaryColorId/secondaryColorId/skinColorId from entity event, unpacks 0xRRGGBB00 → RGBA. Source: `GameEntity.cpp:0x194-0x196` |
| `0x00d78c20` | `register_NetIn_CharacterVisuals` | Returns `"Event_NetIn_CharacterVisuals"` |
| `0x00d9ae80` | `register_NetOut_RequestCharacterVisuals` | Returns `"Event_NetOut_RequestCharacterVisuals"` |

### playCharacter / deleteCharacter

| Address | Function | Notes |
|---------|----------|-------|
| `0x00e755b0` | `GameAccount_EmitNetOut_PlayCharacter` | Emits `playCharacter` (single INT32 `PlayerId`); guards on `CharacterInfo+0x64` (playable flag) |
| `0x00e756e0` | `GameAccount_EmitNetOut_DeleteCharacter` | Emits `deleteCharacter` (single INT32 `PlayerId`); no playable-flag check |
| `0x00d9ab80` | `Event_NetOut_PlayCharacter_Ctor` | 12-byte ctor; stamps `Event_NetOut_PlayCharacter::vftable` |
| `0x00d9a3a0` | `Event_NetOut_DeleteCharacter_Ctor` | 12-byte ctor; stamps `Event_NetOut_DeleteCharacter::vftable` |
| `0x00d9abe0` | `register_NetOut_PlayCharacter` | Returns `"Event_NetOut_PlayCharacter"` |
| `0x00d9a400` | `register_NetOut_DeleteCharacter` | Returns `"Event_NetOut_DeleteCharacter"` |
| `0x00d79160` | `register_NetIn_onCharacterLoadFailed` | Returns `"Event_NetIn_onCharacterLoadFailed"` |

### EntityDef serializers for character-creation types

| Address | Function | Notes |
|---------|----------|-------|
| `0x015d4570` | (EntityDef: CharacterDefinition serializer) | Writes CharDefId, AlignmentId, ArchetypeId, GenderId, BodySet, VisualGroups[] |
| `0x015d4660` | (EntityDef: VisualGroup serializer) | Writes VisGroupId, VisType (wstring, optional), Choices[] |
| `0x015ce700` | (EntityDef: VisualChoice serializer) | Writes ChoiceId, Component (wstring, optional), ItemId |

### CharacterInfo struct (0xC0 bytes) — key field offsets

| Offset | Type | Field |
|--------|------|-------|
| +0x00 | uint32 | playerId |
| +0x04 | wstring | name |
| +0x20 | wstring | extraName |
| +0x3C | byte | alignment (0-5) |
| +0x3D | byte | level (0-20) |
| +0x3E | byte | gender (1-3) |
| +0x3F | byte | archetype (0-7) |
| +0x40 | wstring | worldLocation |
| +0x5C | byte | title |
| +0x60 | uint32 | playerType |
| +0x64 | byte | playable (0=already in world) |
| +0x68 | wstring | bodySet |
| +0x84 | vector | components (wstring[]) |
| +0x90 | float[4] | primaryTint (RGBA) |
| +0xA0 | float[4] | secondaryTint (RGBA) |
| +0xB0 | float[4] | skinTint (RGBA) |

---

## Mission State Machine (Session 4b — W-mission-state)

See [`findings/mission-state-machine.md`](findings/mission-state-machine.md) for full analysis, data structure layouts, and open questions.

### MissionSet Core Handlers

| Address | Function | Notes |
|---------|----------|-------|
| `0x00d1a270` | `MissionSet_HandleOnMissionUpdate` | Main mission update: lookup/allocate MissionEntry, route Status=1→ActivateMission, Status≠1→Propagate; fires UI tokens 0x138F/0x1393 |
| `0x00d18cf0` | `MissionSet_HandleOnStepUpdate` | Step update: searches step list at `this+0x58`, writes status → step+0x3C; fires UI token 0x1390 on Status=1 |
| `0x00d18fd0` | `MissionSet_HandleOnObjectiveUpdate` | Objective update: writes status → objective+0x30, optional → objective+0x34; fires UI tokens 0x1391/0x1392 |
| `0x00d194b0` | `MissionSet_HandleOnTaskUpdate` | Task update: searches task list at `this+0x70`, writes status → task[2], count → task[3]; calls PropagateMissionUpdate |
| `0x00d1a500` | `MissionSet_HandleMissionRewards` | Reward delivery: reads full Rewards{XP, Naquadah, ItemGroups[]} struct; subscribes to Cache_ElementReady<DBInvItem> for deferred display |
| `0x00d19a40` | `MissionSet_HandleMissionOffer` | Mission offer handler: reads MissionID from event+0x0C, constructs outbound NetworkEvent, zeroes this+0x9C/0xA0/0xA4/0xA8 (load-state reset) |
| `0x00d19870` | `MissionSet_HandleMissionSharedOffer` | Shared offer display: reads MissionId, shows "/sharemissionaccept or /sharemissiondecline" hint |
| `0x00d18a30` | `MissionSet_HandleTimerUpdate` | Timer routing: type byte 0x09=countdown, 0x0A=step timer (searches this+0x58), 0x0B=mission timer (searches this+0x64); calls FUN_00c6d1c0 + PropagateMissionUpdate |

### MissionSet Infrastructure

| Address | Function | Notes |
|---------|----------|-------|
| `0x00d163e0` | `MissionSet_FireUiEvent` | Token-indexed UI toast dispatch; token values are string table IDs |
| `0x00d16800` | `MissionSet_FindMissionById` | Lookup MissionEntry by INT32 ID in internal map |
| `0x00d16dd0` | `MissionSet_PropagateMissionUpdate` | State cascade: walks task→objective→step→mission hierarchy, fires UI updates |
| `0x00d17c10` | `MissionSet_ActivateMission` | First-step activation on mission accept/re-accept |
| `0x00d1e030` | `MissionSet_AllocateMissionEntry` | New MissionEntry allocation + initial field init |
| `0x00daa680` | `MissionSet_SubscribeOnMissionUpdate` | Startup registration for onMissionUpdate in CME signal table |
| `0x00d9fc00` | `MissionSet_onMissionUpdate_Subscriber` | Event factory wrapper: allocates 0xC-byte NetworkEvent, calls Event_NetIn_onMissionUpdate ctor |
| `0x00db3390` | `RegisterBulkNetOutSignals` | Bulk startup registration for all CME events (~64KB function, not fully decompiled) |

### CME Field Reader Functions (shared across all mission handlers)

| Address | Function | Notes |
|---------|----------|-------|
| `0x00e3cba0` | (CME INT32 field reader) | Reads named INT32 field from CME event object |
| `0x00d434d0` | (CME INT8 field reader) | Reads named INT8/byte field from CME event object |
| `0x00e3cc20` | (CME pointer/bool field reader) | Reads named pointer or bool field from CME event object |
| `0x00cb1f00` | `CmeEventSignal_SetFieldHelper` | Sets named field on outbound CME event object |

### UI Token Table (Mission State)

| Token (hex) | Token (dec) | Semantic |
|-------------|-------------|---------|
| `0x138F` | 5007 | Mission Removed/Failed |
| `0x1390` | 5008 | Step Activated ("Mission Advance") |
| `0x1391` | 5009 | Objective Unlocked |
| `0x1392` | 5010 | Objective Removed |
| `0x1393` | 5011 | Mission Accepted |

### MissionSet Object Field Layout (recovered)

| Offset | Type | Name | Evidence |
|--------|------|------|----------|
| `+0x58` | list* | stepList | Accessed in onStepUpdate + TimerUpdate (type 0x0A) |
| `+0x64` | list* | missionList | Accessed in onMissionUpdate + TimerUpdate (type 0x0B) |
| `+0x70` | list* | taskList | Accessed in onTaskUpdate |
| `+0x7C` | TimerState* | timerState | Passed to FUN_00c6d1c0 in TimerUpdate |
| `+0x9C`–`+0xA8` | DWORD[4] | missionLoadState | Zeroed in MissionOffer handler; semantics TBD |

### Wire Format Cross-Reference

Task primitive type names (KillCount, CollectItem, VisitRegion, TalkToNpc, UseObject, Timer) are **not present in SGW.exe as strings** — they exist only in server-side Python/PAK data. The client tracks progress numerically via Count + Status only.

The `onTaskUpdate` handler (`FUN_00d194b0`) reads `Count` as INT32 via `FUN_00e3cba0`, but `alias.xml` defines `MissionTaskStatus.count` as INT8. **Potential wire-format mismatch — verify Rust serializer encoding.**

---

## How to Update

When you identify a key address in Ghidra:
1. Replace the relevant TODO with the actual address (e.g., `0x14001a2b0`)
2. Update the "Decompiled?" column if you've analyzed the function
3. Add new rows for important discoveries
4. Cross-reference with findings in `docs/gameplay/` and `docs/protocol/` docs

## Faction / Alignment System (W-faction session, 2026-05-13)

See [`findings/faction-alignment-system.md`](findings/faction-alignment-system.md) for full analysis.

### CME event registration stubs

| Address | Function | Notes |
|---------|----------|-------|
| `0x00d86b60` | `register_NetIn_onAlignmentUpdate` | Returns `"Event_NetIn_onAlignmentUpdate"` |
| `0x00d86e00` | `register_NetIn_onFactionUpdate` | Returns `"Event_NetIn_onFactionUpdate"` |
| `0x00d96a30` | `register_NetOut_GiveFaction` | Returns `"Event_NetOut_GiveFaction"` |
| `0x00d96cd0` | `register_NetOut_SetFaction` | Returns `"Event_NetOut_SetFaction"` |

### `GameBeing` faction/alignment handlers (GameBeing.cpp)

| Address | Function | Notes |
|---------|----------|-------|
| `0x00e02180` | `FUN_00e02180` (onAlignmentUpdate handler) | Reads `"alignment"` INT8 → `this+0x135` (`mAlignment`); `GameBeing.cpp:0x3ef` (line 1007); calls `GameBeing_OnDeadStateChanged` |
| `0x00e02280` | `FUN_00e02280` (onFactionUpdate handler) | Reads `"faction"` INT8 → `this+0x134` (`mFaction`); `GameBeing.cpp:0x3f6` (line 1014); calls `GameBeing_OnDeadStateChanged` |
| `0x00e6e330` | `GameBeing_OnDeadStateChanged` | Visual refresh on entity state change; fires `Event_Entity_InteractionUpdate` via UE3 |

### Slash-command dispatch (faction GM commands)

| Address | Name | Notes |
|---------|------|-------|
| `0x00593600` | `FUN_00593600` (GiveFaction EmitInfo ctor) | Sets `Event_SlashCmd_GiveFaction` RTTI + fields |
| `0x00593810` | `Event_SlashCmd_GiveFaction__vfunc_2` | VTable slot 2 — calls emit helper `FUN_00593710` |
| `0x005936f0` | `Event_SlashCmd_GiveFaction__vfunc_3` | VTable slot 3 — RTTI accessor via `FUN_00a372f0` |
| `0x00593880` | `FUN_00593880` (SetFaction EmitInfo ctor) | Sets `Event_SlashCmd_SetFaction` RTTI + fields |
| `0x00593a90` | `Event_SlashCmd_SetFaction__vfunc_2` | VTable slot 2 — calls emit helper `FUN_00593990` |
| `0x00593970` | `Event_SlashCmd_SetFaction__vfunc_3` | VTable slot 3 — RTTI accessor via `FUN_00a372f0` |
| `0x00c99ea0` | `MemberCallbackRtti_SlashCmd_GiveFaction__SGWTextCommandMgr` | RTTI accessor for SGWTextCommandMgr subscriber |
| `0x00c99f20` | `MemberCallbackRtti_SlashCmd_SetFaction__SGWTextCommandMgr` | RTTI accessor for SGWTextCommandMgr subscriber |
| `0x01843804` | `vtable_Event_SlashCmd_GiveFaction` | SlashCmd event vtable |
| `0x01843820` | `vtable_Event_SlashCmd_SetFaction` | SlashCmd event vtable |

### `GameBeing` field layout (faction/alignment relevant offsets)

| Offset | Field | Notes |
|--------|-------|-------|
| `+0x134` | `mFaction` (INT8) | Set by `onFactionUpdate` handler at `0x00e02280` |
| `+0x135` | `mAlignment` (INT8) | Set by `onAlignmentUpdate` handler at `0x00e02180` |
| `+0x158` | `bStateField` (UINT32) | State flags — BSF_Dead=bit0, BSF_InCombat=bit3, etc. |

### Key data strings

| Address | String | Notes |
|---------|--------|-------|
| `0x019d63ac` | `"faction"` | Property key in `onFactionUpdate` handler |
| `0x019d63c8` | `"aEvent->getProperty<int8>(\"faction\", mFaction)"` | Debug assert string; cites `GameBeing.cpp:0x3f6` |
| `0x019d6364` | `"alignment"` | Property key in `onAlignmentUpdate` handler |
| `0x019d6378` | `"aEvent->getProperty<int8>(\"alignment\", mAlignment)"` | Debug assert string; cites `GameBeing.cpp:0x3ef` |
| `0x019bda0c` | `"Event_NetIn_onAlignmentUpdate"` | Event name string |
| `0x019bda2c` | `"Event_NetIn_onFactionUpdate"` | Event name string |
| `0x019bee58` | `"Event_NetOut_GiveFaction"` | Event name string |
| `0x019bee74` | `"Event_NetOut_SetFaction"` | Event name string |

## Combat Mechanics — Session 5 (2026-05-13)

Full findings in `findings/combat-wire-formats.md` (threat), `findings/effect-execution-model.md`
(stacking/DR), and `findings/combat-damage-analysis.md` (damage pipeline).

### Threat / Aggro Table (Gap 1)

| Address | Function | Notes |
|---------|----------|-------|
| `0x00d8d480` | `register_NetIn_onThreatenedMobsUpdate` | Returns event name string; registration stub |
| `0x00e07da0` | `GetRTTI_Callback_NetIn_onThreatenedMobsUpdate__VGamePlayer` | RTTI subscriber accessor |
| `0x00e07250` | Bulk threatened-mobs list handler | Reads `"aEntityList"` (ARRAY<INT32>); `GamePlayer.cpp:0x69` |
| `0x00e07570` | Per-entity threat add/remove handler | Reads `"EntityId"` (int32) + `"HasThreat"` (uint8); `GamePlayer.cpp:0x91-92` |
| `0x00e071c0` | Threat list clear/reset | Swaps threat container at `GamePlayer+0x170/0x174` |
| `0x00c6bd20` | Sorted-set insert (threat list) | Red-black tree insert keyed on entity ID |
| `0x00e083a0` | Sorted-set remove (threat list) | Removes entity from secondary set at `GamePlayer+0x178` |
| `0x00e07ab0` | Threat list iterator/broadcaster | Walks list at `+0x170/0x174`; calls callback per entity |
| `0x00dd0de0` | `LookupEntityListenerEntry` | Resolves entity ID → `GameEntityBase*` |
| `0x00e6e330` | `GameBeing_OnDeadStateChanged` | Called after threat-list entity lookup |
| `0x00d9bba0` | `register_NetOut_TestLOS` | GM: raycasts two entities; args `INT32 fromId, INT32 toId` |
| `0x00d9be40` | `register_NetOut_ToggleCombatLOS` | GM: toggle combat LOS requirement; arg `UINT8 enabled` |

**GamePlayer threat fields**:
- `GamePlayer+0x16c` — primary threatened-mobs sorted set (bulk `aEntityList` update)
- `GamePlayer+0x170/0x174` — threat list iterator anchors (prev/next pointers)
- `GamePlayer+0x178` — secondary per-entity threat flag set (delta add/remove)

### Effect Stacking + DR (Gap 2)

| Address | Function | Notes |
|---------|----------|-------|
| `0x00e09160` | `EffectSet_HandleOnTimerUpdate` | Timer type 5; reads `SecondaryId/BigWorldTimeComplete/TotalTime` |
| `0x00d2d740` | Active-effect struct constructor | 5 fields: SecondaryId, RefCount, SourceID, TotalTime, BigWorldTimeComplete |
| `0x00e0a620` | Effect lookup-or-insert | Finds by SecondaryId; inserts new if not found (allows stacking) |
| `0x00e0a3b0` | Effect insert path | Builds `"_<SourceID>"` wstring key; no cap check |
| `0x00e0a9e0` | Effect removal/expiry dispatcher | Calls FUN_00e0a6f0 for lookup then FUN_00e0a810 for emit |
| `0x00e0a810` | Effect-removed CME emitter | Emits `"CategoryId"=9` + effect key |
| `0x00c6e220` | BigWorld server clock accessor | Float; current server timestamp |
| `0x00c6d1c0` | Interval-tree updater | Updates `[startTime, endTime]` window at `EffectSet+0x38` |
| `0x015fbd50` | Active-effect vector push_back | Appends new effect instance |
| `0x00e01c90` | `GameBeing_OnStateFieldUpdate` | CC/state bit dispatcher; `GameBeing.cpp:0x341` |
| `0x00e05db0` | `GameBeing_EmitStateFieldChanged` | Fires `Event_Entity_StateFieldChanged` with old/new/delta |

**EffectSet field offsets**:
- `EffectSet+0x10/0x14/0x1c` — active-effect shared-ptr vector
- `EffectSet+0x28` — effect lookup map (searched by SecondaryId)
- `EffectSet+0x38` — interval tree for duration window tracking
- `EffectSet+0x48` — gate flag checked before removal emit

### Damage Pipeline (Gap 3)

| Address | Function | Notes |
|---------|----------|-------|
| `0x00eb1630` | `CombatQueue_HandleOnEffectResults` | Full subscriber; `CombatQueue.cpp:0x2b–0x54` |
| `0x00eb0ef0` | Combat entry struct ctor | 5 fields: SourceID, TargetID, AbilityID, ResultCode, EffectList |
| `0x00eb0f70` | Per-stat entry initializer | 5 fields: StatID, DamageCode, StatResultCode, Delta, RefCount |
| `0x00eb14d0` | CombatQueue drain loop | Drains ring buffer; gates on ability cache; emits text |
| `0x00eb11a0` | Ability-data gate check | `FUN_00ae6b50` ability cache lookup; drops entry if not cached |
| `0x00eb1230` | Combat text event emitter | Allocs 0x14 per entry; `FUN_00eb0a70` → `FUN_00e6beb0` |
| `0x00eb0a70` | CME combat text emitter | `NoSubject` pattern; inserts to list at `CombatQueue+0x14` |
| `0x00be32d0` | Kismet console command parser | Full `TestSequence` keyword→EventID table |
| `0x01e6ce00` | QR result code string table | 20-entry pointer table; UTF-16LE strings |
| `0x019e913c` | String: `ABILITY_INTERRUPT` | Index 0; result code table |
| `0x019e9160` | String: `ABILITY_FAILED` | Index 1 |
| `0x019e9180` | String: `EFFECT_INIT` | Index 2 |
| `0x019e91b8` | String: `EFFECT_HIT_NORMAL` | Index 4 |
| `0x019e91dc` | String: `EFFECT_HIT_CRIT` | Index 5 |
| `0x019e91fc` | String: `EFFECT_HIT_DOUBLE_CRIT` | Index 6 |
| `0x019e922c` | Strings: `EFFECT_HIT_GLANCING`, `EFFECT_HIT_MISS` | Indices 7, 8 |
| `0x019e9274` | String: `EFFECT_PULSE_BEGIN` | Index 9 |
| `0x019e929c` | Strings: `EFFECT_PULSE_END`, `ENTITY_SPAWN` | Indices 10, 11 |
| `0x019e92f4` | Strings: `ENTITY_DEATH`, `ENTITY_ALERT`, `ENTITY_MAKEDEAD` | Indices 12–14 |
| `0x019e92f4+` | Strings: `DESIGNER_1..3` | Indices 15–17 |
| `0x005757f0` | Spectator/debug mode check | CombatQueue visibility filter gate |
| `0x00574430` | Spectator mode flag accessor | Returns bool; filter branch |

---

## Crafting State Machine (W-content-mech Session 5 — 2026-05-13)

See [`findings/crafting-state-machine.md`](findings/crafting-state-machine.md) for full analysis.

| Address | Function | Notes |
|---------|----------|-------|
| `0x00e4a910` | `register_NetOut_Craft` | Returns `"Event_NetOut_Craft"` |
| `0x00e4aac0` | `register_NetOut_Alloy` | Returns `"Event_NetOut_Alloy"` |
| `0x00e4ac70` | `register_NetOut_Research` | Returns `"Event_NetOut_Research"` |
| `0x00e4ae20` | `register_NetOut_ReverseEngineer` | Returns `"Event_NetOut_ReverseEngineer"` |
| `0x00e4afd0` | `register_NetOut_SpendAppliedSciencePoint` | Returns `"Event_NetOut_SpendAppliedSciencePoint"` |
| `0x00aea3d0` | `register_NetOut_RespecCraft` | Returns `"Event_NetOut_RespecCraft"` |
| `0x00d96f70` | `register_NetOut_SetTechSkill` | Returns `"Event_NetOut_SetTechSkill"` |
| `0x00d831a0` | `register_NetIn_onUpdateDiscipline` | Returns `"Event_NetIn_onUpdateDiscipline"` |
| `0x00d83980` | `register_NetIn_onUpdateCraftingOptions` | Returns `"Event_NetIn_onUpdateCraftingOptions"` |
| `0x00d836e0` | `register_NetIn_onUpdateKnownCrafts` | Returns `"Event_NetIn_onUpdateKnownCrafts"` |
| `0x00d7fd00` | `register_NetIn_onCraftingRespecPrompt` | Returns `"Event_NetIn_onCraftingRespecPrompt"` |
| `0x00e45960` | MemberCallback vfunc_3: VCrafting × onUpdateCraftingOptions | RTTI class_SGW::Crafting confirmed |
| `0x00e45a60` | MemberCallback vfunc_3: VCrafting × onUpdateRacialParadigmLevel | **New** — not in wire-formats doc |
| `0x00e45ae0` | MemberCallback vfunc_3: VCrafting × onUpdateDiscipline | RTTI confirmed |
| `0x00e45be0` | MemberCallback vfunc_3: VCrafting × Cache_ElementReady<SGW::Blueprint> | Blueprint cache warm |
| `0x00e45ce0` | MemberCallback vfunc_3: VCrafting × TimerUpdate | Craft induction countdown |
| `0x00d683b0` | `SGWNetworkManager_VEvent_NetOut_Craft___EventHandler__vfunc_0` | Scalar dtor → wire send |
| `0x00d68450` | `SGWNetworkManager_VEvent_NetOut_RespecCraft___EventHandler__vfunc_0` | Scalar dtor → wire send |

---

## Stargate DHD State Machine (W-content-mech Session 5 — 2026-05-13)

See [`findings/stargate-dhd-state-machine.md`](findings/stargate-dhd-state-machine.md) for full analysis.

| Address | Function | Notes |
|---------|----------|-------|
| `0x00d93060` | `register_NetOut_onDialGate` | Returns `"Event_NetOut_onDialGate"` |
| `0x00d8fe80` | `register_NetOut_DHD` | Returns `"Event_NetOut_DHD"` |
| `0x00cbc3e0` | `register_NetOut_GiveStargateAddress` | Returns `"Event_NetOut_GiveStargateAddress"` |
| `0x00ae9dd0` | `register_NetOut_SetRingTransporterDestination` | Returns `"Event_NetOut_SetRingTransporterDestination"` |
| `0x00d87340` | `register_NetIn_setupStargateInfo` | Returns `"Event_NetIn_setupStargateInfo"` |
| `0x00d875e0` | `register_NetIn_updateStargateAddress` | Returns `"Event_NetIn_updateStargateAddress"` |
| `0x00d87880` | `register_NetIn_StargateRotationOverride` | Returns `"Event_NetIn_StargateRotationOverride"` |
| `0x00d88060` | `register_NetIn_StargateTriggerFailed` | **New** — not in gate-travel-wire-formats.md |
| `0x00d88300` | `register_NetIn_onStargatePassage` | Returns `"Event_NetIn_onStargatePassage"` |
| `0x00d7a3c0` | `register_NetIn_onDisplayDHD` | Returns `"Event_NetIn_onDisplayDHD"` |
| `0x00d82c60` | `register_NetIn_onDHDReply` | Returns `"Event_NetIn_onDHDReply"` |
| `0x00d89020` | `register_NetIn_onRingTransporterList` | Returns `"Event_NetIn_onRingTransporterList"` |
| `0x00e2e120` | `EmitNetOut_onDialGate` | Sets TargetAddressId + SourceAddressId; 6-glyph→INT32 resolution loop |
| `0x00aeab70` | `EmitNetOut_SetRingTransporterDestination` | Sets aRegionId + aDestinationId; 0xC-byte Pattern B |
| `0x00ae9d70` | `EventNetOut_SetRingTransporterDestination_Ctor` | NetworkEvent ctor → typed vtable |
| `0x00c8a830` | `SlashCmd_EmitSetRingTransporterDestination` | Source: SGWTextCommandManager.cpp L0xC35–C36 |
| `0x00e30f20` | Dispatch helper (onDialGate path) | Allocates 0x18-byte emit info |
| `0x00d2d8f0` | Glyph accessor | Returns UINT8 glyph from address struct by index (0–5) |
| `0x00d2d910` | Entity-type validator | Used by EmitNetOut_onDialGate |
| `0x00d2d8a0` | Address ID resolver | Returns INT32 address ID from 6-glyph pointer |
| `0x00e2fe10` | MemberCallback vfunc_3: VGateTravel × setupStargateInfo | RTTI class_GateTravel confirmed |
| `0x00e2ff90` | MemberCallback vfunc_3: VGateTravel × StargateTriggerFailed | New event — RTTI confirmed |
| `0x00e2fd10` | MemberCallback vfunc_3: VGateTravel × Cache_ElementReady<DBGateInfo> | Gate info cache warm |
| `0x00e30090` | MemberCallback vfunc_3: VGateTravel × World_StargateEvent | Kismet-scripted gate event |
| `0x00e30110` | MemberCallback vfunc_3: VGateTravel × World_DialStargateAddress | Kismet-scripted dial trigger |
| `0x00cf5440` | MemberCallback vfunc_3: **VCommunicator** × onDHDReply | DHD NPC reply — NOT GateTravel |
| `0x00df7900` | MemberCallback vfunc_3: **VGameProxyPlayer** × onRingTransporterList | Ring transporter destinations |

---

## Loot Generation Pipeline (W-content-mech Session 5 — 2026-05-13)

See [`findings/loot-generation.md`](findings/loot-generation.md) for full analysis.

| Address | Function | Notes |
|---------|----------|-------|
| `0x00d935a0` | `register_NetOut_LootItem` | Returns `"Event_NetOut_LootItem"`; wire: 5 bytes total |
| `0x00d96790` | `register_NetOut_SquadSetLootMode` | Returns `"Event_NetOut_SquadSetLootMode"` |
| `0x00d804f0` | `register_NetIn_LootDisplay` | Returns `"Event_NetIn_LootDisplay"` |
| `0x00d8cc90` | `register_NetIn_onSquadLootType` | Returns `"Event_NetIn_onSquadLootType"` |
| `0x00d93680` | `CME_EventSignal_VEvent_NetOut_LootItem___TypedEmitInfo__vfunc_0` | Plate: "CellMethod: lootItem(Index: INT32)" |
| `0x00d805d0` | `CME_EventSignal_VEvent_NetIn_LootDisplay___TypedEmitInfo__vfunc_0` | Plate: "entityId UINT32 + ARRAY of InvItem FIXED_DICT" |
| `0x00d67b70` | `SGWNetworkManager_VEvent_NetOut_LootItem___EventHandler__vfunc_0` | → FUN_00d56fd0 → FUN_00d56f60 |
| `0x00d67f90` | `SGWNetworkManager_VEvent_NetOut_SquadSetLootMode___EventHandler__vfunc_0` | → FUN_00d59910 → FUN_00d598a0 |
| `0x00e248f0` | MemberCallback vfunc_3: VLootables × LootDisplay | RTTI class_Lootables confirmed |
| `0x00e24970` | MemberCallback vfunc_3: VLootables × Cache_ElementReady<DBInvItem> | Item DB cache warm before loot window |
| `0x00e5e870` | MemberCallback vfunc_3: VSquad × onSquadLootType | RTTI class_Squad confirmed |
| `0x00ce3730` | `SGWScriptedWindow_X_UEvent_UI_LootDisplay___GameEventHandler__vfunc_0` | Flash loot window handler |
| `0x00ccb5a0` | `MemberCallbackRtti_UI_LootDisplay__SGWScriptedWindow` | RTTI for UI loot subscription |

---

## Mercury Layer — Session 5b Completions (2026-05-13)

Full range `[0x01576000, 0x0158efff]` annotated. 145 functions renamed in Ghidra. See [`findings/mercury-protocol-internals.md`](findings/mercury-protocol-internals.md) for full analysis.

### MachineGuard Protocol

| Address | Function | Notes |
|---------|----------|-------|
| `0x01588530` | `MachineGuardMessage__deserialize` | Master deserializer — switch on type byte, allocates correct subtype |
| `0x01588ec0` | `MachineGuard__sendRawPacket` | socket()+bind+sendto UDP broadcast |
| `0x01589f80` | `MachineGuard__createSocketAndSend` | socket()+bindInRange+sendAndRecv; error codes 0xfffffffd/0xfffffffe |
| `0x01587d30` | `WholeMachineMessage__ctor2` | type=1; inits interface+component vectors |
| `0x01587de0` | `WholeMachineMessage__dtor` | frees interface sub-vector, hostname string |
| `0x01588fc0` | `WholeMachineMessage__read` | full deserialization: header+11 fields+iface table+component table |
| `0x01586410` | `ListenerMessage__ctor` | type=4; inits 2 SSO strings |
| `0x015864b0` | `ListenerMessage__dtor` | frees 2 SSO strings |
| `0x01586c20` | `ListenerMessage__read` | header+11 fields |
| `0x01586590` | `CreateMessage__ctor` | type=5; inits 2 SSO strings |
| `0x01586630` | `CreateMessage__dtor` | frees 2 SSO strings |
| `0x01586cf0` | `CreateMessage__read` | ListenerMessage::read + 2 extra bytes |
| `0x01586710` | `SignalMessage__ctor` | type=6 |
| `0x01586140` | `SignalMessage__writeWithName` | writeHeader + byte + bundle string |
| `0x015867c0` | `ErrorMessage__ctor` | type=0xb; severity=5 |
| `0x01586850` | `ErrorMessage__dtor` | frees message string |
| `0x01586f40` | `ErrorMessage__read` | header+severity+source+msg+code |
| `0x01587ef0` | `TagsMessage__ctor` | type=7; zeroes ByteVec |
| `0x01589560` | `TagsMessage__read` | header+tag count+var-length tag bytes |
| `0x01587110` | `ComponentType__getNameForType` | SERVER_COMPONENT(0)/WATCHER_NUB(1)/UNKNOWN |
| `0x01586e40` | `UserMessage__writeFullPayload` | writeHeader+5 strings+components |

### ProcessMessage Serialization Infrastructure

| Address | Function | Notes |
|---------|----------|-------|
| `0x01586180` | `ProcessMessage__writeComponentsVarLen` | var-len ID encoding (0xff prefix for ID>0xfe) |
| `0x015896d0` | `ProcessMessage__read` | header+component count+interface count, fills vectors |
| `0x01586b80` | `ProcessMessage__ComponentEntry__copyConstruct` | copy-construct 0x20-byte entry |
| `0x015872f0` | `ProcessMessage__ComponentEntry__copyIfNotNull` | null-guard wrapper |
| `0x01587390` | `ProcessMessage__ComponentVec__copyRangeReverse` | backward copy 0x20-byte entries |
| `0x01587430` | `ProcessMessage__ComponentVec__copyRangeWithSEH` | forward copy 0x20-byte + SEH |
| `0x015874c0` | `ProcessMessage__ComponentVec__copyRangeForward` | forward copy 0x3c-byte entries |
| `0x01587560` | `ProcessMessage__ComponentVec__copyRangeForwardSEH` | SEH-wrapped 0x3c-byte copy |
| `0x015877a0` | `ProcessMessage__ComponentVec__fillRange` | fill [begin,end) 0x20-byte stride |
| `0x01587970` | `ProcessMessage__ComponentVec__fillRange2` | fill [begin,end) 0x3c-byte stride |
| `0x015878e0` | `ProcessMessage__ComponentVec__uninitCopyN` | copy N * 0x3c-byte entries to uninit buffer |
| `0x015879c0` | `ProcessMessage__ComponentVec__copyRangeReverseWithReturn` | backward copy 0x3c-byte, returns end |
| `0x01587a60` | `ProcessMessage__ComponentVec__copyRangeWithReturn` | forward copy 0x3c-byte, returns end |
| `0x01587b20` | `ProcessMessage__ComponentVec__dtor` | iterates 0x3c-byte entries calling destructor |
| `0x01587c00` | `ProcessMessage__ComponentVec__resize` | resize 0x3c-byte vector |
| `0x01588870` | `ProcessMessage__InterfaceVec__reserve` | reserve capacity 0x3c-byte structs |
| `0x015889b0` | `ProcessMessage__ComponentVec__insertN` | insert N 0x3c-byte entries, realloc if needed |
| `0x01588cb0` | `ProcessMessage__InterfaceVec__resize` | resize 0x20-byte interface vector |
| `0x01588da0` | `ProcessMessage__ComponentVec__insertOne` | insert one 0x3c-byte entry |
| `0x01588e60` | `ProcessMessage__InterfaceVec__resizeDefault` | resize 0x20-byte with default-init |
| `0x015891e0` | `ProcessMessage__ComponentVec__pushBack` | append one 0x3c-byte entry |

### ChannelInternal Lifecycle and Stats

| Address | Function | Notes |
|---------|----------|-------|
| `0x0158c7b0` | `ChannelInternal__ctor` | Full ~0x180-byte init; hash table+stats+timers+bundle+UnAckedHandler+timeouts |
| `0x0158d190` | `ChannelInternal__dtor` | Entry: stamps vtable, resetLocalPart, cleanup1 |
| `0x0158d267` | `ChannelInternal__dtor_cleanup1` | Mercury_Channel_cleanup('\0') + cleanup2 |
| `0x0158d310` | `ChannelInternal__dtor_cleanup2` | frees name/bundle/filter/listener map; restores TimerExpiryHandler::vftable |
| `0x0158bed0` | `ChannelInternal__checkAndSendNubException` | rdtsc vs +0x160/+0x164 → NubException; vs +0x16c → sendAckBundle2 |
| `0x0158bd40` | `ChannelInternal__recordLatency` | TBB store8 into +0x3c..+0x7c; min/max at +0x178/+0x17c |
| `0x0158b9d0` | `ChannelInternal__getAndResetStats` | reads +0x17c/+0x178/+0x7c, resets accumulators |
| `0x0158be30` | `ChannelInternal__processIncomingPacketEntry` | dispatch to Nub::dispatchPacketWithFilter; rdtsc at +0x58/+0x5c |
| `0x0158a850` | `ChannelInternal__getNextChannelInternal` | atomic read +8 + incRef; safe list traversal |
| `0x0158a8e0` | `ChannelInternal__countChain` | walk chain, return count |
| `0x0158ab40` | `ChannelInternal__advanceReadPointer` | advance read cursor; walk to next ChannelInternal |
| `0x015875f0` | `ChannelInternal__unackedList__clear` | free all UnAcked list entries |
| `0x0158b960` | `ChannelInternal__getUnAckedHandlerOffset` | returns param_1+0x114 |
| `0x015868d0` | `ChannelStats__clear` | zeroes 0x2d-byte stats block |

### UnAckedHandler Completion

| Address | Function | Notes |
|---------|----------|-------|
| `0x0158b2d0` | `UnAckedHandler__buildAndSendAckBundle` | builds ACK bundle from 32-bit ack mask |
| `0x0158bbc0` | `UnAckedHandler__sendAckBundle2` | empty bundle + reliable flag + Finalise + Send |

### Packet Chain Operations

| Address | Function | Notes |
|---------|----------|-------|
| `0x0158a340` | `Packet__dtor` | stamps vtable; atomic decrement `DAT_018d4858`; decRef inner |
| `0x0158a3f0` | `Packet__chain__stampSendTime` | walk chain, stamp rdtsc at +0x18..+0x1f |
| `0x0158a4f0` | `Packet__chain__stampRecvTime` | walk chain, stamp rdtsc at +0x20..+0x27 |
| `0x0158a5f0` | `Packet__chain__minSendTime` | minimum sendTime across chain |
| `0x0158a720` | `Packet__chain__maxSendTime` | maximum sendTime across chain |

### ChannelInternalPtr Smart Pointer

| Address | Function | Notes |
|---------|----------|-------|
| `0x0158c100` | `ChannelInternalPtr__decRef` | atomic decRef; zero → destructor chain |
| `0x0158c230` | `ChannelInternalPtr__assign` | incRef new + copy byte flag + decRef old |

### Misc Mercury Utilities

| Address | Function | Notes |
|---------|----------|-------|
| `0x0158a1d0` | `CME__CountedBase__stampBaseVtable` | one-liner stamps base counted-object vtable |
| `0x01589840` | `MGMPacket__readAndInit` | clear all fields + MGMPacket::read |
| `0x01586960` | `ByteVec__insertN` | std::vector<byte> insert N at position |
| `0x01587060` | `ByteVec__resize` | std::vector<byte> resize |
| `0x015862f0` | `ByteRange__memmoveToNewDst` | memmove_s([p1,p2)->p3), returns new end |

### Notable Globals (Mercury)

| Address | Name | Notes |
|---------|------|-------|
| `DAT_018d4858` | Mercury global packet count | Atomically maintained by Packet__dtor |
| `DAT_018cad90` | `BW_TO_UE3_SCALE` | 100.0f — confirmed in world-entry pipeline |

---

## Naming Conventions

- Ghidra function names: `ClassName_methodName` (underscore separator)
- Vtable entries: `ClassName__vfunc_N` (double underscore + index)
- Inferred names: `ClassName__unknown_HEXADDR` (script 10)
- Event handlers: `EventHandler_NetOut_EventName` or `EventHandler_NetIn_EventName`
