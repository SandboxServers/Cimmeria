---
title: "Inventory System"
type: reference
audience: engineers
last_updated: 2026-07-25
---

# Inventory System

> **Last updated**: 2026-07-25
> **Status**: Implemented, including the full vendor stack. Remaining gaps are stat recalculation on equip and the organization vault.

## Overview

The inventory system manages item storage, equipping, movement, and currency for player entities. Items are organized into numbered bags (containers) with fixed slot counts. Each bag may represent general storage, equipment slots, crafting storage, or mission items. Equipped items contribute visual components to the player model and trigger equip/unequip callbacks.

Inventory splits across the two services: cell-side operations live in [`cell/cell_methods/inventory/`](../../crates/services/src/cell/cell_methods/inventory/) (item ops plus the bandolier/active-slot machinery), and everything that touches the database — including the entire vendor stack — lives in [`base/world_entry/methods/inventory/`](../../crates/services/src/base/world_entry/methods/inventory/) and [`base/world_entry/methods/vendor/`](../../crates/services/src/base/world_entry/methods/vendor/). Item definitions come from `db/resources/Items/`.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| Bag/slot storage | DONE | Multiple bags, configurable sizes |
| Item add/remove/move | DONE | Stack merging, slot swapping, quantity splitting |
| Item equipping | DONE | Active slot system with visual component updates and `Item_Equip` / `Item_Unequip` animations |
| Cash (naquadah) | DONE | Add/remove/sync to client |
| Database persistence | DONE | Load/save per character, `sgw_inventory` / `sgw_inventory_base` tables |
| Client sync (flush) | DONE | Batched updates: bags, items, removals, cash |
| Item use | DONE | Fires the item's ability binding and the `ItemUsed` chain trigger |
| Store open/close | DONE | `base/world_entry/methods/vendor/store.rs` |
| Store buy/sell | DONE | `vendor/purchase/`, `vendor/sell/` |
| Buyback | DONE | `vendor/buyback/` |
| Item repair (vendor) | DONE | `vendor/repair.rs` plus the paid-repair variant |
| Item recharge (vendor) | DONE | `vendor/recharge.rs` plus the paid-recharge variant |
| Vendor bag allowlist | DONE | `VENDOR_FILTER_BAGS` confines vendor operations to the main bag, bandolier, the eleven equipment slots, and the quick bar — the bank, mail attachments, and loot bags are unreachable |
| Item repair (direct) | NOT IMPL | `repairItemRequest` (the client-initiated cell method) decodes its args and logs `UNIMPLEMENTED`; repair only works through the vendor path |
| Stat recalculation on equip | NOT IMPL | `inventoryAdjustments` property exists |
| Organization vault | NOT IMPL | `onClearOrgVaultInventory`, `onOrgMoveItemResult` defined; blocked on the organization system |

## Entity Definition (SGWInventoryManager.def)

### Properties

| Property | Type | Flags | Purpose |
|----------|------|-------|---------|
| `playerBags` | PYTHON | CELL_PRIVATE | Bag dictionary |
| `activeSlots` | PYTHON | CELL_PRIVATE | Mapping of bagId to equipped slot |
| `inventoryAdjustments` | PYTHON | CELL_PRIVATE | Stat adjustments from equipped items |
| `pendingItemTransactions` | PYTHON | CELL_PRIVATE | Outstanding DB transaction tracking |
| `cash` | INT32 | CELL_PRIVATE | Current naquadah balance |
| `weaponActivationTimerID` | CONTROLLER_ID | CELL_PRIVATE | Weapon activation timer |
| `weaponDeactivationTimerID` | CONTROLLER_ID | CELL_PRIVATE | Weapon deactivation timer |
| `weaponActivated` | UINT8 | CELL_PRIVATE | Current weapon activation state |
| `inventoryComponents` | ARRAY\<WSTRING\> | CELL_PUBLIC | Visual components from equipped items |
| `knownAmmoTypes` | ARRAY\<INT32\> | CELL_PRIVATE | Discovered ammo types |
| `racialParadigmLevels` | PYTHON | CELL_PRIVATE | Crafting paradigm levels (shared with Crafter) |
| `appliedSciencePoints` | INT32 | CELL_PRIVATE | Crafting discipline points |
| `knownDisciplines` | PYTHON | CELL_PRIVATE | Learned crafting disciplines |
| `knownCrafts` | ARRAY\<INT32\> | CELL_PRIVATE | Known craft IDs |

### Client Methods (Server -> Client)

| Method | Args | Purpose |
|--------|------|---------|
| `onBagInfo` | ARRAY\<BagInfo\> | Send full bag list (id, slot count) |
| `onActiveSlotUpdate` | BagId, SlotId | Notify active slot change |
| `onRemoveItem` | ARRAY\<INT32\> | Notify item removals |
| `onUpdateItem` | ARRAY\<InvItem\> | Batch item updates |
| `onRefreshItem` | ItemId | Single item refresh |
| `onClearOrgVaultInventory` | OrganizationId | Clear org vault display |
| `onCashChanged` | cash | Currency balance update |

### Cell Methods (Client -> Server)

| Method | Exposed | Args | Purpose |
|--------|---------|------|---------|
| `removeItem` | YES | itemID, quantity | Delete item |
| `listItems` | YES | (none) | Request full inventory |
| `moveItem` | YES | itemId, targetBag, targetSlot, quantity | Move/swap item |
| `useItem` | YES | itemID, targetID | Use item on target |
| `repairItemRequest` | YES | itemId, repairRatio | Repair item (NOT IMPL) |
| `requestActiveSlotChange` | YES | BagId, SlotId | Change equipped slot |
| `requestAmmoChange` | YES | ItemId, AmmoType | Change ammo type |
| `giveCash` | NO | Amount | Server-side cash grant |
| `requestGiveItem` | NO | itemId, quantity, requireFull, callbackEntity, callbackRpc, callbackArgs | Server-side item grant |

## Bag Types (EInventoryContainerId)

| Enum | Purpose |
|------|---------|
| `INV_Main` | General inventory |
| `INV_Mission` | Mission-specific items |
| `INV_Crafting` | Crafting materials |
| `INV_Bandolier` | Weapon loadout (equipped) |
| `INV_Buyback` | Store buyback (session-only, not persisted) |
| `INV_CommandBank` | Upper bound / org vault |

## Bandolier and ammo

`INV_Bandolier` (container id `3`) holds 4 weapon slots indexed `0..3` and is the only container that tracks an active slot. Slot count matches legacy `deprecated/python/common/Constants.py:145` (`BAG_SIZES[INV_Bandolier] = 4`); there is no fist-weapon reservation, so all four slots are real weapon slots.

The wire format is **1-indexed** (slots `1..4`). Server-side everything is **0-indexed**; the cell decoder subtracts 1 on inbound `requestActiveSlotChange` / `moveItem` and the grant/sync paths add 1 on outbound `onActiveSlotUpdate`. Mismatch on the inbound side was the original cause of the "switching slots doesn't work" bug — see `crates/services/src/cell/cell_methods/inventory/bandolier.rs` and `item_ops.rs`.

Each bandolier slot persists not only the equipped item but also its **per-slot magazine state**:

| `sgw_inventory` column | Field | Purpose |
|------------------------|-------|---------|
| `ammo`                 | `BandolierItem.current_ammo` | Rounds remaining in this slot's magazine |
| `cur_ammo_type`        | `BandolierItem.cur_ammo_type` | Selected ammo subtype (defaults to item's `default_ammo_type`) |

Both columns are bandolier-slot-scoped — swapping weapons does not pool ammo across slots. The cell server mirrors `current_ammo` to `Stat[AMMO_SLOT_1+slot]` (stat IDs 49–53) so the client UI can subscribe to `Events.StatUpdated` for meter and count refresh.

Persistence is **batched**: dirty slots are flushed at reload completion, slot swap, ammo change, logout, and world transition. Full message flow, sequence diagrams, and legacy reference points are in [weapon-ammo-reload.md](weapon-ammo-reload.md).

## Flush Update Order

The `Inventory.flushUpdates()` method sends updates to the client in this order:

1. `onBagInfo` -- bag list (if bags changed)
2. Per-bag active slot updates
3. `onUpdateItem` -- all dirty items across all bags
4. `onCashChanged` -- naquadah balance
5. `onRemoveItem` -- removed item IDs
6. Visual component update (if equipped items changed)

## Data References

- **Item definitions**: 6,059 in `db/resources/Items/Seed/items.sql`
- **Schema**: `Item.xsd`
- **Persistence**: `sgw_inventory` table (character_id, type_id, bag_id, slot_id, quantity)
- **Bag sizes**: `common.Constants.BAG_SIZES`
- **Item classes**: `cell.Item`, `cell.Bag`

## RE Priorities

1. **Stat recalculation on equip** - How `inventoryAdjustments` feeds into the stat dictionary
2. **Store system** - Buy/sell/buyback flow and price calculation
3. **Item repair/recharge** - Durability system and cost formulas
4. **Organization vault** - Cross-entity item transfer protocol
5. **Stack splitting** - Partial quantity moves to occupied slots

## Related Docs

- [stat-system.md](stat-system.md) - Stats modified by equipped items
- [crafting-system.md](crafting-system.md) - Crafting uses inventory items
- [trade-system.md](trade-system.md) - Trading moves items between inventories
