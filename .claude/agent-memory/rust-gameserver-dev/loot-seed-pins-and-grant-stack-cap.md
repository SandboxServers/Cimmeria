---
name: loot-seed-pins-and-grant-stack-cap
description: Adding rows to an existing loot table breaks exact-row pins in three tests; GrantItem does not cap a looted count at max_stack_size
metadata:
  type: project
---

Adding a row to an existing `resources.loot` table is never "pure data". Three tests pin tables exactly (found in AM-05, #1050, 2026-09-28):

- `cell-catalog` `live_db_castle_loot.rs` pins table 7 exactly. It also rejects any row on tables 4-7 that is not a working consumable, a crafting component or (since AM-05) an ammo item.
- `cell-content` `chain_replay_tests/castle_loot_containers.rs` pins tables 8 and 9 exactly.
- `cell-methods` `debug_hub_dispatch_tests` requires every probability-1 row on table 3 to be granted by Loot All.

AM-05 kept the exact pins and added `AND NOT EXISTS (SELECT 1 FROM resources.ammo_item_types a WHERE a.item_id = loot.design_id)`. The ammo rows are pinned separately in `live_db_ammo_loot.rs`. Follow the same pattern for a new campaign's rows.

`GrantItem` inserts the looted count as one stack without capping it, so a loot row whose `max_quantity` is above the item's `max_stack_size` creates an over-full stack (reported by AM-06).

**How to apply:**

- Keep `max_quantity` at or below the item's `max_stack_size`.
- Check the three test files above before seeding rows onto tables 3-11.
- Seed a new block with the default `loot_id` from `loot_loot_id_seq`, loaded after `loot.sql`, so its ids never collide with explicit ids in `loot.sql`.
