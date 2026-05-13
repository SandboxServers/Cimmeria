# Inventory State Machine — Client-Side Analysis

> **Session**: W-inventory-state (Session 4b, 2026-05-12/13)
> **Binary**: SGW.exe (32-bit x86 PE, MSVC 8.0 / VC80)
> **Image base**: `0x00400000`
> **Cross-links**: [`weapon-ammo-pipeline.md`](weapon-ammo-pipeline.md), [`../../content/equip-from-inventory-pattern.md`](../../content/equip-from-inventory-pattern.md), [`../address-map.md`](../address-map.md)

---

## Overview

The SGW client manages player inventory through a C++ `Inventory` class that subscribes to 14 CME event signals. The class acts as a client-side model — it receives S→C messages (`onContainerInfo`, `onUpdateItem`, `onRemoveItem`, `onRefreshItem`, `onActiveSlotUpdate`, `onClearOrgVaultInventory`), maintains in-memory item trees keyed by container ID + slot ID, and emits C→S messages (`moveItem`, `requestActiveSlotChange`, `requestAmmoChange`, `removeItem`, `useItem`, `repairItemRequest`) through the universal RPC dispatcher at `0x00c6fc40`.

There is **no separate equip wire message**. Equipping and unequipping are implemented entirely as `moveItem(itemId, targetContainerId, targetSlotId, quantity=1)` calls. Dragging an item to an equipment slot sends `moveItem` with the appropriate equipment container ID (4–14). Dragging from an equipment slot to inventory sends `moveItem` back to container 1 (main) or 2 (mission).

The SGWTextCommandMgr class (ctor at `0x00c8d0f0`) provides slash command wrappers (`/equipitem`, `/unequipitem`, `/activatebandolierslot`) that route through the same underlying emit functions.

---

## Container Type Enum

Container IDs are integers 1–20. They are transmitted on the wire inside `InvItem` FIXED_DICT fields (`containerID` and `slotID`) and inside `moveItem` arguments (`targetContainerId`, `targetSlotId`). They map directly to `crates/entity/src/inventory.rs` constants.

| Container ID | Constant | Slot Count | Notes |
|---|---|---|---|
| 1 | `INV_MAIN` | 40 | Primary player inventory bag |
| 2 | `INV_MISSION` | 100 | Mission-granted items; server-controlled only |
| 3 | `INV_BANDOLIER` | 4 | Active weapon slots; see Bandolier section |
| 4 | `INV_HEAD` | 1 | Equipment — head |
| 5 | `INV_FACE` | 1 | Equipment — face |
| 6 | `INV_NECK` | 1 | Equipment — neck |
| 7 | `INV_CHEST` | 1 | Equipment — chest |
| 8 | `INV_HANDS` | 1 | Equipment — hands |
| 9 | `INV_WAIST` | 1 | Equipment — waist |
| 10 | `INV_BACK` | 1 | Equipment — back |
| 11 | `INV_LEGS` | 1 | Equipment — legs |
| 12 | `INV_FEET` | 1 | Equipment — feet |
| 13 | `INV_ARTIFACT1` | 1 | Equipment — artifact slot 1 |
| 14 | `INV_ARTIFACT2` | 1 | Equipment — artifact slot 2 |
| 15 | `INV_CRAFTING` | 100 | Crafting table buffer |
| 16 | `INV_BUYBACK` | 12 | Vendor buyback history |
| 17 | `INV_BANK` | — | Personal bank (not confirmed slot count) |
| 18 | `INV_AUCTION` | — | Auction house staging |
| 19 | `INV_TEAM_BANK` | — | Team/guild bank |
| 20 | `INV_COMMAND_BANK` | — | Command bank |

**Evidence**: `crates/entity/src/inventory.rs` constants; slot counts from `crates/services/src/base/resources.rs` `item_allows_container` table.

**Equipment slots**: Container IDs 4–14 correspond to the 11 equipment body slots. Each has capacity 1 (single item). Moving an item to one of these containers via `moveItem` constitutes an equip operation. The server validates that the item's `itemType` permits the target container via the `item_allows_container` check in `move_/mod.rs`.

---

## Equip / Unequip State Machine

### No dedicated equip message

Searching the binary for `NetOut.*Equip` and `NetOut.*Unequip` patterns returned zero results. There is no `Event_NetOut_EquipItem` or `Event_NetOut_UnequipItem` signal in `RegisterBulkNetOutSignals` (`0x00db3390`). The client and server both use `moveItem` (method index 38) exclusively.

### Client-side equip path

1. **UI drag or slash command**: The player drags an item from `INV_MAIN`/`INV_MISSION` to an equipment slot graphic, or types `/equipitem <name>`.

2. **Slash command handler** (`FUN_00c73da0` → `SGWTextCmdMgr_HandleEquipItem`):
   - Reads `ItemName` (wide string) from `Event_SlashCmd_EquipItem` CME event
   - Calls `FUN_00e1f420` — client-side equip-by-name on the Inventory model
   - `FUN_00e1f420` looks up the item by name, determines the correct equipment container, calls `EmitNetOut_MoveItem`

3. **EmitNetOut_MoveItem** (`FUN_00e1e340`): Pattern A emitter (GetSystem+LookupByName+SetField×4).
   - Fields written: `ItemId`, `TargetBag` (target container ID), `TargetSlot` (target slot, 1-indexed), `Quantity` (=1 for equip)
   - Routes through `CmeEventSignal_Emit_Subscribe` → universal RPC dispatcher at `0x00c6fc40`
   - Wire format: cell method header `0x80 | 38`, then FIXED_DICT args

4. **Server receives `moveItem`** (method index 38):
   - Acquires per-player advisory lock
   - Fetches source item row FOR UPDATE
   - Calls `item_allows_container(item_def, target_container_id)` to validate the equip slot
   - If target slot is occupied: performs occupant swap (moves existing item to source slot)
   - Updates `inv_items` row: `container_id=target, slot_id=target_slot`

5. **Server sends `onUpdateItem`**: Confirms new position to client. The `Inventory_HandleOnUpdateItem` handler (`FUN_00e1fd30`) updates the in-memory item map.

### Unequip path

`FUN_00c73ee0` (`SGWTextCmdMgr_HandleUnequipItem`) is structurally identical to the equip handler. It calls `FUN_00e1f480` (unequip-by-name), which calls `EmitNetOut_MoveItem` with `TargetBag=1` (main inventory) and a free slot in that bag.

### Equip-with-swap (right-click behavior, issue #240)

When the player right-clicks an item to equip it and the target equipment slot is already occupied, the server performs the swap atomically:

1. Reads occupant item from target slot (FOR UPDATE lock on that row too)
2. Moves occupant to source slot (or to `INV_MAIN` if source was already an equipment slot)
3. Moves new item to target slot
4. Sends two `onUpdateItem` events — one for each moved item

The swap logic lives in `crates/services/src/base/world_entry/methods/inventory/move_/mod.rs`. Issue #240 tracks a bug where right-click swap puts the swapped-out item into an inconsistent slot; the binary confirms both items should be resolved in a single transaction.

---

## Inventory Class Event Subscriptions

`FUN_00e20da0` (`Inventory_Init`) is the Inventory class initializer. It registers 14 CME event signal subscriptions, mapping S→C event names to handler functions. All handlers follow the `MemberCallback` pattern (12-byte object: vtable+subscriber+method_ptr) dispatched by `CmeEventSignal_InvokeMemberCallback` at `0x00e04570`.

| CME Event Name | Handler Address | Handler Name |
|---|---|---|
| `Event_NetIn_onContainerInfo` | `FUN_00e1f6b0` | `Inventory_HandleOnContainerInfo` |
| `Event_NetIn_onActiveSlotUpdate` | `FUN_00e1fb20` | `Inventory_HandleOnActiveSlotUpdate` |
| `Event_NetIn_onUpdateItem` | `FUN_00e1fd30` | `Inventory_HandleOnUpdateItem` |
| `Event_NetIn_onRemoveItem` | `FUN_00e1da00` | `Inventory_HandleOnRemoveItem` |
| `Event_NetIn_onRefreshItem` | `FUN_00e1db80` | `Inventory_HandleOnRefreshItem` |
| `Event_NetIn_onClearOrgVaultInventory` | `FUN_00e1dcc0` | `Inventory_HandleOnClearOrgVaultInventory` |
| `Event_NetIn_onCashChanged` | (unknown) | — |
| + 7 additional subscriptions | (not decompiled) | — |

**Evidence chain**: MemberCallback ctor at `0x00e21ce0` (for `onUpdateItem` subscriber) was traced to its caller `FUN_00e224e0`, then to `FUN_00e20da0` which contains all 14 subscription setups.

### Inventory_HandleOnUpdateItem (`FUN_00e1fd30`)

This is the most complex S→C handler (~600 lines of decompiled C). Key structure:

1. Reads `ItemUpdates` ARRAY field from the event — variable count
2. For each entry, extracts 9+ named fields:
   - `id` via param at `0x019d8134`
   - `dbid` via param at `0x019d8138`
   - `stackSize`, `durability`, `slotID`, `containerID`, `isBound`, `curAmmoType`
   - `ammoTypes[]` — variable-length array of ammo type IDs
3. Calls `FUN_00d21750` to create or update an item object in the Inventory map
4. Fires internal CME events to notify UI widgets of item changes

### Inventory_HandleOnContainerInfo (`FUN_00e1f6b0`)

Receives the `onContainerInfo` batch message that the server sends on world entry and after any bag operation:

1. Reads `Bags` ARRAY — one `BagInfo` per container (container ID + slot count)
2. Reads `Items` ARRAY — full `InvItem` FIXED_DICT for each item

Wire format matches `docs/reverse-engineering/findings/inventory-wire-formats.md`:
- BagInfo: 8 bytes (4B bagId + 4B slotCount)
- InvItem: 37 + 4×M bytes (M = ammoTypes count)

### Inventory_HandleOnActiveSlotUpdate (`FUN_00e1fb20`)

Receives `onActiveSlotUpdate(bagId, slotId)` after a `requestActiveSlotChange` is honored by the server:

1. Reads `BagId` (i32) and `SlotId` (i32) from event
2. Updates the active bandolier slot tracking in the Inventory model
3. Triggers weapon visual/animation swap in the UI

---

## Bandolier Ammo Cycling

The bandolier (container ID 3, 4 slots) holds the player's equipped weapons. Each slot corresponds to a weapon loadout position.

### Active Slot Change

**Client emitter**: `FUN_00e1ef70` (`EmitNetOut_RequestActiveSlotChange`) — Pattern A, sets two fields:
- `BagId`: hardcoded to 3 (from `DAT_00000003`)
- `SlotId`: 1-indexed wire slot ID

**Slash command handler** `FUN_00c74d20` (`SGWTextCmdMgr_HandleActivateBandolierSlot`):
- Reads `SlotNum` from `Event_SlashCmd_ActivateBandolierSlot`
- Passes `BagId=3, SlotNum` directly to `EmitNetOut_RequestActiveSlotChange`

**Wire format** (cell method index 41 = `REQUEST_ACTIVE_SLOT_CHANGE`):
```
Header:  0xA9  (0x80 | 41)
Payload: 4B BagId (i32 LE) + 4B SlotId (i32 LE)
```

**Server side** (`crates/services/src/cell/cell_methods/inventory/bandolier.rs`):
- Receives 1-indexed `SlotId`
- Converts: `wire_slot_id.saturating_sub(1)` → 0-indexed
- Updates active slot in player state
- Sends `onActiveSlotUpdate(BagId=3, SlotId=new_slot_1indexed)` back to client
- Sends `onEntityProperty(propId=3, curAmmoType)` to broadcast active weapon's ammo type

**Evidence**: `Bag.py:369` comment in weapon-ammo-pipeline.md confirms 1-indexed wire convention. Binary handler at `FUN_00c74d20` calls `FUN_00e1ef70(&DAT_00000003, SlotNum)` directly.

### Ammo Type Change

**Client emitter**: Method index 42 = `REQUEST_AMMO_CHANGE`.

**Wire format** (from `inventory-wire-formats.md`):
```
Header:  0xAA  (0x80 | 42)
Payload: 4B ItemId (i32 LE) + 4B AmmoType (i32 LE)
```

**Server side**: Updates `curAmmoType` on the weapon item, then broadcasts `onEntityProperty(propId=3, newAmmoType)`.

### Slot ID conventions

| Layer | Indexing | Notes |
|---|---|---|
| Wire (C→S) | 1-indexed | Client sends 1-based slot numbers |
| Server (Rust) | 0-indexed | `saturating_sub(1)` on receipt |
| Wire (S→C) | 1-indexed | Server sends 1-based in `onActiveSlotUpdate` |
| DB | 0-indexed | Stored 0-based in `inv_items.slot_id` |

---

## SGWTextCommandMgr Inventory Handlers

`FUN_00c8d0f0` (`SGWTextCommandMgr_Ctor`) registers ~130 slash command handlers. The 6th through 8th inventory-related subscriptions (by position in the constructor):

| Handler Address | Renamed To | Slash Command | Behavior |
|---|---|---|---|
| `FUN_00c73da0` | `SGWTextCmdMgr_HandleEquipItem` | `/equipitem <name>` | Reads `ItemName`, calls `FUN_00e1f420` (equip-by-name on Inventory model) |
| `FUN_00c73ee0` | `SGWTextCmdMgr_HandleUnequipItem` | `/unequipitem <name>` | Reads `ItemName`, calls `FUN_00e1f480` (unequip-by-name) |
| `FUN_00c74d20` | `SGWTextCmdMgr_HandleActivateBandolierSlot` | `/activatebandolierslot <n>` | Reads `SlotNum`, calls `EmitNetOut_RequestActiveSlotChange(bagId=3, slotNum)` |

**Registration vtable**: `0x019b174c` (`MemberCallbackRtti_SlashCmd_EquipItem__SGWTextCommandMgr`). MemberCallback ctor at `0x00c96db0`, chained via `FUN_00c9d8c0` → `FUN_00c8d0f0`.

---

## Key Emit Functions

| Address | Renamed To | Pattern | Fields Set | Wire Method |
|---|---|---|---|---|
| `FUN_00e1e340` | `EmitNetOut_MoveItem` | A (GetSystem+LookupByName+SetField×4) | `ItemId`, `TargetBag`, `TargetSlot`, `Quantity` | 38 (`MOVE_ITEM`) |
| `FUN_00e1ef70` | `EmitNetOut_RequestActiveSlotChange` | A (GetSystem+LookupByName+SetField×2) | `BagId`, `SlotId` | 41 (`REQUEST_ACTIVE_SLOT_CHANGE`) |

Both emit to the CME event bus via `CmeEventSignal_Emit_Subscribe` (`0x00caf850`), which wraps into a 0x18-byte container and dispatches through the universal RPC dispatcher at `0x00c6fc40`.

---

## CallbackImpl RTTI Cluster — Inventory

The inventory NetIn CallbackImpl cluster spans `0x00e219b0 – 0x00e21a10` (from address-map.md session 3 findings). Functions in this range are RTTI type-name accessors (vfunc_2 pattern) for the inventory event signal subscriber objects. Uniform 0x10 spacing within the cluster.

---

## Wire Format Summary

Wire formats are documented in detail in [`inventory-wire-formats.md`](inventory-wire-formats.md). Key points for state machine:

| Message | Direction | Method Index | Payload |
|---|---|---|---|
| `moveItem` | C→S | 38 | 4B itemId + 4B targetBag + 4B targetSlot + 4B quantity |
| `removeItem` | C→S | 36 | 4B itemId |
| `listItems` | C→S | 37 | (empty) |
| `useItem` | C→S | 39 | 4B itemId |
| `repairItemRequest` | C→S | 40 | 4B itemId |
| `requestActiveSlotChange` | C→S | 41 | 4B bagId + 4B slotId (1-indexed) |
| `requestAmmoChange` | C→S | 42 | 4B itemId + 4B ammoType |
| `onContainerInfo` | S→C | — | BagInfo[] + InvItem[] FIXED_DICTs |
| `onUpdateItem` | S→C | — | InvItem[] FIXED_DICTs (partial update) |
| `onRemoveItem` | S→C | — | 4B itemId |
| `onRefreshItem` | S→C | — | 4B itemId (triggers re-request) |
| `onActiveSlotUpdate` | S→C | — | 4B bagId + 4B slotId (1-indexed) |

---

## Confirmed Non-Existence: Force-Equip to Bandolier

The content pattern at `docs/content/equip-from-inventory-pattern.md` notes that Cimmeria does NOT direct-grant to bandolier (container 3) because `sync_bandolier_after_inventory_change` would be bypassed. The binary confirms this: there is no `Event_NetOut_ForceEquip` or equivalent signal. Server-side mission scripts must grant to `INV_MAIN` (container 1) and let the player move the item to bandolier via `moveItem`.

---

## Open Questions

1. **`onRefreshItem` semantics**: `FUN_00e1db80` (`Inventory_HandleOnRefreshItem`) is not fully decompiled. Does it re-request the full item state from the server, or does it only trigger a UI redraw? The function name implies a lightweight "data is stale, please re-query" pattern.

2. **`onClearOrgVaultInventory` trigger**: `FUN_00e1dcc0` presumably clears container 19 or 20 (team/command bank). The trigger conditions (org disbandment? logout?) are not established from binary alone.

3. **7 unidentified Inventory_Init subscriptions**: `FUN_00e20da0` registers 14 subscriptions total; only 7 handler functions are confirmed above. The remaining 7 may cover: `onStoreOpen`, `onStoreUpdate`, `onStoreClose`, `onCashChanged`, and 3 others. The CallbackImpl RTTI cluster at `0x00e219b0–0x00e21a10` covers `onContainerInfo` through `onCashChanged` (6 entries per session-3 address-map) — the remaining subscriptions are likely outside this range.

4. **Right-click-to-equip UI path**: The slash command path (`/equipitem`) is confirmed. The drag-and-drop UI path invokes `EmitNetOut_MoveItem` through a different call site not traced here. Issue #240 (equip-with-swap) bug root cause is in the Rust `move_item` handler logic, not in the client wire format.

5. **Bank/auction slot counts**: `INV_BANK` (17), `INV_AUCTION` (18), `INV_TEAM_BANK` (19), `INV_COMMAND_BANK` (20) slot counts are not in `resources.rs`. The `item_allows_container` check may not cover these containers yet.
