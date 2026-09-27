---
title: "Inventory System"
type: reference
audience: engineers
last_updated: 2026-09-26
---

# Inventory System

> **Last updated**: 2026-09-26
> **Status**: Implemented, including the full vendor stack in code. The vendor stack has **never been tested in a client on working code** (see [Vendor caveat](#vendor-caveat)). Remaining gaps are stat recalculation on equip and the organization vault.

## Overview

The inventory system manages item storage, equipping, movement, and currency for player entities. Items are organized into numbered bags (containers) with fixed slot counts. Each bag may represent general storage, equipment slots, crafting storage, or mission items. Equipped items contribute visual components to the player model and trigger equip/unequip callbacks.

Inventory splits across the two services: cell-side operations live in [`cell/cell_methods/inventory/`](../../crates/cell-methods/src/cell/cell_methods/inventory/) (item ops plus the bandolier/active-slot machinery), and everything that touches the database — including the entire vendor stack — lives in [`base/world_entry/methods/inventory/`](../../crates/base-methods/src/base/world_entry/methods/inventory/) and [`base/world_entry/methods/vendor/`](../../crates/base-methods/src/base/world_entry/methods/vendor/). Item definitions come from `db/resources/Items/`.

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
| Store open/close | DONE (no client test) | `base/world_entry/methods/vendor/store.rs`. `onStoreOpen` / `onStoreUpdate` go out on SGWPlayer indices 109/110 since #609; see [Vendor caveat](#vendor-caveat) |
| Store buy/sell | DONE | `vendor/purchase/`, `vendor/sell/` |
| Buyback | DONE | `vendor/buyback/` |
| Item repair (vendor) | DONE | `vendor/repair.rs` plus the paid-repair variant |
| Item recharge (vendor) | DONE | `vendor/recharge.rs` plus the paid-recharge variant |
| Player-movable allowlist | DONE | `moveItem` checks both the source and the target container against `player_movable` (see [Container capacity and movability](#container-capacity-and-movability)). Buyback (16) is refused both ways (#798) |
| Vendor bag allowlist | DONE | `VENDOR_FILTER_BAGS` confines vendor operations to the main bag, bandolier, the eleven equipment slots, and the crafting bag (15) — the bank, mail attachments, and loot bags are unreachable |
| Item repair (direct) | NOT IMPL | `repairItemRequest` (the client-initiated cell method) decodes its args and logs `UNIMPLEMENTED`; repair only works through the vendor path |
| Stat recalculation on equip | NOT IMPL | `inventoryAdjustments` property exists |
| Organization vault | NOT IMPL | `onClearOrgVaultInventory`, `onOrgMoveItemResult` defined; blocked on the organization system |
| Personal vault window | DONE (open path only) | A Banker click or GM `.bank` opens it and starts a vault session; see [Opening the vault](#opening-the-vault). Moves into and out of the vault are BV-03 |

### Vendor caveat

The vendor rows above (store open/close through vendor bag allowlist) are server-side DONE, not client-verified. Until #609 (2026-07-26) the store payload went out on SGWPlayer indices 80/81, which are the Missionary `onMissionUpdate` / `onStepUpdate` methods, so the store window could never open and earlier manual vendor testing is void. No client has tested a vendor since #609. Coverage is live-DB tests plus the server-side PL/pgSQL smoke [`tools/vendor_store_smoke.sql`](../../tools/vendor_store_smoke.sql).

No world spawns a vendor today. Template 25 ("Interaction Debug NPC - DO NOT USE") is the only vendor template, and Harset packet H13 removed its only spawn row, so `.spawn 25` is the only way to reach a store. Only two test item lists are seeded. Status detail: [gap-analysis.md §15](../gap-analysis.md#15-stores--vendors----nt).

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
| `moveItem` | YES | itemId, targetBag, targetSlot, quantity | Move/swap item. Source and target must both be player-movable; a refused move resends the refused item |
| `useItem` | YES | itemID, targetID | Use item on target |
| `repairItemRequest` | YES | itemId, repairRatio | Repair item (NOT IMPL) |
| `requestActiveSlotChange` | YES | BagId, SlotId | Change equipped slot |
| `requestAmmoChange` | YES | ItemId, AmmoType | Change ammo type |
| `giveCash` | NO | Amount | Server-side cash grant |
| `requestGiveItem` | NO | itemId, quantity, requireFull, callbackEntity, callbackRpc, callbackArgs | Server-side item grant |

## Bag Types (EInventoryContainerId)

| Id | Enum | Purpose |
|----|------|---------|
| 1 | `INV_Main` | General inventory |
| 2 | `INV_Mission` | Mission-specific items |
| 3 | `INV_Bandolier` | Weapon loadout (equipped) |
| 4-14 | `INV_Head` ... `INV_Artifact2` | The eleven equipment slots |
| 15 | `INV_Crafting` | Crafting materials and Field Crafting Tools |
| 16 | `INV_Buyback` | Store buyback. Persisted in `sgw_inventory` with the unit sale price in `flags`; left only through `buybackItems`, which charges |
| 17 | `INV_Bank` | Personal vault |
| 18 | `INV_Auction` | Auction escrow |
| 19 | `INV_TeamBank` | Team (organization) vault |
| 20 | `INV_CommandBank` | Command (organization) vault |

## Container capacity and movability

**Capacity has one source**, `bag_max_slots` in [`crates/entity/src/inventory.rs`](../../crates/entity/src/inventory.rs), with the values of `BAG_SIZES` in `deprecated/python/common/Constants.py`. `BAG_SIZES` in the same file, which `onBagInfo` declares, is built from it at compile time, so the two cannot disagree. `cimmeria_wire::containers::bag_max_slots` and `base::resources::bag_max_slots` re-export it.

| Id | Capacity | `player_movable` |
|----|----------|------------------|
| 1 | 40 | Yes |
| 2 | 100 | Yes |
| 3 | 4 | Yes |
| 4-14 | 1 each | Yes |
| 15 | 100 | Yes |
| 16 | 12 | No |
| 17 | 100 (ceiling); the player's own size is `sgw_player.bank_slots` | VaultSession |
| 18 | 100 | No |
| 19 | 100 | No |
| 20 | 100 | No |
| anything else | 0 | No |

**The personal vault's size is per player.** `sgw_player.bank_slots` is a `smallint`, default 40, constrained to 40-100 in steps of 10. It loads with the player, and `onBagInfo` declares container 17 at that size both at world entry (`map_loaded.rs`) and on the post-respawn resync (`send_full_inventory_resync`). Every other container is declared at its capacity.

**Player moves go through an allowlist.** `handle_move_inventory_item` checks the target container before the slot-range check and the source container after it locks the source row, using `player_movable` in [`move_/container_policy.rs`](../../crates/base-methods/src/base/world_entry/methods/inventory/move_/container_policy.rs). A capacity alone never makes a container movable. `VaultSession` means movable only while a vault session is open; until the vault session exists, it refuses like `No`. A refused move changes nothing, logs `move_rejected` at WARN under the `bank` target with a `reason` (`source_container_not_player_movable`, `target_container_not_player_movable`, `source_container_needs_vault_session` or `target_container_needs_vault_session`), and resends that one item (`onUpdateItem` for the refused `item_id` only, read and sent under the per-player move lock and a row lock on that item, so no concurrent write to it, such as a grant merging into the stack, can be overtaken by the older state) so the client snaps it back. An `item_id` the player does not own gets no packet. If either lock cannot be taken, nothing is resent either (`move_resync_skipped reason=lock_timeout`): the client keeps the dragged position until the next update of that item, rather than risk an unlocked resend overtaking a concurrent write. Whether a given item may sit in a movable container is still decided by its `container_sets` (`item_allows_container`).

**Grants never write into 17-20.** Loot and content grants target an item's first `container_sets` entry, which is 17 for the seeded crafting components (`{17,15}`). `handle_grant_item` refuses those containers and logs `grant_rejected` under `bank` with `reason=grant_into_storage_container`, which is what the grant did before the vaults had a capacity.

## Bandolier and ammo

`INV_Bandolier` (container id `3`) holds 4 weapon slots indexed `0..3` and is the only container that tracks an active slot. Slot count matches legacy `deprecated/python/common/Constants.py:145` (`BAG_SIZES[INV_Bandolier] = 4`); there is no fist-weapon reservation, so all four slots are real weapon slots.

The wire format is **1-indexed** (slots `1..4`). Server-side everything is **0-indexed**; the cell decoder subtracts 1 on inbound `requestActiveSlotChange` / `moveItem` and the grant/sync paths add 1 on outbound `onActiveSlotUpdate`. Mismatch on the inbound side was the original cause of the "switching slots doesn't work" bug — see `crates/cell-combat/src/cell/cell_methods/inventory/bandolier.rs` and `item_ops.rs`.

Each bandolier slot persists not only the equipped item but also its **per-slot magazine state**:

| `sgw_inventory` column | Field | Purpose |
|------------------------|-------|---------|
| `ammo`                 | `BandolierItem.current_ammo` | Rounds remaining in this slot's magazine |
| `cur_ammo_type`        | `BandolierItem.cur_ammo_type` | Selected ammo subtype (defaults to item's `default_ammo_type`) |

Both columns are bandolier-slot-scoped — swapping weapons does not pool ammo across slots. The cell server mirrors `current_ammo` to `Stat[AMMO_SLOT_1+slot]` (stat IDs 49–53) so the client UI can subscribe to `Events.StatUpdated` for meter and count refresh.

Persistence is **batched**: dirty slots are flushed at reload completion, slot swap, ammo change, logout, and world transition. Full message flow, sequence diagrams, and legacy reference points are in [weapon-ammo-reload.md](weapon-ammo-reload.md).

## Opening the vault

The personal vault is container 17 (`INV_Bank`). Its rows load at login with the rest of the inventory, and the world-entry `onBagInfo` declares its size, so opening the window needs neither a base round trip nor a fresh `onBagInfo` (BV-E1, [bank-vault-client.md](../reverse-engineering/findings/bank-vault-client.md) Q1). The client has no open control of its own: it shows the window only when the server sends `onVaultOpen` (client method 106), and it sends nothing when the window closes.

**At a Banker.** A Banker is an NPC whose template carries `INT_Banker` with `vault_scope = 'personal'` ([interaction-flags.md](../content/interaction-flags.md#bankers)). A right-click passes the usual interact gate (same space, within `MAX_INTERACT_DISTANCE` = 5), pins the Banker as the interaction target, and reaches the Banker arm of `handle_interact`. That arm, in [`crates/cell-interactions/src/cell/interactions/bank/`](../../crates/cell-interactions/src/cell/interactions/bank/mod.rs):

1. records a vault session on the player's cell entity: `VaultSession { scope: Personal, banker_id: Some(banker), space_id, opened_at }`;
2. sends `onVaultOpen(banker_id, banker_position)` (`INT32`, then `VECTOR3`) from the cell;
3. logs `vault_open` under the `bank` target.

A click from out of range sends nothing and opens no session. A `team` or `command` Banker is refused with a chat line and logs `vault_open_rejected reason=org_vault_not_available` until the organization vaults land.

**GM `.bank`.** Opens the same window wherever the GM stands, with a session whose `banker_id` is `None`, and `onVaultOpen` addressed to the GM's own entity and position. A player without GM access gets a refusal line ([commands.md](../commands.md)).

**The session ends** when the player changes space or logs out (both destroy the cell entity that holds it), or when a later `interact` pins a different target. Re-clicking the same Banker keeps it. Each end logs `vault_session_cleared` at DEBUG with its `reason`.

**The move rule.** `vault_move_allowed(&player, &space_mgr)` is the single check a bank move must pass: an open session, opened in the player's current space, and, for a Banker session, the Banker still present, in the same space and within the interact distance. A GM session skips the proximity check. The client ignores `onVaultOpen`'s position (BV-E1 Q4), so walking away does not close the window; this check, run on every move, is the only enforcement. Wiring it into `moveItem` is BV-03.

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
- **Bag sizes**: `bag_max_slots` in `crates/entity/src/inventory.rs` (values from `common.Constants.BAG_SIZES`); the personal vault's size is `sgw_player.bank_slots`
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
