# Crafting State Machine

> **Date**: 2026-05-13
> **Source audit**: 2026-09-19 (Rust and entity definitions; no new binary analysis)
> **Phase**: V5 Documentation Campaign — W-content-mech Session 5
> **Confidence**: HIGH (MemberCallback RTTI confirms class names; emitter stubs confirm event names; wire-format fields confirmed from `.def`)
> **Sources**: Ghidra decompilation of SGW.exe; cross-reference to `crafting-wire-formats.md`

---

## Overview

The crafting system is implemented client-side by `class_SGW::Crafting` (client view class `VCrafting`). This class subscribes to all server-pushed crafting events via the CME EventSignal bus. The server drives state; the client is a pure display consumer. Four NetOut events (Craft, Alloy, Research, ReverseEngineer) plus SpendAppliedSciencePoint and RespecCraft carry player intent. Six NetIn events carry server state updates.

The VCrafting MemberCallback table also confirms `onUpdateRacialParadigmLevel` and a TimerUpdate subscription. The racial-paradigm argument schema is now recorded in [crafting-wire-formats.md](crafting-wire-formats.md#onupdateracialparadigmlevel--racial-paradigm-level-update); the runtime audit below separates that contract from implemented server behavior.

---

## CME EventSignal Event Inventory

### NetOut Events (Client → Server)

All six are name-registration stubs confirmed from Ghidra decompilation:

| Event Name | Address (stub) | Notes |
|---|---|---|
| `Event_NetOut_Craft` | `0x00e4a910` | Craft recipe + item instance IDs |
| `Event_NetOut_Alloy` | `0x00e4aac0` | Alloy recipe + current-tier + lower-tier items |
| `Event_NetOut_Research` | `0x00e4ac70` | Item ID + kicker items |
| `Event_NetOut_ReverseEngineer` | `0x00e4ae20` | Item ID only |
| `Event_NetOut_SpendAppliedSciencePoint` | `0x00e4afd0` | Discipline sequence ID |
| `Event_NetOut_RespecCraft` | `0x00aea3d0` | No arguments |
| `Event_NetOut_SetTechSkill` | `0x00d96f70` | Tech skill override (GM/debug) |

`SGWNetworkManager` dispatches each via dedicated `EventHandler` wrappers:

| Handler | Address | Inner cleanup |
|---|---|---|
| `SGWNetworkManager_VEvent_NetOut_Craft___EventHandler__vfunc_0` | `0x00d683b0` | → `FUN_00d5c250` (scalar dtor, vtable reset) → `FUN_00d5c1e0` (send wrapper) → `FUN_00d47bf0` (MemberCallback ctor) |
| `SGWNetworkManager_VEvent_NetOut_RespecCraft___EventHandler__vfunc_0` | `0x00d68450` | → `FUN_00d5c890` → `FUN_00d5c820` → `FUN_00d47e70` |

The send wrapper chain for all six events follows the same three-tier pattern: EventHandler vfunc_0 → scalar destructor with vtable reset → MemberCallback constructor that stamps the typed vtable into the signal object, then calls `FUN_00a374a0` (universal wire-send).

### NetIn Events (Server → Client) — VCrafting Subscriptions

The `class_SGW::Crafting` (RTTI name from MemberCallback vfunc_3 descriptors) subscribes to the following events. All confirmed from RTTI descriptors at addresses below:

| Event Name | MemberCallback vfunc_3 Address | Notes |
|---|---|---|
| `Event_NetIn_onUpdateCraftingOptions` | `0x00e45960` | RTTI: `SGW::Crafting` × `Event_NetIn_onUpdateCraftingOptions` |
| `Event_NetIn_onUpdateKnownCrafts` | `0x00e459e0` | RTTI: `SGW::Crafting` × `Event_NetIn_onUpdateKnownCrafts` |
| `Event_NetIn_onUpdateRacialParadigmLevel` | `0x00e45a60` | RTTI confirmed; argument schema verified from `SGWPlayer.def` |
| `Event_NetIn_onUpdateDiscipline` | `0x00e45ae0` | RTTI: `SGW::Crafting` × `Event_NetIn_onUpdateDiscipline` |
| `Event_NetIn_onDisciplineRespec` | `0x00e45b60` | RTTI: `SGW::Crafting` × `Event_NetIn_onDisciplineRespec` |
| `Event_Cache_ElementReady<SGW::Blueprint>` | `0x00e45be0` | Cache warming — when blueprint DB entry loads |
| `Event_NetIn_onCraftingRespecPrompt` | `0x00e45c60` | RTTI: const* variant |
| `Event_NetIn_TimerUpdate` | `0x00e45ce0` | Frame-tick timer; drives craft induction countdown |

**Registration stubs** (all returning string literals):

| Event | Stub Address |
|---|---|
| `register_NetIn_onUpdateDiscipline` | `0x00d831a0` |
| `register_NetIn_onUpdateCraftingOptions` | `0x00d83980` |
| `register_NetIn_onUpdateKnownCrafts` | `0x00d836e0` |
| `register_NetIn_onCraftingRespecPrompt` | `0x00d7fd00` |

**TypedEmitInfo destructors** (confirm event class existence):

| Event | TypedEmitInfo vfunc_0 | Inner dtor |
|---|---|---|
| `onCraftingRespecPrompt` | `0x00d7fde0` | `FUN_00d7fd80` |
| `onUpdateCraftingOptions` | `0x00d83a60` | `FUN_00d83a00` |
| `onUpdateKnownCrafts` | `0x00d837c0` | `FUN_00d83760` |
| `onUpdateDiscipline` | `0x00d83280` | `FUN_00d83220` |

---

## State Machine

The crafting system is a **request-response** model, not a persistent state machine with client-side guard states. The client sends a request event; the server processes and pushes back one or more update events. There is no client-side state enum.

### Craft / Research / Reverse-Engineer / Alloy Flow

```text
Player UI action
       │
       ▼
CME EventSignal bus
       │  (Event_NetOut_Craft / Alloy / Research / ReverseEngineer)
       ▼
SGWNetworkManager EventHandler
       │  (EventHandler vfunc_0 → scalar dtor → MemberCallback ctor → FUN_00a374a0)
       ▼
BigWorld wire send → server cell method
       │
       ▼ (server processes; result: success or failure)
       │
  ┌────┴────────────────────────────────────────┐
  │                                             │
  ▼                                             ▼
onUpdateKnownCrafts               (error — no wire message;
(if a new recipe was learned       server-side only or via
 from research)                    generic error channel)
  │
  ▼
onUpdateCraftingOptions
(updated available items/entities)
  │
  ▼
  [Client UI refreshes]
```

### Discipline / Applied Science Point Flow

This describes the client event contract, not an implemented Rust progression path.

```text
Player clicks "Spend Applied Science Point"
       │
       ▼
Event_NetOut_SpendAppliedSciencePoint (disciplineSeqId)
       │
       ▼
Server validates + processes
       │
       ▼
onUpdateDiscipline (disciplineSeqId, expertise)
       │
       ▼ (optional — if racial paradigm changed)
onUpdateRacialParadigmLevel (racialParadigmId: INT32, level: INT8)
```

### Respec Flow

```text
Player initiates respec
       │
       ▼
Event_NetOut_RespecCraft  (no args)
       │
       ▼
Server: validates cost availability
       │
       ▼
onCraftingRespecPrompt (CostToRespec: INT32) → UI shows confirmation dialog
       │
       ▼ (player confirms — note: no second NetOut is confirmed from binary;
          the server likely treats the first RespecCraft as both request and confirm
          after showing the prompt, or the UI re-sends; open question — see below)
       │
       ▼
onDisciplineRespec (no args) → UI clears discipline state

SGWScriptedWindow handles:
  - UEvent_UI_CraftingRespecPrompt: 0x00ce39b0 (→ FUN_00ce1130)
  - UEvent_UI_CraftingAllowedUpdate: 0x00ce3970 (→ FUN_00ce0e50)
  - UEvent_UI_CraftInductionStart:   0x00ce9c30 (→ FUN_00ce9a90)
```

### Craft Induction (Timer) Flow

`VCrafting` subscribes to `Event_NetIn_TimerUpdate` (MemberCallback at `0x00e45ce0`). This drives the craft induction countdown UI — while a craft is in-progress, the client receives server-pushed timer ticks. The `UEvent_UI_CraftInductionStart` (TypedEmitInfo `0x00e45860`) fires on the client event bus when a craft induction begins.

---

## SGW::Crafting Class Anatomy

The client crafting class is `class_SGW::Crafting` (demangled from RTTI descriptors in MemberCallback vfunc_3 functions). It also consumes the Blueprint data cache:

- Subscribes to `Event_Cache_ElementReady<SGW::Blueprint>` at `0x00e45be0` — when blueprint data is available from the asset DB, VCrafting updates its available-recipe list.
- Subscribes to `Event_NetIn_TimerUpdate` for induction countdown rendering.

---

## UI Event Signals (Client-Internal)

These are CME events fired by VCrafting to the UI layer (not wire events):

| CME Event | TypedEmitInfo vfunc_0 | CallbackImpl vfunc_2 |
|---|---|---|
| `UEvent_UI_CraftInductionStart` | `0x00e45860` | `0x00cc7e80` |
| `UEvent_UI_CraftingAllowedUpdate` | `0x00e457c0` | `0x00cc7e30` |
| `UEvent_UI_CraftingRespecPrompt` | `0x00e45840` | `0x00cc7e50` |

`SGWScriptedWindow` subscribes to all three and drives Flash/Scaleform UI state.

---

## onUpdateRacialParadigmLevel: Verified Schema

`Event_NetIn_onUpdateRacialParadigmLevel` is subscribed by `class_SGW::Crafting` (binary RTTI at `0x00e45a60`). [SGWPlayer.def](../../../entities/defs/SGWPlayer.def) declares `INT32 aRacialParadigmId` followed by `INT8 aLevel`; the [canonical client dispatch table](../../protocol/client-method-dispatch-table.md) assigns method index **138**. The argument payload is **five bytes**, excluding transport framing.

**Confidence: HIGH for the definition-based schema and method index.** This corrects the earlier “wire format unknown” statement using entity-definition and dispatch-table evidence. No new emitter/constructor decompilation or packet capture was performed. The subscription alone does not establish how the client presents the level or when the server should increase it.

### Rust runtime audit (2026-09-19)

Static review at `beaf79471154a2e558fd7d112115950519a3f530` found a missing progression and synchronization path, not a missing notification after an otherwise implemented level-up:

| Surface | Finding | Source |
|---|---|---|
| Method 138 delivery | The constant and wire-log decoder exist; no runtime sending call site was found | [player client methods](../../../crates/wire/src/cell/client_methods/player.rs), [generated decoder](../../../crates/services/src/wire_log/decoders/generated.rs) |
| Level mutation | The state stores levels, but production callers grant expertise or applied science points without changing the paradigm map; `.allcraft` reports incomplete implementation | [CraftingState](../../../crates/entity/src/crafting.rs), [grant handlers](../../../crates/services/src/base/crafting/handlers.rs), [console crafting](../../../crates/services/src/cell/console/crafting.rs) |
| Persistence and login | Load/save helpers decode and re-encode the map; no login caller of `load_crafting_state` was found | [crafting persistence](../../../crates/services/src/base/crafting/persistence.rs) |

`onPlayerDataLoaded` has no arguments in `SGWPlayer.def`; it does not itself carry paradigm levels. This audit therefore does **not** establish that relogging restores the crafting UI. The legacy [Crafter](../../../deprecated/python/cell/Crafter.py) mutation path calls `onRacialParadigmUpdated`, whose [SGWPlayer](../../../deprecated/python/cell/SGWPlayer.py) implementation emits the update; that is reference intent, not a Rust implementation.

The implementation gap is tracked in [#723](https://github.com/SandboxServers/Cimmeria/issues/723). No live level gain, client UI update, or packet capture was exercised. Those checks remain necessary once progression and initial synchronization are implemented.

---

## Contradictions with crafting-wire-formats.md

1. **Resolved omission:** `onUpdateRacialParadigmLevel` is now documented in `crafting-wire-formats.md` with the definition-verified two-argument schema. Runtime delivery remains unimplemented as recorded above.
2. **`TimerUpdate` subscription** is not mentioned. VCrafting uses it for induction countdown. Not a new network message — it's the shared system timer event.
3. **`Cache_ElementReady<SGW::Blueprint>`** subscription is not mentioned. VCrafting waits for blueprint data cache before populating recipe lists.
4. Wire fields for craft actions (craft recipe ID, item arrays, etc.) are confirmed accurate from `.def` — no contradictions in the base wire format table.

---

## Related Documents

- [crafting-wire-formats.md](crafting-wire-formats.md) — wire format tables, including `onUpdateRacialParadigmLevel`
- [cme-event-signal.md](cme-event-signal.md) — CME EventSignal pipeline anatomy
- [inventory-wire-formats.md](inventory-wire-formats.md) — item ID and InvItem FIXED_DICT layout
