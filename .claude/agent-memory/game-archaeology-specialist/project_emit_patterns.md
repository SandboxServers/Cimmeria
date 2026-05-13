---
name: project-emit-patterns
description: Two distinct CME emit patterns confirmed in SGW.exe; lower half [0x00400000, 0x00b00000) has only 2 EmitNetOut functions using Pattern B; emitter catalog addresses.
metadata:
  type: project
---

## CME EventSignal Emit Patterns (confirmed 2026-05-13, W-emit-A)

Two distinct emit patterns exist in SGW.exe. Both fire the wire-level NetOut signal but construct it differently.

### Pattern A — Canonical (GetSystem + LookupByName + SetField)
Used by: `EmitNetOut_DebugMinigameInstance` at `0x00c79120` (upper half).
Steps:
1. `CmeEventSignal_GetSystem` (`0x0155f790`) — get singleton
2. `CmeEventSignal_LookupByName` (`0x00a5c0f0`) — resolve signal by static name string (e.g. "Event_NetOut_debugMinigameInstance")
3. `CmeEventSignal_SetField` (`0x0043b850`) × N — set field key-value pairs
4. vtable dispatch at `*this+0xC` — fire

### Pattern B — Vtable-typed constructor + SetField
Used by: `EmitNetOut_callForAid` (`0x00aea880`) and `EmitNetOut_SetRingTransporterDestination` (`0x00aeab70`) (both lower half).
Steps:
1. `scalable_malloc(0xC)` — allocate 12-byte signal object
2. `EventNetOut_*_Ctor` — call NetworkEvent base ctor (`FUN_004412e0`), then stamp `Event_NetOut_*::vftable`
3. `CmeEventSignal_SetField` (`0x0043b850`) × N — set fields
4. vtable dispatch at `*this+0x8` — fire (note: different vtable slot than Pattern A's `+0xC`)
Does NOT call GetSystem or LookupByName at emit time — event type baked into vtable.

### Lower-half emitter inventory (complete, 2026-05-13)
Only 4 functions in `[0x00400000, 0x00b00000)` are EmitNetOut_* or their direct constructors:

| Address | Name | Event |
|---------|------|-------|
| `0x00aea880` | `EmitNetOut_callForAid` | `Event_NetOut_callForAid` — field: respawnerID (int*) |
| `0x00aeab70` | `EmitNetOut_SetRingTransporterDestination` | `Event_NetOut_SetRingTransporterDestination` — fields: aRegionId, aDestinationId |
| `0x00ae9590` | `EventNetOut_callForAid_Ctor` | constructor for callForAid signal object |
| `0x00ae9d70` | `EventNetOut_SetRingTransporterDestination_Ctor` | constructor for SetRingTransporterDestination signal object |

### NetworkEvent_Ctor — renamed (W-cleanup session 3, 2026-05-13)
- `0x004412e0` → `NetworkEvent_Ctor` — RENAMED. Has 200+ xrefs but almost all are typed ctors (one per event type in `0x00573d70–0x005b0e30`). Only 3 Pattern B *emitters* confirmed:
  - `0x00aea880` `EmitNetOut_callForAid`
  - `0x00aeab70` `EmitNetOut_SetRingTransporterDestination`
  - `0x00c8a830` `SlashCmd_EmitSetRingTransporterDestination` (SGWTextCommandManager.cpp wrapper, newly discovered)

### CmeMemberCallback struct — created (W-cleanup session 3, 2026-05-13)
Struct `CmeMemberCallback` (12 bytes) created in Ghidra root category:
- `+0x00 void* pVtable`
- `+0x04 void* pSubscriber`
- `+0x08 void* pMethodPtr`
NOTE: `create_data_type_category` for `/CmeEventSignal` failed ("Transaction not started"). Struct is in root. Move to `/CmeEventSignal` category manually in future Ghidra UI session.

### New pipeline helpers named (W-cleanup session 3, 2026-05-13)
- `0x00cb1f00` → `CmeEventSignal_SetFieldHelper` — wrapper: FUN_004410d0 (get handle) + SetField + release
- `0x00a5c150` → `CmeEventSignal_Subscribe` — inserts callback into subscriber set; returns bool (newly inserted)
- `0x00db3390` → `RegisterBulkNetOutSignals` — bulk startup registration of 40+ Event_NetOut_* signals

### Globals flushed (W-cleanup session 3, 2026-05-13)
24 globals applied from Sessions 1-2-3 checkpoints. See `globals-applied.json` for full list.
Notable: `g_pUBWNetDriverClass` (0x01ee2bb8), `g_pUBWConnectionClass` (0x01ee2bbc), `g_bEntityRpcDebug` (0x01ef2224), `g_pLogSink` (0x01ef2448), `g_pGEngine` (0x01ea576c), `g_pFCallbackEventDevice` (0x01ea577c).

### Non-emitters that call GetSystem/LookupByName (do NOT confuse with NetOut emitters)
- `FUN_005c75d0` — massive action/input dispatcher (Event_Action_*, Event_SlashCmd_*, Event_Editor_*, Event_Option_*)
- `FUN_0056f4d0` — SGWUIManager ctor (registers Event_UI_SlashCommand, Event_UI_BindableAction)
- `FUN_00578f20`, `FUN_00579040`, `FUN_00579ae0` — property-change emitters with dynamic signal names
- `FUN_00956620` — keybinding state emitter
- `FUN_00a4cfd0` — slash-command dispatcher

**Why:** LookupByName is used by subscribers AND emitters. GetSystem appears in constructors and dispatchers too. The SetField xref list is the reliable discriminant for emitters.
