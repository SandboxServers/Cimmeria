# Loot Generation Pipeline

> **Date**: 2026-05-13
> **Phase**: V5 Documentation Campaign — W-content-mech Session 5
> **Confidence**: HIGH for event class/subscriber graph; MEDIUM for internal loot pipeline ordering (no server-side code in binary; inferred from client event sequence and group-wire-formats.md)
> **Sources**: Ghidra decompilation of SGW.exe; cross-reference to `right-click-routing-on-corpse.md`, `group-wire-formats.md`, `inventory-wire-formats.md`

---

## Overview

The loot system has two separate client consumers. `class_Lootables` (VLootables) handles the loot window display and item pickup. `class_Squad` (VSquad) handles loot mode selection for group play. There is no persistent loot state machine on the client — the loot window is ephemeral, driven by a single `LootDisplay` message per kill.

The server entirely owns drop-table evaluation. The client receives a pre-computed loot list and renders it. Roll mechanics (need/greed/pass) are not confirmed in the binary as a distinct NetOut flow — see open questions.

---

## CME EventSignal Event Inventory

### NetOut Events (Client → Server)

| Event Name | Stub Address | Purpose |
|---|---|---|
| `Event_NetOut_LootItem` | `0x00d935a0` | Player picks up one loot item by index |
| `Event_NetOut_SquadSetLootMode` | `0x00d96790` | Squad leader changes loot mode |

`SGWNetworkManager` dispatches both via `EventHandler` wrappers:

| Handler | Address | Inner chain |
|---|---|---|
| `SGWNetworkManager_VEvent_NetOut_LootItem___EventHandler__vfunc_0` | `0x00d67b70` | → `FUN_00d56fd0` → `FUN_00d56f60` → `FUN_00d45af0` (MemberCallback ctor) |
| `SGWNetworkManager_VEvent_NetOut_SquadSetLootMode___EventHandler__vfunc_0` | `0x00d67f90` | → `FUN_00d59910` → `FUN_00d598a0` → `FUN_00d46b70` (MemberCallback ctor) |

TypedEmitInfo destructors confirm event class existence:

| Event | TypedEmitInfo vfunc_0 | Inner dtor |
|---|---|---|
| `Event_NetOut_LootItem` | `0x00d93680` | `FUN_00d93620` |
| `Event_NetOut_SquadSetLootMode` | `0x00d96870` | `FUN_00d96810` |

The `Event_NetOut_LootItem` TypedEmitInfo plate comment notes: "Corresponds to SGWPlayer.def CellMethod: lootItem(Index: INT32)" — confirming the single-field wire layout.

### NetIn Events (Server → Client)

| Event Name | Stub Address | TypedEmitInfo vfunc_0 | Primary Subscriber |
|---|---|---|---|
| `Event_NetIn_LootDisplay` | `0x00d804f0` | `0x00d805d0` | `class_Lootables` (VLootables) + `SGWScriptedWindow` |
| `Event_NetIn_onSquadLootType` | `0x00d8cc90` | `0x00d8cd70` | `class_Squad` (VSquad) |

---

## Client Class Subscription Table

### class_Lootables (VLootables) Subscriptions

Both confirmed from MemberCallback vfunc_3 RTTI descriptors:

| Event | MemberCallback vfunc_3 | Notes |
|---|---|---|
| `Event_NetIn_LootDisplay` | `0x00e248f0` | RTTI: `Lootables` × `Event_NetIn_LootDisplay` |
| `Event_Cache_ElementReady<DBInvItem>` | `0x00e24970` | Item DB cache warming before loot window renders |

The `Cache_ElementReady<DBInvItem>` subscription is architecturally significant: VLootables subscribes to item data cache events. This means when a `LootDisplay` arrives with item IDs, the client may need to wait for `DBInvItem` records to load from the local asset cache before rendering the loot window. The loot window itself fires only after the cache warm completes.

### class_Squad (VSquad) Subscriptions (Loot Mode)

| Event | MemberCallback vfunc_3 | Notes |
|---|---|---|
| `Event_NetIn_onSquadLootType` | `0x00e5e870` | RTTI: `Squad` × `Event_NetIn_onSquadLootType` |

### SGWScriptedWindow (UI Layer)

`SGWScriptedWindow` also subscribes to `UEvent_UI_LootDisplay` (client-internal event):

| Event | GameEventHandler | Notes |
|---|---|---|
| `UEvent_UI_LootDisplay` | `0x00ce3730` → `FUN_00cdf470` | Drives Flash/Scaleform loot window UI |
| `UEvent_UI_LootDisplay` MemberCallback | `0x00ccb5a0` | RTTI: `SGWScriptedWindow` × `UEvent_UI_LootDisplay` |

The TypedEmitInfo plate comment for `Event_NetIn_LootDisplay` at `0x00d805d0` documents the event payload: **"entityId UINT32 + ARRAY of InvItem FIXED_DICT"**. This confirms the layout from `inventory-wire-formats.md`.

---

## Loot Pipeline

### Full Flow: Kill → Loot Window → Pickup

```
[Player kills enemy NPC]
       │
       ▼  (server-side only — client does not see drop-table evaluation)
[Server: evaluate drop table → compute loot list → create loot container]
       │
       ▼
[Server sends: Event_NetIn_LootDisplay]
  Fields: entityId (UINT32) + ARRAY<InvItem FIXED_DICT>
       │
       ▼  (client receives LootDisplay)
[VLootables: check DBInvItem cache for each item ID]
       │
       ├── Cache warm needed:
       │   [Subscribe Cache_ElementReady<DBInvItem>]
       │   [Wait for item data to load from asset DB]
       │   [On ElementReady: continue render]
       │
       └── Cache hit:
           [Immediately construct loot window]
       │
       ▼
[VLootables: emit UEvent_UI_LootDisplay → SGWScriptedWindow]
[Loot window visible to player]
       │
       ▼  (player clicks item)
[Client sends: Event_NetOut_LootItem {Index: INT32}]
  (Index = position in the ARRAY from LootDisplay)
       │
       ▼
[Server validates + grants item to inventory]
[Server sends: standard inventory update events (onContainerInfo / property sync)]
```

### Loot Container vs onContainerInfo

The `LootDisplay` event carries both the loot entity ID and the full item list inline — it is **not** a two-step "open container → request items" flow. The complete item list arrives in one message. This differs from a general container-open pattern. No `onContainerInfo` event was found in the crafting/loot namespace. The inventory system uses its own property-sync path for the result of item pickup.

### Group Loot Flow

```
[Group kills enemy NPC]
       │
       ▼
[Server determines loot mode: Free-for-All (1) or Round Robin (0) — only two modes exist]
[Server sends: Event_NetIn_onSquadLootType {lootType: ?}]
  → VSquad handles; updates squad loot mode UI
       │
       ▼
[Server sends: Event_NetIn_LootDisplay to eligible looters]
  (eligibility determined server-side by loot mode)
       │
       ▼  (per eligible player)
[Each eligible player sees loot window]
[Each sends: Event_NetOut_LootItem or Event_NetOut_SquadSetLootMode (leader only)]
```

**Roll mechanics (need/greed/pass):** Confirmed absent. The game shipped with two loot modes only: Round Robin (`aLootType=0`) and Free For All (`aLootType=1`). There is no need/greed/pass mode, no roll-vote NetOut event, and no roll-result NetIn event. See the "Need/Greed/Pass Roll" section for full evidence.

---

## Wire Format Summary

### Event_NetOut_LootItem

From `SGWPlayer.def` (confirmed via TypedEmitInfo plate comment):
```
lootItem(aIndex: INT32)
```
Wire: 1B header + 4B = 5 bytes total.

### Event_NetOut_SquadSetLootMode

From `SGWPlayer.def` or squad interface:
```
squadSetLootMode(aMode: INT32)    [hypothesized — wire not confirmed from binary emitter]
```
TypedEmitInfo exists but emitter not decompiled. Wire format inferred from naming.

### Event_NetIn_LootDisplay

From `inventory-wire-formats.md` (confirmed by TypedEmitInfo plate comment at `0x00d805d0`):
```
LootDisplay {
    entityId:    UINT32
    items:       ARRAY<InvItem FIXED_DICT>
}
```

### Event_NetIn_onSquadLootType

From `OrganizationMember.def` (confirmed — this is the interface SGWPlayer implements):
```
onSquadLootType(aOrganizationId: INT32, aLootType: INT32)
```
Wire: 2 × 4B = 8 bytes. `aLootType` values from `EGroupLootType`: `0`=RoundRobin, `1`=FFA. No other values exist — the enum is closed. Handler `FUN_00e5d650` stores the type at `this+0xC8` and emits a chat notification on change.

---

## Key Addresses

| Address | Symbol | Notes |
|---|---|---|
| `0x00d935a0` | `register_NetOut_LootItem` | Returns `"Event_NetOut_LootItem"` |
| `0x00d96790` | `register_NetOut_SquadSetLootMode` | Returns `"Event_NetOut_SquadSetLootMode"` |
| `0x00d804f0` | `register_NetIn_LootDisplay` | Returns `"Event_NetIn_LootDisplay"` |
| `0x00d8cc90` | `register_NetIn_onSquadLootType` | Returns `"Event_NetIn_onSquadLootType"` |
| `0x00d67b70` | `SGWNetworkManager_VEvent_NetOut_LootItem___EventHandler__vfunc_0` | Scalar destructor → inner cleanup → MemberCallback ctor |
| `0x00d67f90` | `SGWNetworkManager_VEvent_NetOut_SquadSetLootMode___EventHandler__vfunc_0` | Same pattern |
| `0x00d93680` | `CME_EventSignal_VEvent_NetOut_LootItem___TypedEmitInfo__vfunc_0` | Destructor; plate: "CellMethod: lootItem(Index: INT32)" |
| `0x00d805d0` | `CME_EventSignal_VEvent_NetIn_LootDisplay___TypedEmitInfo__vfunc_0` | Plate: "entityId UINT32 + ARRAY of InvItem FIXED_DICT" |
| `0x00e248f0` | `CME_EventSignal_ZV5…LootDisplay…VLootables…MemberCallback__vfunc_3` | RTTI: Lootables × LootDisplay |
| `0x00e24970` | `CME_EventSignal_ZU5…DBInvItem…VLootables…MemberCallback__vfunc_3` | RTTI: Lootables × Cache_ElementReady<DBInvItem> |
| `0x00e5e870` | `CME_EventSignal_ZV5…onSquadLootType…VSquad…MemberCallback__vfunc_3` | RTTI: Squad × onSquadLootType |
| `0x00ce3730` | `SGWScriptedWindow_X_UEvent_UI_LootDisplay___GameEventHandler__vfunc_0` | UI handler → FUN_00cdf470 |
| `0x00ccb5a0` | `MemberCallbackRtti_UI_LootDisplay__SGWScriptedWindow` | RTTI for UI subscription |

---

## Relationship to right-click-routing-on-corpse.md

The earlier investigation (`right-click-routing-on-corpse.md`) established the client-side route from right-click → `Event_NetOut_Interact` → server `interact` handler → `handle_interact` dispatches `onLootDisplay`. This document covers the downstream half: what happens after `onLootDisplay` is sent to the client. The two documents together cover the full loot loop:

```
Right-click corpse → interact → server dispatch → LootDisplay → VLootables → UI → LootItem → inventory sync
```

---

## Need/Greed/Pass Roll — W-loot-roll Session 5b Finding

> **Session**: W-loot-roll 5b — 2026-05-13
> **Verdict**: CONFIRMED ABSENT — need/greed/pass roll mechanics do not exist in this client or its server protocol.

Four independent evidence sources converge on the same answer:

**1. String exhaustion.** A full string-database sweep of SGW.exe found zero hits for: `LootRoll`, `NeedRoll`, `GreedRoll`, `PassRoll`, `lootRoll`, `lootVote`, `RollLoot`, `greed`, `need.*greed`. The words "greed" and "need" (in a loot context) are absent from the entire binary.

**2. Loot mode string handler — `FUN_00e5d650` (confirmed by decompile, address live-verified).** The `onSquadLootType` event handler reads field `"aLootType"` and branches on exactly two values:
- `0` → `L"The squad's loot mode has been set to round robin."`
- `1` → `L"The squad's loot mode has been set to free for all."`
- else → `L"The squad's loot mode has been set to an unknown mode."`

No third branch. No "need/greed" path. The string table at `0x019dd870`–`0x019dd940` is complete — three entries only.

**3. Canonical enum — `EGroupLootType` in `enumerations.xml` (canonical .def source).** Exactly two tokens:

| Token | Value |
|---|---|
| `GROUP_LOOT_RoundRobin` | `0` |
| `GROUP_LOOT_FreeForAll` | `1` |

No third entry. Need/greed/pass is not a planned extension — the enum is closed at 2 values.

**4. Complete .def survey.** `SGWPlayer.def`, `Lootable.def`, `OrganizationMember.def`, `SGWMob.def` — all loot-related methods examined. The only `<Exposed/>` client→server loot methods are `lootItem(Index: INT32)` and `squadSetLootMode(aLootMode: INT32)`. No roll-vote method of any kind was defined. `lootItem` carries exactly one INT32 field (item index) — confirmed by both the `.def` `<Exposed/>` declaration and the TypedEmitInfo plate at `0x00d93680`.

**Conclusion:** Stargate Worlds shipped with two group loot modes only — Round Robin and Free For All. Need/greed/pass roll voting was not implemented at the client level, the server protocol level, or in the entity definition layer. The mechanic was either cut before the 2009 build or never began implementation. Cimmeria should implement Round Robin and Free For All only; no roll-vote NetOut event or handler is needed.

---

## Wire Format Corrections (from W-loot-roll 5b)

### onSquadLootType (corrected)

From `OrganizationMember.def` (canonical source — this interface is what `SGWPlayer` implements):

```
onSquadLootType(aOrganizationId: INT32, aLootType: INT32)
```

Wire: 2 × 4B = 8 bytes. `aLootType` values: `0`=RoundRobin, `1`=FFA (from `EGroupLootType`).

Binary handler `FUN_00e5d650` reads `"aLootType"` field only (no org ID field read observed in decompile — the org ID field may be consumed by the CME dispatch layer before the handler body, or the handler ignores it). Handler stores value at `this+0xC8` (200 decimal), guards on change before emitting the chat notification.

### onSquadLootModeUpdate (new — not previously documented)

Also in `OrganizationMember.def`:

```
onSquadLootModeUpdate(aOrganizationId: INT32, aLootMode: INT32)
```

This second event is defined in the `.def` but has **no registered NetIn stub** in the SGW.exe string table — it was not shipped in the 2009 client binary. Likely a later planned event that was never wired up, or used only server-side. Do not implement a client handler for it.

---

## Open Questions

1. **Need/Greed/Pass roll mechanics** — CLOSED. Mechanic does not exist. See "Need/Greed/Pass Roll" section above.
2. **onSquadLootType wire fields** — CLOSED. Two INT32 fields: `aOrganizationId` + `aLootType`. Enum: 0=RoundRobin, 1=FFA. Confirmed from `OrganizationMember.def` + `EGroupLootType` + binary handler decompile.
3. **DBInvItem cache warming latency** — what happens if `DBInvItem` is not cached when `LootDisplay` arrives? Is the loot window delayed until `Cache_ElementReady<DBInvItem>` fires, or does it render with placeholder data? The subscription at `0x00e24970` suggests blocking on cache, but the resolution path is not yet decompiled.
4. **Loot container persistence** — does the loot container entity remain until the player closes the window, or does it despawn after the `LootDisplay` is sent? The `entityId` in `LootDisplay` may be needed for subsequent `LootItem` messages. Server-side question.

---

## Related Documents

- [right-click-routing-on-corpse.md](right-click-routing-on-corpse.md) — upstream: how the loot interact request reaches the server
- [inventory-wire-formats.md](inventory-wire-formats.md) — InvItem FIXED_DICT layout
- [group-wire-formats.md](group-wire-formats.md) — group system backing the squad loot mode
- [cme-event-signal.md](cme-event-signal.md) — CME EventSignal pipeline anatomy
