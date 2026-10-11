# Inventory System Wire Formats

> **Date**: 2026-03-01
> **Confidence**: HIGH (3+ sources agree on all formats)
> **Sources**: Ghidra decompilation of `sgw.exe`, `.def` files, `alias.xml`, Python client scripts

---

## Overview

Inventory messages follow the same universal RPC dispatcher architecture documented in [combat-wire-formats.md](combat-wire-formats.md). All wire formats are derived from the `.def` method signatures and `alias.xml` type definitions.

The inventory system is defined in `entities/defs/interfaces/SGWInventoryManager.def`, which SGWPlayer implements via its `<Implements>` list.

---

## Custom Types (from alias.xml)

### InvItem

```xml
<InvItem>FIXED_DICT
    <Properties>
        <id>          <Type> INT32 </Type></id>
        <dbid>        <Type> INT32 </Type></dbid>
        <stackSize>   <Type> INT32 </Type></stackSize>
        <slotID>      <Type> INT32 </Type></slotID>
        <containerID> <Type> INT32 </Type></containerID>
        <isBound>     <Type> UINT8 </Type></isBound>
        <durability>  <Type> INT32 </Type></durability>
        <ammoTypes>   <Type> ARRAY<of>INT32</of> </Type></ammoTypes>
        <curAmmoType> <Type> INT32 </Type></curAmmoType>
        <charges>     <Type> INT32 </Type></charges>
    </Properties>
</InvItem>
```

**Wire layout** (serialized in declaration order):

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| +0 | 4 | int32 | id | Item instance ID |
| +4 | 4 | int32 | dbid | Item definition ID (database) |
| +8 | 4 | int32 | stackSize | Current stack count |
| +12 | 4 | int32 | slotID | Slot position within container |
| +16 | 4 | int32 | containerID | Bag/container this item is in |
| +20 | 1 | uint8 | isBound | 0 = tradeable, 1 = bound to player |
| +21 | 4 | int32 | durability | Current durability |
| +25 | 4 | uint32 | ammoCount | Number of ammo types (ARRAY prefix) |
| +29 | 4*M | int32[] | ammoTypes | Available ammo type IDs |
| +var | 4 | int32 | curAmmoType | Currently loaded ammo type |
| +var | 4 | int32 | charges | Remaining charges |

**Size**: 37 + 4*M bytes (M = number of ammo types, typically 0-5)

### BagInfo

```xml
<BagInfo>FIXED_DICT
    <Properties>
        <BagId>         <Type> INT32 </Type></BagId>
        <NumberOfSlots> <Type> INT32 </Type></NumberOfSlots>
    </Properties>
</BagInfo>
```

**Wire layout**: 8 bytes fixed (int32 + int32)

### ItemID

```xml
<ItemID> INT32 </ItemID>
```

Alias for INT32 — 4 bytes.

---

## Inventory Messages: Client → Server

### moveItem

**Source**: `SGWInventoryManager.def` CellMethods (Exposed)

```xml
<moveItem>
    <Exposed/>
    <Arg> INT32 <ArgName>aItemId</ArgName></Arg>
    <Arg> INT32 <ArgName>aTargetBag</ArgName></Arg>
    <Arg> INT32 <ArgName>aTargetSlot</ArgName></Arg>
    <Arg> INT32 <ArgName>aQuantity</ArgName></Arg>
</moveItem>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 1 | uint8 | Header | `methodID \| 0x80` |
| 1 | 4 | int32 | aItemId | Instance ID of item to move |
| 5 | 4 | int32 | aTargetBag | Destination bag/container ID |
| 9 | 4 | int32 | aTargetSlot | Destination slot index |
| 13 | 4 | int32 | aQuantity | Number of items to move (for stacks) |

**Total**: 17 bytes

---

### useItem

**Source**: `SGWInventoryManager.def` CellMethods (Exposed)

```xml
<useItem>
    <Exposed/>
    <Arg> INT32 <ArgName>aItemID</ArgName></Arg>
    <Arg> INT32 <ArgName>aTargetID</ArgName></Arg>
</useItem>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 1 | uint8 | Header | `methodID \| 0x80` |
| 1 | 4 | int32 | aItemID | Instance ID of item to use |
| 5 | 4 | int32 | aTargetID | Target entity ID (0 for self-use) |

**Total**: 9 bytes

---

### removeItem

**Source**: `SGWInventoryManager.def` CellMethods (Exposed)

```xml
<removeItem>
    <Exposed/>
    <Arg> ItemID <ArgName>itemID</ArgName></Arg>
    <Arg> INT16  <ArgName>quantity</ArgName></Arg>
</removeItem>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 1 | uint8 | Header | `methodID \| 0x80` |
| 1 | 4 | int32 | itemID | Instance ID of item to destroy |
| 5 | 2 | int16 | quantity | Number to remove from stack |

**Total**: 7 bytes

---

### listItems

**Source**: `SGWInventoryManager.def` CellMethods (Exposed)

```xml
<listItems>
    <Exposed/>
</listItems>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 1 | uint8 | Header | `methodID \| 0x80` |

**Total**: 1 byte (no args — server responds with full inventory dump)

---

### repairItemRequest

**Source**: `SGWInventoryManager.def` CellMethods (Exposed)

```xml
<repairItemRequest>
    <Exposed/>
    <Arg> INT32 <ArgName>itemId</ArgName></Arg>
    <Arg> FLOAT <ArgName>repairRatio</ArgName></Arg>
</repairItemRequest>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 1 | uint8 | Header | `methodID \| 0x80` |
| 1 | 4 | int32 | itemId | Instance ID of item to repair |
| 5 | 4 | float32 | repairRatio | Ratio of durability to restore (0.0-1.0) |

**Total**: 9 bytes

---

### requestActiveSlotChange

**Source**: `SGWInventoryManager.def` CellMethods (Exposed)

```xml
<requestActiveSlotChange>
    <Exposed/>
    <Arg> INT32 <ArgName>BagId</ArgName></Arg>
    <Arg> INT32 <ArgName>SlotId</ArgName></Arg>
</requestActiveSlotChange>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 1 | uint8 | Header | `methodID \| 0x80` |
| 1 | 4 | int32 | BagId | Equipment bag ID |
| 5 | 4 | int32 | SlotId | Slot to activate |

**Total**: 9 bytes

---

### requestAmmoChange

**Source**: `SGWInventoryManager.def` CellMethods (Exposed)

```xml
<requestAmmoChange>
    <Exposed/>
    <Arg> INT32 <ArgName>ItemId</ArgName></Arg>
    <Arg> INT32 <ArgName>AmmoType</ArgName></Arg>
</requestAmmoChange>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 1 | uint8 | Header | `methodID \| 0x80` |
| 1 | 4 | int32 | ItemId | Weapon instance ID |
| 5 | 4 | int32 | AmmoType | Ammo type to switch to |

**Total**: 9 bytes

`ItemId` is the weapon's **instance** id (`InvItem.id`, `sgw_inventory.item_id`), not its design id: the sender `FUN_00e1ee10` writes `item+0x0C`, which the item constructor `FUN_00d21750` fills from `id` (issue #534's decompile). The server matches the bandolier slot on it and reads the ammo whitelist by that slot's design id (ammo campaign AM-03, PR #1051).

---

### Store Operations

**Source**: `SGWPlayer.def` CellMethods (Exposed)

```xml
<purchaseItems>
    <Exposed/>
    <Arg> ARRAY<of>INT32</of> <ArgName>ItemIndices</ArgName></Arg>
    <Arg> ARRAY<of>INT32</of> <ArgName>Quantities</ArgName></Arg>
</purchaseItems>

<sellItems>
    <Exposed/>
    <Arg> ARRAY<of>INT32</of> <ArgName>ItemIds</ArgName></Arg>
    <Arg> ARRAY<of>INT32</of> <ArgName>Quantities</ArgName></Arg>
</sellItems>

<buybackItems>
    <Exposed/>
    <Arg> ARRAY<of>INT32</of> <ArgName>ItemIds</ArgName></Arg>
    <Arg> ARRAY<of>INT32</of> <ArgName>Quantities</ArgName></Arg>
</buybackItems>

<repairItems>
    <Exposed/>
    <Arg> ARRAY<of>INT32</of> <ArgName>ItemIds</ArgName></Arg>
</repairItems>

<rechargeItems>
    <Exposed/>
    <Arg> ARRAY<of>INT32</of> <ArgName>ItemIds</ArgName></Arg>
</rechargeItems>
```

**Verified against the client (2026-10-10, static RE of `SGW.exe`; no live capture).** The stock `SGWPlayer.def` shipped in the client (`Common/res/entities/defs/SGWPlayer.def`) is byte-identical to `entities/defs/SGWPlayer.def`. Each method is `<Exposed/>` with only the arguments shown above. An `ARRAY` is a `u32` count followed by the elements (see the dispatch-table notes and the client array decoder `FUN_015a2e60`), so every element is a bare `INT32`.

Cell-method args (after the method-id header byte `methodID | 0x80`), all little-endian:

| Method | Index | Args on the wire |
|--------|-------|------------------|
| `purchaseItems` | 78 | `u32 n`, `n x i32 ItemIndices`, `u32 m`, `m x i32 Quantities` |
| `sellItems` | 79 | `u32 n`, `n x i32 ItemIds`, `u32 m`, `m x i32 Quantities` |
| `buybackItems` | 80 | `u32 n`, `n x i32 ItemIds`, `u32 m`, `m x i32 Quantities` |
| `repairItems` | 81 | `u32 n`, `n x i32 ItemIds` |
| `rechargeItems` | 82 | `u32 n`, `n x i32 ItemIds` |

- **Two parallel arrays, not `(id, qty)` pairs.** Element `i` of the second array is the quantity for element `i` of the first. The client always builds both arrays with the same length.
- **No vendor template id is sent by any of the five methods.** There is no trailing `INT32`, in the `.def`, in the Lua call sites or in the native senders. The vendor is known server-side from the player's open vendor session. A server that requires a trailing template id (or reads a bare `(id, qty)` pair stream) will mis-parse every real request: the first array's `u32` count is read as the pair count, and the bytes after it are interpreted as pairs.
- `ItemIndices` (purchase) is the vendor-list row index the Lua UI passes (`vendorBuyIndividual( index, -1 )`, 0-based `scrollPos + i - 1`). `ItemIds` (sell, buyback, repair, recharge) is the inventory item instance id (`getVendorItemInfo( index, VendorItem.VendorItem ).ItemId`).
- **Quantity `-1` is resolved client-side before sending.** For the individual-click path the Lua passes `-1`; the native sender replaces it with the item's own stack count from the container cache (`FUN_00e0e930` reads `*(item + 0x10)` through `thunk_FUN_00e1c450(*(SGWPlayer+0x8c -> +0x24), itemId)`). The server never sees `-1` from this path.
- The "selected" (multi-select) path builds the two arrays from every selected row whose selected quantity is `> 0`: row `ItemIds` from `*(*(row + 8) + 0xc)`, quantity from `*(row + 0xc)`; the vendor window's quantities are zeroed on completion.

**Evidence trail** (`SGW.exe`, headless Ghidra `Probe.java`, static):

| Step | Address | Evidence |
|------|---------|----------|
| Lua call sites | `Content/UI/Core/Vendor/Vendor.lua:741-772, 850-866` | `vendorBuySelected()`, `vendorSellSelected()`, `vendorBuybackSelected()`, `vendorRepairSelected()`, `vendorRechargeSelected()`; right-click: `vendorBuyIndividual( index, -1 )`, `vendorSellIndividual( ItemId, -1 )`, `vendorBuybackIndividual( ItemId, -1 )`, `vendorRepairIndividual( ItemId )`, `vendorRechargeIndividual( ItemId )` |
| Lua binding table | `0x00ad4f20-0x00ad5001` (inside the CEGUI Lua registration, `FUN_00403ec0(L, name, fn)`) | Individual: sell `0xaa3c30`, buyback `0xaa3d30`, repair `0xaa3e30`, recharge `0xaa3f00`; selected: buy `0xaa3b40`, sell `0xaa3ce0`, buyback `0xaa3de0`, repair `0xaa3eb0`, recharge `0xaa3f80`. Arg checks: `(index, qty)` for buy/sell/buyback individual, `(itemId)` for repair/recharge individual |
| Thunks to the vendor manager | `0x00ad7ea0-0x00ad7fe0` | Each does `FUN_00c66ad0()+0x8c -> [ptr] -> +0x28` (vendor object) and jumps to the sender |
| Senders, individual | sell `0x00e0e930`, buyback `0x00e0ee60`, repair `0x00e0f390`, recharge `0x00e0f6e0` | Build one CME event, put `"ItemIds"` (and for sell/buyback `"Quantities"`) property lists, emit it |
| Senders, selected | buy `0x00e0e640`, sell `0x00e0eb70`, buyback `0x00e0f0a0`, repair `0x00e0f4f0`, recharge `0x00e0f840` | Walk the selected rows and fill the same property lists. Buy uses `"ItemIndices"` and `"Quantities"` |
| Event classes | `Event_NetOut_PurchaseItems` ctor `0x00d97c30`, `SellItems` `0x00d97ed0`, `BuybackItems` `0x00d98180`, `RepairItems` `0x00d98420`, `RechargeItems` `0x00d986c0` | One event per method. No other property names appear in any sender |
| Event to method mapping | `register_NetOut_onStrikeTeamResponse@00db3390` (mislabelled; it is a bulk NetOut registration), sites `0x00dc279f-0x00dc29d5`; handler objects ctor `0x00d5a0f0, 0x00d5a230, 0x00d5a370, 0x00d5a4b0, 0x00d5a5f0` | Maps each NetOut event to its cell method name; the args are the event properties named as the `.def` `ArgName`s |

The event property names (`ItemIndices`, `ItemIds`, `Quantities`) equal the `.def` `ArgName`s exactly and the lists are single `INT32` lists, which is what makes the two-array layout above the only one the stock client can produce. Note `0x00d470e0`, sometimes cited as the emit site, is the `Event_NetOut_RepairItems` `MemberCallback` RTTI accessor (`annotation-script-shift-bugs.md`), not a sender.

**Evidence level.** HIGH for the layout: stock `.def` plus a complete static trace from the Lua call through the native sender to the typed events, with matching property names. Not confirmed: a byte capture of a real client request (the final event-to-Mercury marshalling in `SGWNetworkManager` was inferred from the `.def`/property-name match, not traced byte by byte), and the pair-stream-plus-template-id layout in `crates/cell-methods/.../vendor/wire.rs` has no support anywhere in the binary.
---

### lootItem

**Source**: `SGWPlayer.def` CellMethods (Exposed)

```xml
<lootItem>
    <Exposed/>
    <Arg> INT32 <ArgName>Index</ArgName></Arg>
</lootItem>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 1 | uint8 | Header | `methodID \| 0x80` |
| 1 | 4 | int32 | Index | Loot window item index |

**Total**: 5 bytes

---

## Inventory Messages: Server → Client

### onBagInfo

**Source**: `SGWInventoryManager.def` ClientMethods

```xml
<onBagInfo>
    <Arg> ARRAY<of>BagInfo</of> <ArgName>BagInfo</ArgName></Arg>
</onBagInfo>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 4 | uint32 | Count | Number of bags |
| 4 | 8*N | BagInfo[] | BagInfo | Bag definitions |

**Per BagInfo entry** (8 bytes):

| Offset | Size | Type | Field |
|--------|------|------|-------|
| +0 | 4 | int32 | BagId |
| +4 | 4 | int32 | NumberOfSlots |

**Total**: 4 + 8*N bytes

**Python validation**: `python/client/Inventory.py` `onBagInfo(bagInfoList)` — matches. Used on login to tell the client about all available bags (backpack, equipment, ammo belt, etc.).

---

### onUpdateItem

**Source**: `SGWInventoryManager.def` ClientMethods

```xml
<onUpdateItem>
    <Arg> ARRAY<of>InvItem</of> <ArgName>ItemUpdates</ArgName></Arg>
</onUpdateItem>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 4 | uint32 | Count | Number of item updates |
| 4 | var | InvItem[] | ItemUpdates | Full item data per update |

Each `InvItem` is 37 + 4*M bytes (see InvItem layout above). This message can batch multiple item changes in a single call.

**Python validation**: `python/client/Inventory.py` `onUpdateItem(items)` — matches. Creates or updates items in the client's inventory model.

---

### onRemoveItem

**Source**: `SGWInventoryManager.def` ClientMethods

```xml
<onRemoveItem>
    <Arg> ARRAY<of>INT32</of> <ArgName>ItemIdList</ArgName></Arg>
</onRemoveItem>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 4 | uint32 | Count | Number of items to remove |
| 4 | 4*N | int32[] | ItemIdList | Instance IDs of removed items |

**Total**: 4 + 4*N bytes

**Python validation**: `python/client/Inventory.py` `onRemoveItem(itemIdList)` — matches. Removes items from client-side inventory.

---

### onActiveSlotUpdate

**Source**: `SGWInventoryManager.def` ClientMethods

```xml
<onActiveSlotUpdate>
    <Arg> INT32 <ArgName>BagId</ArgName></Arg>
    <Arg> INT32 <ArgName>SlotId</ArgName></Arg>
</onActiveSlotUpdate>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 4 | int32 | BagId | Equipment bag ID |
| 4 | 4 | int32 | SlotId | Newly active slot index |

**Total**: 8 bytes

---

### onRefreshItem

**Source**: `SGWInventoryManager.def` ClientMethods

```xml
<onRefreshItem>
    <Arg> INT32 <ArgName>ItemId</ArgName></Arg>
</onRefreshItem>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 4 | int32 | ItemId | Instance ID to refresh from cache |

**Total**: 4 bytes

---

### onCashChanged

**Source**: `SGWInventoryManager.def` ClientMethods

```xml
<onCashChanged>
    <Arg> INT32 <ArgName>cash</ArgName></Arg>
</onCashChanged>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 4 | int32 | cash | New total cash (naquadah) amount |

**Total**: 4 bytes

---

### onClearOrgVaultInventory

**Source**: `SGWInventoryManager.def` ClientMethods

```xml
<onClearOrgVaultInventory>
    <Arg> INT32 <ArgName>OrganizationId</ArgName></Arg>
</onClearOrgVaultInventory>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 4 | int32 | OrganizationId | Organization whose vault to clear |

**Total**: 4 bytes

---

### Store Display Messages

**Source**: `SGWPlayer.def` ClientMethods

```xml
<onStoreOpen>
    <Arg> INT32                         <ArgName>EntityId</ArgName></Arg>
    <Arg> INT32                         <ArgName>VendorType</ArgName></Arg>
    <Arg> ARRAY<of>StoreItem</of>       <ArgName>Items</ArgName></Arg>
    <Arg> ARRAY<of>ItemCost</of>        <ArgName>SellPrices</ArgName></Arg>
    <Arg> ARRAY<of>ItemCost</of>        <ArgName>BuybackPrices</ArgName></Arg>
    <Arg> ARRAY<of>ItemCost</of>        <ArgName>RepairPrices</ArgName></Arg>
    <Arg> ARRAY<of>ItemCost</of>        <ArgName>RechargePrices</ArgName></Arg>
</onStoreOpen>
```

**`StoreItem`** = `FIXED_DICT` (from `alias.xml`):

| Field | Type | Size |
|-------|------|------|
| index | INT32 | 4 |
| itemId | INT32 | 4 |
| costList | ARRAY<of>BuyCost</of> | 4 + 12*K |
| usable | UINT8 | 1 |
| quantity | INT32 | 4 |

**`BuyCost`** = `FIXED_DICT`: type(INT32) + typeId(INT32) + quantity(INT32) = 12 bytes

**`ItemCost`** = `FIXED_DICT`: cost(INT32) + itemId(INT32) = 8 bytes

**Wire format**: Complex variable-length message. Total = 8 + store items array + 4 sell/buyback/repair/recharge price arrays.

---

### Loot Display

**Source**: `SGWPlayer.def` ClientMethods

```xml
<onLootDisplay>
    <Arg> INT32                  <ArgName>EntityID</ArgName></Arg>
    <Arg> LootItemQuantityList   <ArgName>ItemList</ArgName></Arg>
    <Arg> INT8                   <ArgName>Initial</ArgName></Arg>
</onLootDisplay>
```

**`LootItemQuantityList`** = `ARRAY<of>LootItemQuantity</of>`

**`LootItemQuantity`** = `FIXED_DICT`: itemID(INT32) + quantity(INT16) + index(INT32) + typeID(INT32) = 14 bytes

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 4 | int32 | EntityID | Mob entity ID being looted |
| 4 | 4 | uint32 | ItemCount | Number of loot items |
| 8 | 14*N | LootItemQuantity[] | ItemList | Loot entries |
| var | 1 | int8 | Initial | 1 = first open, 0 = update |

---

## Cross-Validation Summary

| Message | .def | alias.xml | Python Client | Confidence |
|---------|------|-----------|---------------|------------|
| moveItem | Y | — | Y | HIGH |
| useItem | Y | — | Y | HIGH |
| removeItem | Y | Y (ItemID) | Y | HIGH |
| listItems | Y | — | Y | HIGH |
| repairItemRequest | Y | — | Y | HIGH |
| requestActiveSlotChange | Y | — | Y | HIGH |
| requestAmmoChange | Y | — | Y | HIGH |
| purchaseItems | Y | — | Y | HIGH |
| sellItems | Y | — | Y | HIGH |
| onBagInfo | Y | Y (BagInfo) | Y | HIGH |
| onUpdateItem | Y | Y (InvItem) | Y | HIGH |
| onRemoveItem | Y | — | Y | HIGH |
| onActiveSlotUpdate | Y | — | Y | HIGH |
| onCashChanged | Y | — | Y | HIGH |
| onStoreOpen | Y | Y (StoreItem, ItemCost, BuyCost) | Y | HIGH |
| onLootDisplay | Y | Y (LootItemQuantity) | Y | HIGH |

## Implementation Impact

1. **Inventory initialization flow**: On login, server sends `onBagInfo([...])` to define bag structure, then `onUpdateItem([...])` with all items, then `onCashChanged(amount)`.

2. **Item moves are validated server-side**: Client sends `moveItem(itemId, bag, slot, qty)` — server must validate the move is legal (item exists, slot is empty or stackable, bag has capacity) before sending `onUpdateItem` confirmations.

3. **InvItem.ammoTypes is variable-length**: The ARRAY within the FIXED_DICT means `InvItem` is not actually fixed-size. Most items will have 0 ammo types (non-weapons), but weapons can have 1-5.

4. **Batch operations**: Both `onUpdateItem` and `onRemoveItem` accept arrays, allowing the server to batch multiple inventory changes into a single message for efficiency.

## Related Documents

- [Combat Wire Formats](combat-wire-formats.md) — Combat system messages
- [Entity Property Sync Protocol](../../protocol/entity-property-sync.md) — Property sync
- [Inventory System](../../gameplay/inventory-system.md) — Gameplay mechanics
