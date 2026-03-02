# Inventory System

> **Last updated**: 2026-03-01
> **Status**: ~80% implemented

## Overview

The inventory system manages item storage, equipping, movement, and currency for player entities. Items are organized into numbered bags (containers) with fixed slot counts. Each bag may represent general storage, equipment slots, crafting storage, or mission items. Equipped items contribute visual components to the player model and trigger equip/unequip callbacks.

The `Inventory` class in `python/cell/Inventory.py` handles all inventory logic. Item definitions come from `db/resources.sql` and are resolved via `DefMgr.get('item', designId)`.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| Bag/slot storage | DONE | Multiple bags, configurable sizes via `BAG_SIZES` |
| Item add/remove/move | DONE | Stack merging, slot swapping, quantity splitting (partial) |
| Item equipping | DONE | Active slot system with visual component updates |
| Cash (naquadah) | DONE | Add/remove/sync to client |
| Database persistence | DONE | Load/save per character, `sgw_inventory` table |
| Client sync (flush) | DONE | Batched updates: bags, items, removals, cash |
| Item use | DONE | Delegates to item class and fires `item.use` event |
| Backup/restore | DONE | Full inventory snapshot for transactions |
| Stack splitting to occupied slot | NOT IMPL | `NotImplementedError` raised |
| Store buy/sell | PARTIAL | Store open/close events exist, buyback bag defined |
| Item repair | NOT IMPL | `repairItemRequest` defined but no handler |
| Item recharge | NOT IMPL | No recharge logic |
| Stat recalculation on equip | NOT IMPL | `inventoryAdjustments` property exists |
| Organization vault | NOT IMPL | `onClearOrgVaultInventory`, `onOrgMoveItemResult` defined |

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

## Flush Update Order

The `Inventory.flushUpdates()` method sends updates to the client in this order:

1. `onBagInfo` -- bag list (if bags changed)
2. Per-bag active slot updates
3. `onUpdateItem` -- all dirty items across all bags
4. `onCashChanged` -- naquadah balance
5. `onRemoveItem` -- removed item IDs
6. Visual component update (if equipped items changed)

## Data References

- **Item definitions**: 6,060 in `db/resources.sql`
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
