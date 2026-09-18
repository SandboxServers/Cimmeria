---
name: vendor-trainer-seed-gap
description: Vendor buy/sell/repair/recharge Rust plumbing is complete and correct, but vendor content is essentially unseeded system-wide (not just Harset) — only one test entity_template wires to real item_lists
metadata:
  type: project
---

Confirmed 2026-09-17 during a Harset zone evidence pass (READ-ONLY).

**The mechanism is real and complete:**
- `crates/services/src/base/world_entry/interactions/vendor.rs::send_store_open` sends `CellToBaseMsg::OpenVendorStore { vendor_template_id, .. }` (vendor_template_id = the NPC entity's `template_id`, read off the spawned entity, not a hardcoded value).
- `crates/services/src/base/world_entry/methods/vendor/store.rs` (~line 77-79) resolves it: `SELECT buy_item_list, sell_item_list, repair_item_list, recharge_item_list FROM resources.entity_templates WHERE template_id = $1`.
- `crates/services/src/base/world_entry/methods/vendor/data/mod.rs` (`load_store_buy_items` etc.) then joins `resources.item_list_items` / `resources.item_list_prices` on those list ids to build the actual store payload.
- Buy/sell/repair/recharge/buyback each have their own module under `crates/services/src/base/world_entry/methods/vendor/` (`purchase/`, `sell/`, `repair.rs`, `recharge.rs`, `buyback/`) with their own tests.

**The content is not there.** `db/resources/Items/Seed/item_lists.sql` has exactly 2 rows total, both test fixtures: `(1, 'Test vendor buy list')`, `(2, 'Test vendor sell/repair/recharge list')`. `item_list_items.sql` has 6 INSERTs, `item_list_prices.sql` has 1. Across all 153 rows in `db/resources/Entities/Seed/entity_templates.sql`, exactly **one** (`template_id = 25`) has any non-null `buy_item_list`/`sell_item_list`/`repair_item_list`/`recharge_item_list`/`trainer_ability_list_id` — and it points at the two test lists (1, 2). Every other NPC template, including all ~60 Harset vendor/trainer NPCs implied by moniker text rows in `db/resources/Texts/Seed/texts.sql` (grep `DN_npc_ven_Harset_`, `DN_npc_ven_TBD_Harset_`, `DN_npc_trn_TBD_Harset_`), has these columns NULL.

**Display-name monikers exist for the Harset vendor/trainer roster even though the item lists don't.** e.g. `DN_npc_ven_Harset_BetaCompVendor` → "Beta Component Vendor" (texts.sql:41534), `DN_npc_ven_BasicProcurementOffice_Harset_HumanTier0MissionTo` → "Basic Equipment Procurement Officer" (texts.sql:55114), `DN_npc_ven_GldArmLo'taur_Harset_Goa'uldTier1-5MissionToken` → "Goa'uld Armor Lo'taur" (texts.sql:55124), `DN_npc_Harset_Banker` → "Storage Lotaur" (texts.sql:41876, confirms a bank/storage NPC concept existed in original design). Several rows carry an empty-string `text` value even in the moniker table itself (e.g. `DN_npc_ven_Harset_HumanWeaponsTier01` at texts.sql:39068) — meaning even the *original* SGW developers never finished naming some of these, independent of anything Cimmeria has or hasn't done.

**Net: this is a pure content-authoring gap, not a code gap**, for any Harset (or other zone's) vendor/trainer rollout — populate `item_lists` + `item_list_items` (+ `item_list_prices` for repair/recharge) rows, then point the NPC's `entity_templates` row at them. No Rust changes needed for the base case. Related: [[item_use_trigger_mechanism]] for the parallel finding on item_use chains being a content gap too.

**No dedicated bank/storage service exists.** Grepped for "bank" as a service concept — only hits are the `BANK` container (DB `containers.sql`: `container_id=17, name='BANK'`) being treated as a valid item-location in trade whitelisting (`crates/services/src/base/world_entry/methods/trade/tests/container_whitelist.rs`) and vendor purchase affordability checks (`crates/services/src/base/world_entry/methods/vendor/purchase_helpers.rs`, though note its test-local `const INV_BANK: i32 = 2` does NOT match the real DB container_id 17 — that's an arbitrary test fixture value, not evidence of the real container id being wrong, but worth a sanity check before reusing that constant anywhere non-test). No bank.rs / deposit/withdraw method group exists alongside `vendor/` in `crates/services/src/base/world_entry/methods/`. A "Storage Lo'taur (Bank)" NPC for Harset would today just be a MISSION/BANK-container-aware vendor-adjacent NPC at best — there's no deposit/withdraw handler to hang it on; it'd need new Rust work, not just content.
