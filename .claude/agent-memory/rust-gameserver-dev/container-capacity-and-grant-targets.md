---
name: container-capacity-and-grant-targets
description: Raising any container's bag_max_slots silently opens GRANTS into it (loot/content grant into container_sets[1]); no seeded item allows both 1 and 15; the move allowlist is separate from capacity.
metadata:
  type: project
---

`bag_max_slots` (now `crates/entity/src/inventory.rs`, BV-01) is not only the move path's slot bound: `reserve_free_inventory_slots` uses it for every grant. Loot (`cell-interactions/.../loot.rs`) and content `grant_item` grant into the item's `container_sets[1]` (`cell-catalog/.../spawner/loot.rs::load_item_containers`), and 752 seeded crafting items are `{17,15}`, so their preferred container is the bank (17). Before BV-01 those grants failed as "container full" because 17 had capacity 0; BV-01 gave 17-20 a capacity and had to add `grant_container_refused` to keep them failing.

Also: no seeded item's `container_sets` contains both 1 and 15 (groups: `{3,1,17}`, `{2}`, `{17,15}`, `{1,17}`, `{1,N,17}`), so a 1 <-> 15 move test needs a synthetic `resources.items` row.

**Why:** a capacity change looks like a UI/declaration change but is also a grant-routing change; nobody reading `bag_max_slots` would guess loot routes through it.

**How to apply:** before changing any `bag_max_slots` arm, grep `reserve_free_inventory_slots` callers and the `container_sets[1]` distribution for that container. The player-movable allowlist lives in `move_/container_policy.rs` and is independent of capacity. See [[stat-with-no-consumer-trap]] for the opposite shape (a value with no consumer).
