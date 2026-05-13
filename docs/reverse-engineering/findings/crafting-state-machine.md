# Crafting State Machine

> **Date**: 2026-05-13
> **Phase**: V5 Documentation Campaign — W-content-mech Session 5
> **Confidence**: HIGH (MemberCallback RTTI confirms class names; emitter stubs confirm event names; wire-format fields confirmed from `.def`)
> **Sources**: Ghidra decompilation of SGW.exe; cross-reference to `crafting-wire-formats.md`

---

## Overview

The crafting system is implemented client-side by `class_SGW::Crafting` (client view class `VCrafting`). This class subscribes to all server-pushed crafting events via the CME EventSignal bus. The server drives state; the client is a pure display consumer. Four NetOut events (Craft, Alloy, Research, ReverseEngineer) plus SpendAppliedSciencePoint and RespecCraft carry player intent. Six NetIn events carry server state updates.

**New finding not in `crafting-wire-formats.md`:** A seventh NetIn event — `onUpdateRacialParadigmLevel` — and a TimerUpdate subscription are present in the VCrafting MemberCallback table. These were not derived from `.def` analysis.

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
| `Event_NetIn_onUpdateRacialParadigmLevel` | `0x00e45a60` | **New** — not in crafting-wire-formats.md; RTTI confirmed |
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

```
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

```
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
onUpdateRacialParadigmLevel   [NEW — not previously documented]
```

### Respec Flow

```
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

## New Finding: onUpdateRacialParadigmLevel

`Event_NetIn_onUpdateRacialParadigmLevel` is subscribed by `class_SGW::Crafting` (RTTI confirmed at `0x00e45a60`). This event is absent from `crafting-wire-formats.md` and from the known `.def` analysis. It likely carries a racial paradigm level integer — analogous to `onUpdateDiscipline` — and updates a crafting sub-system tied to player race selection. Wire format is unknown; needs a separate investigation of the emitter/constructor for this event class.

---

## Contradictions with crafting-wire-formats.md

1. **`onUpdateRacialParadigmLevel`** is not documented in `crafting-wire-formats.md`. Confirmed present by binary RTTI. Must be added.
2. **`TimerUpdate` subscription** is not mentioned. VCrafting uses it for induction countdown. Not a new network message — it's the shared system timer event.
3. **`Cache_ElementReady<SGW::Blueprint>`** subscription is not mentioned. VCrafting waits for blueprint data cache before populating recipe lists.
4. Wire fields for craft actions (craft recipe ID, item arrays, etc.) are confirmed accurate from `.def` — no contradictions in the base wire format table.

---

## Related Documents

- [crafting-wire-formats.md](crafting-wire-formats.md) — wire format tables (extend with `onUpdateRacialParadigmLevel`)
- [cme-event-signal.md](cme-event-signal.md) — CME EventSignal pipeline anatomy
- [inventory-wire-formats.md](inventory-wire-formats.md) — item ID and InvItem FIXED_DICT layout
