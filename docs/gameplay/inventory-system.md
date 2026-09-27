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
| Personal vault window | DONE | A Banker click or GM `.bank` opens it and starts a vault session; see [Opening the vault](#opening-the-vault). Deposits and withdrawals: [Moving items in and out of the vault](#moving-items-in-and-out-of-the-vault) |
| Vault expansion | PARTIAL (server done; client dialog quarantined) | +10 slots per purchase at a Banker, 40 to 100, priced by `resources.bank_expansion_price`. The Expand dialog override is not served until the #943 crash is explained; see [Expanding the vault](#expanding-the-vault) |

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

**The personal vault's size is per player.** `sgw_player.bank_slots` is a `smallint`, default 40, constrained to 40-100 in steps of 10. It loads with the player, and `onBagInfo` declares container 17 at that size at world entry (`map_loaded.rs`), on the post-respawn resync (`send_full_inventory_resync`), and after each expansion bought at a Banker (see [Expanding the vault](#expanding-the-vault)). Every other container is declared at its capacity. `bank_slots` only ever grows: the bank moves read it without a lock, which is safe only while no writer can shrink it.

**Player moves go through an allowlist.** `handle_move_inventory_item` checks the target container before the slot-range check and the source container after it locks the source row, using `player_movable` in [`move_/container_policy.rs`](../../crates/base-methods/src/base/world_entry/methods/inventory/move_/container_policy.rs). A capacity alone never makes a container movable. `VaultSession` means movable only with the cell's vault verdict open (see [Moving items in and out of the vault](#moving-items-in-and-out-of-the-vault)). A refused move changes nothing, logs `move_rejected` at WARN under the `bank` target with a `reason` (`source_container_not_player_movable` or `target_container_not_player_movable` for the allowlist; the vault reasons are listed below), and resends that one item (`onUpdateItem` for the refused `item_id` only, read and sent under the per-player move lock and a row lock on that item, so no concurrent write to it, such as a grant merging into the stack, can be overtaken by the older state) so the client snaps it back. An `item_id` the player does not own gets no packet. If either lock cannot be taken, nothing is resent either (`move_resync_skipped reason=lock_timeout`): the client keeps the dragged position until the next update of that item, rather than risk an unlocked resend overtaking a concurrent write. Whether a given item may sit in a movable container is still decided by its `container_sets` (`item_allows_container`).

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

The personal vault is container 17 (`INV_Bank`). Its rows load at login with the rest of the inventory, and the world-entry `onBagInfo` declares its size (the player's `sgw_player.bank_slots`, see [Container capacity and movability](#container-capacity-and-movability)), so opening the window needs neither a base round trip nor a fresh `onBagInfo` (BV-E1, [bank-vault-client.md](../reverse-engineering/findings/bank-vault-client.md) Q1). The client has no open control of its own: it shows the window only when the server sends `onVaultOpen` (client method 106), and it sends nothing when the window closes.

**At a Banker.** A Banker is an NPC whose template carries `INT_Banker` with `vault_scope = 'personal'` ([interaction-flags.md](../content/interaction-flags.md#bankers)). A right-click passes the usual interact gate (same space, within `MAX_INTERACT_DISTANCE` = 5), pins the Banker as the interaction target, and reaches the Banker arm of `handle_interact`. That arm, in [`crates/cell-interactions/src/cell/interactions/bank/`](../../crates/cell-interactions/src/cell/interactions/bank/mod.rs):

1. records a vault session on the player's cell entity: `VaultSession { scope: Personal, banker_id: Some(banker), space_id, opened_at }`;
2. sends `onVaultOpen(banker_id, banker_position)` (`INT32`, then `VECTOR3`) from the cell;
3. logs `vault_session_opened` (DEBUG) under the `bank` target, inside the INFO span `bank.banker_interact`.

Every refusal sends the player a chat line and logs `vault_open_rejected` (WARN) with a stable `reason`: `out_of_range` (a click on a Banker from beyond the interact distance or from another space; it opens no session), `org_vault_not_available` (a `team` or `command` Banker, until the organization vaults land), `not_gm` (`.bank` from a player), or `banker_missing` (the Banker vanished between the range gate and the arm).

**GM `.bank`.** Opens the same window wherever the GM stands, with a session whose `banker_id` is `None`, and `onVaultOpen` addressed to the GM's own entity and position. A player without GM access gets a refusal line ([commands.md](../commands.md)).

**The session ends** when the player changes space or logs out (both destroy the cell entity that holds it), or when a later `interact` pins a different target. Re-clicking the same Banker keeps it. Each end logs `vault_session_closed` at DEBUG with `reason` `space_change`, `logout` or `re_pin`, and `open_ms`.

**The move rule.** `vault_move_allowed(&player, &space_mgr)` is the single check a bank move must pass: an open session, opened in the player's current space, and, for a Banker session, the Banker still present, in the same space and within the interact distance. A GM session skips the proximity check. The client ignores `onVaultOpen`'s position (BV-E1 Q4), so walking away does not close the window; this check, run on every move, is the only enforcement. It lives in [`cimmeria-cell-world`](../../crates/cell-world/src/cell/space_manager/vault_access.rs) (re-exported beside the Banker), with the interact range rule it uses.

## Moving items in and out of the vault

The vault session lives on the cell's player entity, and the inventory transaction runs on the base. So the cell takes a **verdict** for every inventory request it forwards (`moveItem`, `useItem`, `removeItem`, content `RemoveItem`, `gmRemoveItem`): `vault_access(entity_id, space_mgr)` runs `vault_move_allowed` at that moment and attaches the result, `VaultAccess` ([`crates/wire/src/cell/vault.rs`](../../crates/wire/src/cell/vault.rs)), to the `CellToBaseMsg`. It is `Open { scope, banker_id, distance }` or `Closed { reason, banker_id, distance }`. The base consults it only when the request touches container 17, and only a `Personal` scope opens 17. The verdict is as fresh as the request: a player who walks away is refused on the next drag, and one who walks back is allowed again. In-process moves that never reach the vault (right-click auto-equip between 1 and 3) carry `VaultAccess::NO_SESSION`.

**A move into or out of 17** ([`move_/`](../../crates/base-methods/src/base/world_entry/methods/inventory/move_/mod.rs)) needs, in order:

1. the verdict open at both ends that touch 17 (checked before the transaction for the target, after the `FOR UPDATE` source read for the source);
2. a target slot below the player's `sgw_player.bank_slots`, read inside the move transaction, not the ceiling of 100;
3. for a deposit, an item that is not a mission item (D-BV08): it sits in the mission bag (2), or its type lists 2 in `container_sets` (`EItemFlag` has no mission bit). A swap out of the vault puts the occupant into 17, so the occupant must pass this too. Bound items may be banked;
4. `item_allows_container`, as for every move.

Deposit, withdraw, a move within the vault, a split onto an empty slot, a swap and a merge all go through the one path. **A merge** happens when the whole or partial stack is dropped on a stack of the same type with room for it (`max_stack_size`) and the same `bound`, `durability` and `charges`: the legacy `Inventory.py:391-395` rule, which the Rust port had lost for every container. A whole merge deletes the source row and sends `onRemoveItem` for it. A bound stack never merges into an unbound one; the move swaps instead.

A refused vault move logs `move_rejected` (WARN, `bank`) with a stable `reason` and `vault_end` (`source` or `target`), sends a chat line, then resends the dragged item so it snaps back:

| `reason` | When | Line |
|---|---|---|
| `no_vault_session` | no window open | "Your vault is closed. Visit a Banker to use your vault." |
| `banker_out_of_range`, `banker_other_space`, `vault_session_other_space` | walked away, or changed space | "You are too far from the Banker. Return to the Banker to use your vault." |
| `banker_gone` | the pinned Banker despawned | "The Banker has left. Visit a Banker to use your vault." |
| `player_missing` | no cell entity (a race with logout) | "Your vault is closed. ..." |
| `vault_scope_mismatch` | an org-vault session (none can open yet) | "Your vault is closed. ..." |
| `target_slot_beyond_bank_slots` | a slot at or past `bank_slots` (`bank_slots` logged) | "That vault slot is locked. Your vault has N slots." |
| `mission_item_not_bankable` | a mission item bound for 17 | "Mission items cannot be stored in the vault." |
| `item_not_allowed_in_container` | `container_sets` refuses the item, or a swap occupant | "That item cannot be placed there." |
| `split_onto_occupied_slot` | a split onto a slot that cannot merge | "Split a stack onto an empty slot." |

Every committed vault move logs `move_accepted` (DEBUG, `bank`) with `kind` (`deposit`, `withdraw`, `within`, `split`, `merge`, `swap`), both ends, the stacks before and after at each end, `bank_slots`, and the Banker (`banker_id`, `distance`) or `gm_override=true`. The bank branch of `moveItem` runs in the INFO span `bank.move_item`.

**Use and removal** find an item by id (or by type), and used to find it in any container, so an item in buyback could be used and a banked item used from anywhere. `player_accessible(container_id, &vault)` beside the move allowlist now decides: 1-15 always, 17 only with the verdict open, never 16 or 18-20. `useItem`, `removeItem`, content `RemoveItem` by instance and `gmRemoveItem` refuse an item elsewhere with `use_rejected` (WARN, `bank`, `reason=container_not_accessible`, `container`, `op`) and a chat line. Content `RemoveItem` by type (a turn-in) searches only 1-15, even with the vault open: the bank is storage only (D-BV04), so what a chain consumes must not depend on whether a window is open.

**Slot reservation.** `reserve_free_inventory_slots` bounds 17 by `bank_slots` too, so no path that reserves vault slots can hand a 40-slot player slot 40. Grants into 17 are refused before they get there.

## Expanding the vault

The personal vault starts at 40 slots and grows to 100 in steps of 10, each step bought at a Banker (decision D-BV02). The price of each step is a row in `resources.bank_expansion_price` (`to_slots`, `price_naquadah`), seeded at 100 naquadah for every step from 50 to 100, so it can be tuned in the seed without code. A missing row makes that step unbuyable, never free.

**The offer.** Every personal vault open (a Banker click or GM `.bank`) also asks the base for a quote (`BankCellToBase::ExpansionQuote`). The base reads `bank_slots`, the cash and the next step's price:

- below 100 slots, it answers the cell with `BankBaseToCell::OfferExpansion { from_slots, price }`. The cell records `from_slots` on the vault session (`VaultSession::expansion_offer`) and shows dialog 60110, "Expand vault": one screen with one Generic 1 button, authored as a cooked-data override. It also sends a chat line with the size and the price, because the dialog text cannot carry a seed value;
- at 100 slots, it sends no offer and tells the player the vault is full.

The offer is dropped (`expand_offer_dropped`) if the session ended or moved to another speaker before it arrived.

> **Not reachable from the client yet.** Dialog 60110's override is held in `QUARANTINED_DIALOG_OVERRIDES` ([`dialog_overrides/mod.rs`](../../crates/resources/src/base/dialog_overrides/mod.rs)) with the debug-hub dialogs: pushing Cimmeria-authored dialog overrides crashed a client on map load (#943), and 60110 shares two of the suspect fields (a screen id above 200000, a Generic button). Until the quarantine lifts, the client has no entry for 60110, shows no dialog, and nothing can be bought from the UI; the chat line with the price still arrives. The server side below is complete and tested.

**The purchase.** Pressing the button sends `dialogButtonChoice(60110, 8)`. The #479 gate checks only that the dialog was shown, so the button is not an authority check. The cell routes the answer by dialog id to the purchase path, never to a content chain. There it takes the offer (one-shot) and a **fresh** vault verdict, `vault_access`, the rule every bank move takes, and forwards both as `BankCellToBase::Expand`. A close (`-1`) is never a purchase. The base then:

1. refuses unless the verdict opens the personal vault (a Banker in range, or a GM session) and the session held an offer;
2. buys in one statement ([`bank_expand/persist.rs`](../../crates/base-session/src/base/bank_expand/persist.rs)): `bank_slots + 10` and `naquadah - price` together, only while the row is still at the offered size, below 100, with a price row for the step and the cash to pay it. The offered size is the replay key: a second send for the same offer matches no row and charges nothing;
3. on success, re-declares every container with `onBagInfo` (container 17 at the new size), sends `onCashChanged` and a chat line. BV-E1 infers that the new size resizes an open vault window live through `InventoryUpdateContainerSize`; UAT confirms it. The chat line is sent either way, so the press is acknowledged even with the window closed.

A zero-row purchase is classified by a follow-up read, replay key first. Every refusal logs `expand_rejected` (WARN, `bank`) with a stable `reason`, the size and the cash it read, and sends a chat line:

| `reason` | When |
|---|---|
| the verdict's label (`no_vault_session`, `banker_out_of_range`, `banker_gone`, `banker_other_space`, `vault_session_other_space`, `player_missing`, `vault_scope_mismatch`) | no session, walked away, or changed space |
| `no_offer` | the session holds no offer (answered twice, or after the vault was reopened) |
| `replay` | the vault is no longer at the offered size |
| `at_ceiling` | already at 100 slots |
| `insufficient_cash` | less naquadah than the price (`price` logged) |
| `price_missing` | no price row for the step |
| `player_row_missing`, `db_unavailable`, `query_failed` | infrastructure |

A purchase logs `expand` (INFO, `bank`) with `bank_slots_before`/`after`, `price`, `cash_before`/`after`, and the Banker (`banker_id`, `distance`) or `gm_override=true`. The cell side runs in the INFO span `bank.expand`, the purchase in `bank.expand_purchase`, the quote in `bank.expansion_quote`.

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
