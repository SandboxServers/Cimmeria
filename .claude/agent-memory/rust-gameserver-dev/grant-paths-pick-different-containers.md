---
name: grant-paths-pick-different-containers
description: gmGiveItem grants into INV_MAIN, but loot and content grant_item use the first container_sets entry (often 17, the bank); item-use rules keyed on bag must allow both
metadata:
  type: project
---

The three item grant paths do not agree on the destination bag (checked 2026-09-27, CR-15):

- `gmGiveItem` (`crates/cell-console/src/cell/console/gm/give.rs`) sends `GrantItem` with `container_id: INV_MAIN` (1) explicitly.
- Loot (`crates/cell-interactions/src/cell/interactions/loot.rs`) and the content engine's `grant_item` (`crates/cell-content/src/cell/content/executor/inventory.rs::item_container`) use `space_mgr.item_containers`, the **first** element of `resources.items.container_sets`, defaulting to 1.

Crafting items (Blueprint items, Paradigm Guides, Field Crafting Tools) all have `container_sets = '{17,15}'`, so loot/content put them in the bank (17), GM grants put them in the main bag, and neither lands in the crafting bag (15) that tools and item use care about. CR-16 is meant to make grants fall through past storage containers.

**Why:** an advisor proposed restricting crafting item use to bag 15 only because 1 is not in `container_sets`; that would have refused every GM-granted item. Loot of these items is also a data-loss risk until CR-16 (loot removes from the corpse before the base accepts the grant).

**How to apply:** when a rule depends on which bag an item is in, check all three grant paths, not `container_sets` alone; don't add loot rows for `{17,…}` items before CR-16 lands. Related: [[stat-with-no-consumer-trap]].
