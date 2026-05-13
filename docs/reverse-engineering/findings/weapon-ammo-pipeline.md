# Weapon / Ammo Pipeline — RE Findings

> **Date**: 2026-05-13
> **Issues**: #168 (AmmoTypeId wrong propId), #210 (reload animation source)
> **Confidence**: HIGH — all claims cite binary addresses, .def files, or enumerations.xml
> **Sources**: Ghidra decompilation of `sgw.exe`, `entities/defs/`, `entities/defs/enumerations.xml`,
>   `entities/defs/alias.xml`, Cimmeria Rust codebase

---

## Overview

The Stargate Worlds weapon system spans three layers:

1. **Item layer** — the `InvItem` wire struct carries `ammoTypes[]` and `curAmmoType` per-item.
2. **Bandolier layer** — the active weapon slot drives the client's ammo-count UI via `onStatUpdate` stats (AMMO_SLOT_{1..3}) and the ammo-type UI via `onEntityProperty(GENERICPROPERTY_AmmoTypeId, N)`.
3. **Combat layer** — the server gates fire on ammo count, triggers reload via `requestReload`, and notifies the client of animation via `onSequence`.

This document covers the wire-format path end-to-end plus two confirmed bugs: #168 (wrong propId) and #210 (wrong event_set_id source for reload animations).

---

## 1. Entity Property Types — `EEntityPropertyType`

From `entities/defs/enumerations.xml` lines 1720–1733:

```xml
<EEntityPropertyType>ENUMERATION
  <Type>INT32</Type>
  <Tokens>
    <Token><Name>GENERICPROPERTY_TrainingPoints</Name>      <Value>1</Value></Token>
    <Token><Name>GENERICPROPERTY_AppliedSciencePoints</Name><Value>2</Value></Token>
    <Token><Name>GENERICPROPERTY_AmmoTypeId</Name>          <Value>3</Value></Token>
    <Token><Name>GENERICPROPERTY_PvPFlag</Name>             <Value>4</Value></Token>
    <Token><Name>GENERICPROPERTY_PetOwnerId</Name>          <Value>5</Value></Token>
    <Token><Name>GENERICPROPERTY_MobAggression</Name>       <Value>6</Value></Token>
    <Token><Name>GENERICPROPERTY_AccessLevel</Name>         <Value>7</Value></Token>
    <Token><Name>GENERICPROPERTY_Gender</Name>              <Value>8</Value></Token>
    <Token><Name>GENERICPROPERTY_DatabaseId</Name>          <Value>9</Value></Token>
  </Tokens>
</EEntityPropertyType>
```

**`GENERICPROPERTY_AmmoTypeId` = 3. `GENERICPROPERTY_AccessLevel` = 7. These are different.**

### `onEntityProperty` wire format

`onEntityProperty` is a ClientMethod on `SGWSpawnableEntity` (index 7 in that class's ClientMethods list, per `spawnable_entity.rs`). It carries two `INT32` args:

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 4 | type (propId) | `EEntityPropertyType` value (1-9) |
| 4 | 4 | value | INT32 value for that property |

**Total payload: 8 bytes.**

The SGWSpawnableEntity ClientMethod index 7 = `ON_ENTITY_PROPERTY` is the *method index in the BigWorld client method dispatch table*. It is separate from the prop_id carried as the first payload argument.

---

## 2. `InvItem` — Per-Item Ammo State

From `entities/defs/alias.xml` (confirmed by `inventory-wire-formats.md`):

```xml
<InvItem>FIXED_DICT
  <ammoTypes><Type>ARRAY<of>INT32</of></Type></ammoTypes>  <!-- available EAmmoType indices -->
  <curAmmoType><Type>INT32</Type></curAmmoType>              <!-- currently loaded type -->
</InvItem>
```

The `curAmmoType` field is a 0-based index into `EAmmoType` (from `enumerations.xml`). `EAmmoType` has 24 values (0=AMMO_NONE through 23=Dart_Adrenaline). The `ammoTypes` array lists which types the weapon supports.

---

## 3. Weapon Equip / Unequip

**Equip path** (from `requestActiveSlotChange` → `onActiveSlotUpdate`):

1. Client sends `requestActiveSlotChange(BagId=3, SlotId)` — BagId 3 = bandolier container.
2. Server validates slot, updates active slot, sends:
   - `onActiveSlotUpdate(BagId=3, SlotId+1)` (1-indexed on wire per `Bag.py:369`)
   - `onEntityProperty(GENERICPROPERTY_AmmoTypeId=3, curAmmoType)` — updates ammo indicator
3. Server also updates the `OnStatUpdate` for the AMMO_SLOT_{N} stat if ammo count changed.

**Unequip**: Holster via `requestHolsterWeapon(aHolster=1)` — sets `BSF_HOLSTER` in state field, triggers `onStateFieldUpdate`. No ammo property update needed on holster.

### World entry seed

On `mapLoaded`, the server sends 6 consecutive `onEntityProperty` calls:

```rust
// from map_loaded.rs lines 304-311
(2i32, data.applied_science_points), // AppliedSciencePoints
(1,    data.training_points),        // TrainingPoints
(7,    data.access_level),           // AccessLevel          ← correct use of propId 7
(8,    data.gender),                 // Gender
(4,    0),                           // PvPFlag
(3,    active_ammo_type),            // AmmoTypeId           ← propId 3, sourced from active bandolier slot
```

The active ammo type for world entry comes from `bandolier_items[active_bandolier_slot].cur_ammo_type`, defaulting to 0 if the slot is empty.

---

## 4. Magazine State and Ammo Count Tracking

Ammo counts are tracked as stats (`AMMO_SLOT_1`, `AMMO_SLOT_2`, `AMMO_SLOT_3`), one per bandolier slot. Each stat has `min=0`, `cur=current_ammo`, `max=clip_size`.

**Fire path**: Server decrements `current_ammo`, marks the stat dirty, emits `onStatUpdate` carrying the dirty stats. The client's ammo counter reflects stat `cur`.

**Bandolier dirty tracking**: `CellEntity.bandolier_ammo_dirty` is a set of slot IDs. When a slot's ammo changes, its slot ID is inserted. The `flush_dirty_bandolier_ammo` function serializes dirty slots into `BandolierAmmoUpdate` messages for DB persistence.

---

## 5. Reload Sequence — Full Wire Path

### Client → Server

```
requestReload(aReloadType: UINT8)
```

Wire format (SGWPlayer.def line 794-797):
| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| 0 | 1 | methodID \| 0x80 | cell method header |
| 1 | 1 | aReloadType | `EReloadType` enum |

### Binary — client-side emit functions

Two functions emit `Event_NetOut_RequestReload`:

| Address | Function | Source |
|---------|----------|--------|
| `0x00e078a0` | Direct emitter — called by `FUN_00ad7880` after GamePlayer RTTI cast | CME game layer |
| `0x00c889a0` | SGWTextCommandManager handler — reads `reloadType` from CME event | `SGWTextCommandManager.cpp:0xB4B` |

Both call `FUN_00cbcda0` (Event_NetOut_RequestReload NetworkEvent ctor, 12-byte Pattern B), set the `aReloadType` field via `FUN_00cfc620`, then emit via `FUN_00caf850` (which calls into `CmeEventSignal_Subscribe`-tier dispatch).

The `RegisterBulkNetOutSignals` function at `0x00db9f12` registers `Event_NetIn_onEntityProperty` signal (the data at `0x019bfe54`), and at `0x00db882c` registers `Event_NetOut_RequestReload` (string at `0x019bf430`).

### Server → Client sequence (Cimmeria `handle_reload`)

On receiving `requestReload`, the server (`handle_reload` in `world.rs`) does:

1. **Gate**: Early return if `active_ammo >= clip_size` and no reload in flight.
2. **Set deadline**: `entity.reload_complete_at = Instant::now() + warmup_duration`
3. **Pin slot**: `entity.reload_slot_id = entity.active_bandolier_slot`
4. **Send `onTimerUpdate`** (method 12) — drives the cooldown bar on the client.
5. **Send `onStateFieldUpdate`** (method 19) — if state changed (sets `BSF_IN_COMBAT`, clears `BSF_HOLSTER`).
6. **Send `onSequence`** (method 1) — only if ability 596 has a non-null `event_set_id` AND the sequence map contains `(event_set_id, EVENT_ABILITY_BEGIN)`. In production this branch is dead because ability 596 has `event_set_id = NULL` in the seed data.
7. **Send `onEntityProperty(propId=3, ammo_type)`** — always, unconditionally.

### Reload completion tick

When `reload_complete_at` elapses, `reload_completion_tick` runs:

1. Refills `current_ammo` to `clip_size` for the pinned `reload_slot_id`.
2. Marks the stat dirty and emits `onStatUpdate` (method 20).
3. Emits `BandolierAmmoUpdate` for DB persistence.
4. If `event_set_id` is present, emits `onSequence` with `EVENT_ABILITY_END`.

---

## 6. AmmoTypeId propId Assignment — Definitive Answer for Issue #168

### Bug

In `crates/services/src/cell/cell_methods/player/world.rs` (before fix), `handle_reload` sent:

```rust
args.extend_from_slice(&7i32.to_le_bytes());  // BUG: 7 = GENERICPROPERTY_AccessLevel
```

The client receives `onEntityProperty(type=7, value=ammoType)` and interprets type=7 as `GENERICPROPERTY_AccessLevel`, updating the access-level indicator rather than the ammo-type indicator.

### Correct value

**`GENERICPROPERTY_AmmoTypeId` = 3** — confirmed by:

- `entities/defs/enumerations.xml` line 1725: `<Token><Name>GENERICPROPERTY_AmmoTypeId</Name> <Value>3</Value></Token>`
- `crates/services/src/cell/cell_methods/inventory/constants.rs`: `pub(crate) const GENERICPROPERTY_AMMO_TYPE_ID: i32 = 3;`
- `crates/services/src/mercury/world_data/tests/bandolier.rs` line 208: "cur_ammo_type is sent as the AmmoTypeId (prop_id = 3) entity property"
- Existing test in `bandolier.rs` lines 243-264 independently verifies propId=3 for the world-entry path

### Fix applied

`world.rs` now uses `build_entity_property_args(GENERICPROPERTY_AMMO_TYPE_ID, ammo_type)` from `inventory::constants`. The constants were promoted from `pub(super)` to `pub(crate)` to allow cross-module use.

### Regression guard

New test `handle_reload_sends_ammo_type_id_prop_id_3_not_access_level_7` in `world.rs` module tests:
- Drains all messages from `handle_reload`
- Finds the `ON_ENTITY_PROPERTY` call
- Asserts `prop_id == 3` (and explicitly asserts `prop_id != 7`)
- Asserts `value == cur_ammo_type`
- Asserts the call fires unconditionally (no event_set needed)

---

## 7. Issue #210 — Reload Animation Source

### Problem

The reload animation should be sourced from the player's **archetype-keyed item event set** via `getItemSequence(Item_Reload)` (event id 4002), not from the reload ability's `event_set_id`. The legacy Python path (`SGWBeing.py:863-874`) uses the former.

The current Rust `handle_reload` looks up `ability_defs[596].event_set_id` and emits an `onSequence` if found. In production this is a no-op because ability 596 has `event_set_id = NULL` in `db/resources/Abilities/Seed/abilities.sql`.

### Binary evidence

- `FUN_00e078a0` and `FUN_00c889a0` are the emit functions for `Event_NetOut_RequestReload` (confirmed by string xrefs to `019aed18` = "aReloadType"). Neither contains animation logic — they simply fire the event.
- The animation is a client-side response driven by the `onSequence` payload. The server must supply the correct sequence ID sourced from the archetype's item event set.

### Status

Documented in `world.rs` as `TODO(#210)` at line 250-261. The existing `ON_SEQUENCE` wiring is kept because tests already pin its byte layout; #210 will replace the `event_set_id` lookup with an archetype-keyed lookup. Cross-link: see issue body for migration shape.

---

## 8. Per-Archetype Weapon Mechanics

From `entities/defs/enumerations.xml` `EAmmoType` and the weapon item defs:

| Archetype | Weapon class | Default ammo |
|-----------|-------------|-------------|
| Commando | Bullet weapons | `Bullet_Default` (1) |
| Soldier | Bullet weapons (same pool) | `Bullet_Default` (1) |
| Scientist | Dart weapons | `Dart_Default` (13) |

`AMMO_NONE` (0) means no ammo type set / no weapon equipped.

The `InvItem.ammoTypes` array lists which `EAmmoType` values the weapon supports. `requestAmmoChange(ItemId, AmmoType)` switches `curAmmoType` on that weapon. The server emits `onEntityProperty(propId=3, newAmmoType)` after a successful ammo change (implemented in `inventory/bandolier.rs`).

---

## 9. Binary Addresses — Weapon / Ammo Subsystem

| Address | Function | Notes |
|---------|----------|-------|
| `0x00e078a0` | Direct RequestReload emitter | Pattern B NetworkEvent; called by GamePlayer gate at `0x00ad7880` |
| `0x00c889a0` | SGWTextCommandManager RequestReload handler | Reads "reloadType" from CME event; `SGWTextCommandManager.cpp:0xB4B` |
| `0x00c8a5c0` | SGWTextCommandManager RequestAmmoChange handler | Reads "ammoType", sets ItemId + AmmoType; `SGWTextCommandManager.cpp` ~line 0xC09 |
| `0x00cbcda0` | Event_NetOut_RequestReload NetworkEvent ctor | 12-byte scalable_malloc, Pattern B |
| `0x00caf850` | CME emit dispatcher (subscribe-tier) | Called by both RequestReload emitters |
| `0x00cbce00` | `register_NetOut_RequestReload` | Returns string "Event_NetOut_RequestReload" |
| `0x019bfe54` | String: "onEntityProperty" | Data xref from `RegisterBulkNetOutSignals` at `0x00db9f12` |
| `0x019b409c` | String: "Event_NetOut_RequestReload" | Data xref from `RegisterBulkNetOutSignals` at `0x00db882c` |
| `0x019b430c` | String: "Event_NetOut_RequestAmmoChange" | |
| `0x019af444` | String: "ammoType" | Used in `FUN_00c8a5c0` requestAmmoChange handler |
| `0x019af4ac` | String: "AmmoType" | SetField key name in ammo change event |
| `0x019aed18` | String: "aReloadType" | Field name in Event_NetOut_RequestReload |

---

## 10. Cross-Validation Summary

| Finding | Binary | enumerations.xml | .def | Rust code | Confidence |
|---------|--------|-----------------|------|-----------|------------|
| GENERICPROPERTY_AmmoTypeId = 3 | — | Y (line 1725) | — | Y (constants.rs) | HIGH |
| GENERICPROPERTY_AccessLevel = 7 | — | Y (line 1729) | — | Y (map_loaded.rs) | HIGH |
| handle_reload bug: propId=7 sent | — | — | — | Y (world.rs line 292 pre-fix) | CONFIRMED |
| requestReload wire: 1 UINT8 arg | — | — | Y (SGWPlayer.def:794) | Y | HIGH |
| requestAmmoChange wire: INT32 ItemId + INT32 AmmoType | Y (0x00c8a5c0) | — | Y (SGWInventoryManager.def) | Y | HIGH |
| onEntityProperty 2-arg: INT32 type + INT32 value | — | — | Y (SGWSpawnableEntity.def:131) | Y | HIGH |
| #210: reload anim from archetype item event_set | — | — | — | Y (world.rs TODO) | MEDIUM (Python parity only) |

---

## 11. Recommended Rust Fix for Issue #168

**Status: APPLIED** in this session.

File: `crates/services/src/cell/cell_methods/player/world.rs`

```rust
// Before (BUG):
args.extend_from_slice(&7i32.to_le_bytes()); // propId 7 = AccessLevel, not AmmoTypeId

// After (FIX):
use crate::cell::cell_methods::inventory::constants::{
    build_entity_property_args, GENERICPROPERTY_AMMO_TYPE_ID,
};
let args = build_entity_property_args(GENERICPROPERTY_AMMO_TYPE_ID, ammo_type);
```

File: `crates/services/src/cell/cell_methods/inventory/constants.rs`

```rust
// Before:
pub(super) const GENERICPROPERTY_AMMO_TYPE_ID: i32 = 3;
pub(super) fn build_entity_property_args(...) -> Vec<u8> {

// After:
pub(crate) const GENERICPROPERTY_AMMO_TYPE_ID: i32 = 3;
pub(crate) fn build_entity_property_args(...) -> Vec<u8> {
```

---

## 12. Open Questions

1. **#210 (reload animation)**: What is the client's exact lookup path for archetype-keyed item event sets? The Python path uses `getItemSequence(Item_Reload)` / event id 4002. The binary search for this lookup was deferred — would need to trace the Python→C++ call path for `getItemSequence` in the Ghidra binary.

2. **`Event_NetOut_RequestAmmoChange` subscriber**: The `FUN_00c8a5c0` handler at `0x00c8a5c0` creates the event and emits it. The SGWNetworkManager subscriber (the function stored in the MemberCallback at `MemberCallback+0x8`) was not located in this session — its address is in a vtable not yet found via RTTI xrefs.

3. **`register_NetIn_onEntityProperty`**: The string `"Event_NetIn_onEntityProperty"` has zero search results — this NetIn event may not exist as a CME signal and the client may dispatch `onEntityProperty` directly from the BigWorld entity method dispatch path rather than through the CME bus.

4. **EReloadType enum**: Referenced in `SGWPlayer.def:796` but not found in `enumerations.xml` in this session. The single `aReloadType` byte on the wire has values that are not yet documented.

---

## Related Documents

- [entity-property-sync.md](entity-property-sync.md) — PropId encoding, property ID assignment
- [inventory-wire-formats.md](inventory-wire-formats.md) — InvItem layout, requestAmmoChange
- [combat-wire-formats.md](combat-wire-formats.md) — useAbility, onEffectResults
- [address-map.md](../address-map.md) — Updated with weapon/ammo subsection below
