---
name: grant-paths-starting-clip-and-archetype-param
description: Which item-grant paths share the GrantItem INSERT (and so start firearms loaded), where the AmmoSlot stat is mirrored, and why an absent `archetype` param still reads -1 (CS-01b, 2026-10-05).
metadata:
  type: project
---

**One INSERT, three callers.** `CellToBaseMsg::GrantItem` -> `handle_grant_item` /
`handle_loot_grant` -> `grant/persist.rs::persist_grant` is used by content
chains (`Action::GrantItem`, which is also how mission rewards grant items),
loot pickup (`cell-interactions/loot`) and GM `gmGiveItem`
(`cell-console/gm/give.rs`). Since CS-01b that INSERT writes
`ammo = CASE WHEN clip_size > 0 THEN clip_size ELSE charges END`, and
`equip_epilogue` sends `BandolierItem::granted_ammo(clip_size)` to the cell.
Paths with their **own** INSERT that were not changed: vendor purchase
(`vendor/purchase/mod.rs`, `purchase_helpers.rs`: `ammo` left at the column
default, so a bought gun is empty), buyback/sell (copy the row), crafting
grant, mail restore, character creation (`starter_kit.rs`, already loaded).

**AmmoSlot stat.** The client's bandolier counter reads `Stat[AMMO_SLOT_1+slot]`
(49-53), not the item. `handle_update_bandolier_item` now mirrors the item into
that stat and pushes `onStatUpdate` (it used not to, so GM/loot grants into the
bandolier showed no rounds). The content executor still seeds it optimistically
before the base round-trip.

**`archetype` param.** Every player-scoped `fire_*` sets it from
`entity.archetype_id`; dialog open/choice were the only gap (fixed CS-01b).
`npc_flanked` deliberately has none (NPC source). The evaluator's -1 default for
a missing value was **kept**: `mission_701/arrival.rs::live_db_player_without_an_archetype_gets_the_human_branch`
pins it as intentional (archetype-less player gets the Human branch, not a dead
end). Fail-closed would also turn the ~20 seeded `neq 8` chains into dead ends
for such a player. Seeded archetype conditions (40 chains, 2026-10-05) sit only
on interact_tag/template, player_loaded, mission_*, enter_region,
entity_dead_tag and stargate_dialed triggers.

**Archetype ids** are the 0-based `EArchetype` index: Any 0, Soldier 1,
Commando 2, Scientist 3, Archeologist 4, Asgard 5, Goa'uld 6, Shol'va 7,
Jaffa 8. The shared `:5433` `sgw` DB can be stale against the seed; scan
`db/resources/` instead when counting seeded chains.
