---
name: grant-paths-starting-clip-and-archetype-param
description: Every item-acquisition path and the rounds a gun arrives with (OD-CS13, guns acquired empty), the AmmoSlot stat mirror, the removed executor active-slot guess, and why an absent `archetype` param still reads -1 (CS-01b, 2026-10-05).
metadata:
  type: project
---

**Owner rule OD-CS13 (2026-10-05): every gun is acquired empty.** 0 rounds,
the player reloads once (default reloads free, D-AM02). Equip, unequip and
swap never change the count. The coordinator first relayed it backwards
("never unloaded") and then corrected it; the correction is the rule.

**Acquisition paths and their gun ammo:**
- `CellToBaseMsg::GrantItem` -> `grant/persist.rs::persist_grant`: content
  chains (`Action::GrantItem`, also how mission rewards grant), loot pickup
  (`handle_loot_grant`), GM `gmGiveItem`. `ammo = ri.charges` = 0 for guns.
- Vendor purchase (`vendor/purchase/mod.rs`): own INSERT, `ammo` left at the
  column default 0.
- Crafting (`base-crafting/.../transaction/grant.rs`): `ammo = ri.charges`.
- Mail minted items (`mail/system/write.rs`): `ammo = ri.charges`.
- Instance moves keep `ammo`: trade (`trade/execute/swap.rs`, UPDATE
  character_id), mail escrow + take (`RESTORE_SQL`, also BM settlements),
  buyback, org vault withdraw (`move_/org/apply.rs`).
- Character creation (`starter_kit.rs`) still LOADS the clip; CS-02 owns it.
Guards: `grant::empty_on_acquire_tests`, `vendor::purchase::empty_gun_tests`,
`move_::equip_keeps_clip_tests` (live-DB).

**AmmoSlot stat.** The client counter reads `Stat[AMMO_SLOT_1+slot]` (49-53),
not the item; `onUpdateItem` carries no round count at all.
`handle_update_bandolier_item` mirrors the item into the stat and pushes
`onStatUpdate` (it used not to; a granted gun showed a blank counter).

**Executor active-slot guess removed.** `Action::GrantItem` used to insert the
gun into the cell's *active* slot before the base round-trip; the base uses
the first free slot, so an occupied active slot got a phantom weapon. The
base's `UpdateBandolierItem` is now the only writer (test
`executor::tests::grant_item`).

**`archetype` param.** Every player-scoped `fire_*` sets it from
`entity.archetype_id`; dialog open/choice were the gap (fixed CS-01b).
`world_context_contract_tests::archetype_gated_chain_fires_only_for_that_archetype`
is the drift guard: add new player dispatchers to its `ALL`. `npc_flanked`
has none on purpose (NPC source). The evaluator's -1 default was kept:
`mission_701/arrival.rs::live_db_player_without_an_archetype_gets_the_human_branch`
pins it (Human branch, not a dead end).

**Archetype ids** are the 0-based `EArchetype` index: Any 0, Soldier 1,
Commando 2, Scientist 3, Archeologist 4, Asgard 5, Goa'uld 6, Shol'va 7,
Jaffa 8. The shared `:5433` `sgw` DB can be stale against the seed; scan
`db/resources/` instead when counting seeded chains.
