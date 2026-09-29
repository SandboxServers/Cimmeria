---
name: request-ammo-change-instance-id
description: requestAmmoChange (cell method 42) ItemId is the weapon INSTANCE id (sgw_inventory.item_id), not the design id; the handler was fixed in AM-03 (#534)
metadata:
  type: project
---

`requestAmmoChange(ItemId, AmmoType)` carries the weapon's **instance** id
(`InvItem.id`), per issue #534's decompile (`FUN_00e1ee10` sends `item+0x0C`,
filled from `id` by the item ctor `FUN_00d21750`) and
`inventory-wire-formats.md` ("Weapon instance ID"). Until AM-03 (2026-09-28)
the handler matched `BandolierItem.item_id` (design), so real client swaps
most likely fell to "item not in bandolier".

**Why:** a design-keyed match makes every whitelist test pass in unit tests
while doing nothing in game; tests that build args with the design id hide it.

**How to apply:** any cell handler that takes an item id from the client:
check whether the client sends `id` (instance) or `dbid` (design) before
matching. Match `BandolierItem.instance_id`; read `item_defs` by the matched
slot's design id, never by the wire value. Related: [[grant-placement-and-loot-handback-traps]].
