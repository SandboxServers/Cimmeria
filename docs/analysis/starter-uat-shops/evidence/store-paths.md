# Store paths: Quartermaster open failure, sell, buyback, repair, recharge, trainer

Packet SS-01. Read-only research against the code at this worktree. No lab tool was run. Line numbers are from the files as they stand on branch `ss-01`.

## 1. Why the stasis-room Quartermaster opened nothing (2026-10-12)

Template 300 (spawn 400) is the stasis-room Quartermaster. Its `interaction_type` is 65536 (`INT_VENDOR_GENERAL`), per `db/resources/Entities/Seed/entity_templates.sql:1011`. The flag maps to `NpcInteractionType::Vendor` in `crates/cell-world/src/cell/space_manager/spawn.rs:441-453`. Template 415 (the Quartermaster clone) carries the lists `1300/2/2/2` (`entity_templates.sql:1098-1099`).

The chain on a right-click is:

1. Client right-click sends `interact`. The cell checks the distance against `MAX_INTERACT_DISTANCE` (5.0) at `crates/cell-interactions/src/cell/interactions/dispatch/interact.rs:84-98`. A too-far click answers with a line and stops.
2. `Some(NpcInteractionType::Vendor)` at `interact.rs:211-221` calls `send_store_open` (`crates/cell-interactions/src/cell/interactions/vendor.rs:12-79`), which sends `CellToBaseMsg::OpenVendorStore`.
3. The base routes it in `crates/base-world-entry/src/base/world_entry/cell_dispatch/vendor_dispatch.rs:31` to `handle_open_vendor_store` (`crates/base-methods/src/base/world_entry/methods/vendor/store.rs:37-163`).
4. In `store.rs`, a missing database pool gives `failed no_database` (line 68), and a missing template gives `refused no_vendor_template` (line 84). The `no_vendor_template` branch still sends an empty store window before the refusal, so a window with no rows is the server's answer to a request with no template. `template_not_found` (line 100) and `template_query_failed` (line 113) are the other failure branches.
5. `store_opened` is logged only after the lists load and the window is sent (`telemetry.rs:16`, `:259-265`). Its fields are `buy_count`, `sell_count`, `buyback_count`, `repair_count` and `recharge_count`.

What the round-1 evidence shows: `docs/analysis/haiku-lab-uat/ledger.md:69` (08:38) records that the Quartermaster never opened `VendorWin`, that a "Vendor / Items and Inventory" popup appeared, that clicking `Popup_Item1` did nothing, and that the server evidence was "none". There is no server row for that click. So the failure sits at one of three points, and round 1 cannot tell them apart:

- The click: the popup item did not send `interact` (or it was occluded or hidden). The second line of the popup, "Items and Inventory", is the text of tutorial dialog 5863, which the client raises by itself (`.claude/agent-memory/mission-systems-advisor/dialog-chain-authoring-rules.md`, section 6). So the popup may be a client tutorial rather than the vendor menu; the round-1 notes do not say which, and a human click would settle it.
- The server: a refusal before `store_opened`. The refusal lines are `no_database`, `no_vendor_template`, `template_not_found` and `template_query_failed`.
- The window: the store window arrived but did not show, which the empty `no_vendor_template` window would look like.

The proof line that the store opened is the server event `store_opened` for `vendor_template_id = 300` (or 415 for the clone) with `buy_count > 0`, logged by the `vendor` target at INFO. A click that never reaches the server shows as no `interact` row. A row `interact: vendor → OpenVendorStore` (`interact.rs:217`) without a following `store_opened` means the server failed. A `refused` or `failed` row names the reason. The round-2 check should first look for the `interact` row, then `store_opened`, then the window.

## 2. Sell, buyback, repair and recharge: client call, server method, lab tool

The client Lua is not in this repository, so the window and button names below come from the lab notes, not from the client's source. Where the name is not confirmed, the row says so.

| Action | Client call or control | Server message and handler | Lab tool |
|---|---|---|---|
| Buy | `client_vendor_buy` (`crates/lab/src/supervisor/ui/vendor_buy.rs`): sets the row quantity, then clicks `Vendor_BuyButton`. Window names known: `VendorWin`, `Vendor_BuyTabInactive`, `VendorWin__auto_closebutton__` | `PurchaseItems` (opcode 78), `purchase/mod.rs`; `transaction` action `buy` | existing tool |
| Sell | No named control in the repo. The round-1 sell half was a client-read check only (README step 9). The sell is a drag of a carried item onto the vendor window, which is an assumption to test | `SellVendorItems` (79), `handle_sell_vendor_items` (`vendor/sell/mod.rs:43`); `transaction` action `sell` | `client_drag_drop` (N1, unverified) |
| Buyback | Tab `Buy` + `Back` in the vendor window, named "Buyback" in the BANK-C3 brief. The tab click is `client_window_click` with `text:"Buyback"`. Container name `Buyback` (container 16) is an assumption from the bank telemetry `source_container_id=16` | `BuybackVendorItems` (80), `handle_buyback_vendor_items` (`vendor/buyback/mod.rs:37`); `transaction` action `buyback` | `client_window_click` (N1) then `client_drag_drop` (N1) |
| Repair | No repo evidence of the tab or button name. Repair is `RepairItems` (81) | `RepairInventoryItems` (`cell_dispatch/inventory_dispatch.rs:157`, `repair_inventory_items` at `:456`) to `handle_repair_inventory_items` (`vendor/repair.rs:124`) to `handle_paid_repair_inventory_items` (`vendor/paid_repair/mod.rs:36`), which is the live path; `transaction` action `repair` or `repair_all` | none confirmed; needs `client_window_read` kind `vendor` with `include_nodes` on a live round (N3 through `client_lua_eval` if the Lua name is found) |
| Recharge | No repo evidence of the tab or button name. Recharge is `RechargeItems` (82) | `RechargeInventoryItems` (`inventory_dispatch.rs:175`, `recharge_inventory_items` at `:481`) to `handle_recharge_inventory_items` (`vendor/recharge.rs:16`) to `handle_paid_recharge_inventory_items` (`vendor/paid_recharge/mod.rs:36`); `transaction` action `recharge` or `recharge_all` | none confirmed, as for repair |

The cell routes the store methods in `crates/cell-methods/src/cell/cell_methods/player/vendor/store_ops.rs:185-215` (`REPAIR_ITEMS` at `:207`, `RECHARGE_ITEMS` at `:213`). The method ids are in `crates/wire/src/cell/cell_methods/player/constants.rs:16-20`. The base uses the template id from the player's vendor session, not the client's (`repair.rs:119-123`, `recharge.rs:12-15`).

## 3. Can an in-game action or GM command lower durability or drain charge?

None. A search of the non-test crates for durability or charge writes finds only:

- the repair UPDATE that adds to durability with the cap and floor in the SQL (`vendor/repair.rs:62`);
- the repair that sets `durability = 100` (`vendor/paid_repair/mod.rs:211`);
- the recharge that sets `charges = ri.charges` (`vendor/paid_recharge/mod.rs:207`);
- the grant that writes the initial values (`base-methods/.../inventory/grant/persist.rs`), and the lab fixture items.

`/gmrechargeitem` (`docs/commands.md`) raises charge. The consumable use path removes one stack unit ("item used: one unit consumed" in the ledger), not charge. Combat (`cell-combat/.../damage/pipeline.rs`) does not touch durability. So nothing in-game or GM lowers durability or drains charge today. D-SS3 applies.

## 4. Trainer (template 301)

Template 301 has `interaction_type` 128 and `trainer_ability_list_id` 1 (`entity_templates.sql:1019`). Right-click opens the trainer window (`interact` then `trainer_resend`). The round-1 buy was a real click on the node: `docs/analysis/haiku-lab-uat/ledger.md:44`. The client call behind "Train" is not in the repo; the buy is a click on the node in the AbilityWin.

Server events, in order:

1. `train_requested` at `crates/cell-methods/src/cell/cell_methods/player/vendor/train.rs:105`.
2. `train_persisted` at `crates/base-methods/src/base/world_entry/methods/progression/train_ability.rs:255`, after `handle_train_ability` at `:92`.
3. `granted` for the ability.

Refusals are `train_rejected` (`train.rs:235-291`), `train_negative_cost` and `train_player_mismatch` (`train_ability.rs:130`, `:174`). `train_raw_cost_zero` (`train.rs:164`) is a warning, not a refusal.

## Recipes

| Action | Tool call (exact args) | Tier | Server event that proves it |
|---|---|---|---|
| Open a vendor (any store) | `client_entity_find` {"lease_id":"<LEASE_ID>","name":"<vendor>","fields":["matches"]}, then `client_camera` {"lease_id":"<LEASE_ID>","face_entity_id":<id>,"zoom_to":400}, then right-click the entity at 3.5 to 4.5 m (`client_world_click`) and choose "Vendor" in the popup | N1 | `interact` row, then `store_opened` with the template id. No `store_opened` means read the refusal reason |
| Buy one item | `client_vendor_buy` (existing tool; sets the row quantity, then clicks `Vendor_BuyButton`) | N1 | `transaction` action `buy`, with `price` and `cash_after` |
| Sell one item (unverified) | `client_drag_drop` from {"container":"Main","slot":<n>} to {"container":"<vendor window>"} | N1 | `transaction` action `sell`. Absent: the drag did not start, so read `client_inventory` first |
| Buyback one item | `client_window_click` {"text":"Buyback","root":"<vendor window>"}, then `client_drag_drop` from {"container":"Buyback","slot":0} to {"container":"Main","slot":<empty>} | N1 | `transaction` action `buyback` |
| Repair (unconfirmed control) | `client_window_read` {"lease_id":"<LEASE_ID>","kind":"vendor","include_nodes":true,"max_nodes":150} to find the repair control, then `client_window_click` on it | N1 once named; N3 otherwise | `transaction` action `repair` or `repair_all` |
| Recharge (unconfirmed control) | As repair, with the recharge control | N1 once named; N3 otherwise | `transaction` action `recharge` or `recharge_all` |
| Train one node | Right-click the trainer at 3.5 to 4.5 m, then `client_window_click` on the node in AbilityWin | N1 | `train_requested`, then `train_persisted`, then `granted` for the id. `train_rejected` names the refusal |
