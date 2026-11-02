---
title: "Debug Armory client limits"
type: reference
audience: engineers
last_updated: 2026-11-01
---

# Debug Armory client limits (DR-02)

Static evidence from the stock client (`Content/UI/Core` Lua, read-only install copy) and the SGW.exe Ghidra project. No lab or x64dbg run. Addresses are SGW.exe image addresses. "Not found" means the search was done and nothing was there.

## Q1. Store window row limit

- The client does not cap buy rows. `VendorMod.resetScrollBar` sets `Vendor_Scrollbar` `DocumentSize` to the full `getVendorItemCount()` (`Vendor/Vendor.lua:482-489`).
- The count is native `FUN_00ad7da0`, which returns `(end - begin) >> 2` of a vector (`FUN_00e0aab0`). No cap was found in either.
- `Vendor.layout` has 7 slot widgets (`Vendor_Item1`..`Vendor_Item7`). Rows past the visible slots are reached by scrolling the bar. The scrollbar `PageSize` is 6 (`Vendor/Vendor.layout:87`) and the scroll clamp uses 6 (`Vendor/Vendor.lua:538-539`), so a page is six rows; `Vendor_Item7` is shown once the count reaches 7 (`Vendor.lua:490-494`).
- Store open is subscribed at `Vendor/Vendor.lua:1293` (`Events.VendorOpen` to `VendorMod.onStartVending`).
- Not checked: the wire array decoder for `onStoreOpen` (`Event_NetIn_onStoreOpen`, near `00d7ed40`), so a count limit on the wire is open.
- Answer: no client cap found; the window scrolls. Six rows are visible at a time.

## Q2. Client-side equip checks

- The Lua does not gate equip. A right-click on a backpack slot calls `InventoryMod.onSlotItemMouseButtonUp` (`Inventory/Inventory.lua:745-757`), which calls native `contextSensitiveUseItem(containerID, slotID)`. Drag and drop calls `moveItem(...)` (`Inventory/Inventory.lua:704`). Neither path checks level, archetype, faction or discipline in Lua.
- Native `contextSensitiveUseItem` is `0xaa73e0` (name string `0x195344c`, registration at `00ad576e`). It reads the two slot arguments, calls `FUN_00ad8c30`, which calls `FUN_00e1f570` on the inventory manager. That calls `FUN_00e1c4c0` (slot lookup, returns the item) and then `FUN_00e1f2e0` (the use or move branch).
- `FUN_00e1f2e0` calls a per-item predicate `FUN_00d21110` and a slot finder `FUN_00e1c6a0`. The predicate bodies were not decoded.
- Not found: any level, archetype, faction or discipline comparison in the Lua or in the decompiled path above. The predicate `FUN_00d21110` is still unread, so a native gate is not ruled out.

## Q3. Equip call for armor and the unequip of the old piece

- No Lua calls an equip function directly. The equip path is the native `contextSensitiveUseItem` (Q2). A spec can therefore use the same action for armor and weapons: a backpack slot right-click, which the client sends as the native call. This is what `@item_action { action = "equip", name }` already does: `crates/lab/src/supervisor/ui/item_action.rs:4-12` documents that `equip`, `use`, `unequip` and `rightclick` are all one right-click on the slot (`contextSensitiveUseItem`), with the `/equip <item_id>` slash command as the fallback (`item_action.rs:65`). No spec change is needed for armor.
- Whether the client unequips the old piece itself: not decoded. The branch in `FUN_00e1f2e0` that calls `FUN_00e1ef70(item, param_1[5])` looks like the move send, but this was not confirmed. The server's move handler performs the swap.

## Q4. Ability press and archetype

- Action bar press: `ActionButtonMod.onActionPress` calls `useAction(actionId, false)` (`ActionButtons/ActionButtons.lua:158-166`). Native `useAction` is `0xaa94e0`, which calls `FUN_00ad9580`.
- Ability window press: `AbilityMod.onAbilityPress` calls `useAbility(id, Unit.Target)` only when `haveIt` is set, and calls `buyTrainable` otherwise (`Ability/Ability.lua:177-185`). Native `useAbility` is `0xaa2910`, which calls `FUN_00ad78e0`, then `FUN_00d2afc0` and `FUN_00d2ae40`.
- `FUN_00d2ae40` returns early when a state field equals 3, and otherwise sends a cell call with `AbilityID` and `TargetID` (`0x00cacd50`).
- Not found: any archetype check in the Lua, or in the press path `useAbility` to `FUN_00d2ae40`. The client does not refuse a press by archetype on the paths read. No archetype check was found in the client; the only per-ability client gate found is the AbilitySet membership in the next bullet.
- Known drop: `FUN_00d2afc0` (`JZ` at `0x00d2afcf`) discards a press when the ability id is not in the client's `AbilitySet`. No log, no error, no wire message. The set is filled only by server messages (`onKnownAbilitiesUpdate`, `onAbilityTreeInfo`, and similar). So an Armory ability the server did not teach the client is dead on press. Source: [ability-client-hook-anchors.md, AB-C2](../../../reverse-engineering/findings/ability-client-hook-anchors.md), row 5. That page lists the other drop rows (1 to 10); none is an archetype check.

## For DR-03a

Row cap: 19 (default). No client cap was found, so the cap is a generator choice, not a client limit.
