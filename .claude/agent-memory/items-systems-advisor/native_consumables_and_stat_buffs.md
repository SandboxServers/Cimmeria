---
name: native-consumables-and-stat-buffs
description: items_event_sets event 5 is LIVE since 2026-09-28 for native consumables (heals + stimpacks); the 597 filler gate, the consume-first round trip, pulse_count=1 never registers, stat-keyed buff stacking, what stays unwired and why
metadata:
  type: project
---

Landed on branch `feat/item-use-abilities` (2026-09-28). Supersedes the "event 5 is dead" parts of [[item_use_trigger_mechanism]] and [[items_event_sets_dual_purpose]].

**Which items are native.** `cell::content::consumable_use::classify`: an `items_event_sets` row with `event_id = 5`, ability not 597, every effect's `script_name` in {`HealHealth`, `HealFocus`, `StatBuff`}, and no `item_use` chain for the item (`ChainEngine::has_item_use_chain`; a chain owns its item, so item 19 / chain 1034 stays a chain item). 46 items: 12 health heals (2893 AND 4735, a duplicate "Health Slappack TC1" row on ability 648), 10 focus heals, 24 stimpacks. The live-DB guard `live_db_exactly_the_intended_items_are_native_consumables` pins the set.

**The 597 gate is load-bearing, not cosmetic.** 158 event-5 rows bind ability 597 "Heal Focus" to unrelated mission items (1893 Opheltes's Injection, 1937 Banged-up Radio, 2216 DHD Bypass Card, 2133 "Med Kit" ...). 597's effect 659 has `script_name = 'HealFocus'` and `HealPercentage 35`, so without the gate every quest item would heal 35% focus. Mutation proof: removing the gate fails 4 tests incl. the live 1893 round trip.

**Consume first, then apply.** Cell refuses (dead / all healed pools full) with onErrorCode + CHAN_FEEDBACK; else `CellToBaseMsg::ConsumeItemForUse` → base `remove_instance` (FOR UPDATE, `AND type_id = $3` guard) → on commit only, `BaseToCellMsg::ItemUseConsumed` sent directly (at most once, NOT via outbox) → cell `apply_consumed_item` → `effect_apply::apply_ability_effects`. This closed a real double-heal: two clicks on the last slappack both passed a chain's `stat_below_max` before either `remove_item` committed.

**pulse_count = 1 never registers** (`register_active_effect`: `remaining = total - 1 = 0` → returns false, no `active_effects`, no `on_remove`). Every stimpack row is `pulse_count 1, pulse_duration 3600, flags 2`. Timed buffs therefore live in `CellEntity::stat_buffs` (entity `cell_entity/stat_buff.rs`), expired by `stat_buff_tick` (cell-combat `effects/stat_buffs/`), which also sends the queued `onTimerUpdate` starts/clears. The AbsorbShield/Stun doc comments claiming pulse_count=1 "ages out" are stale (combat's to fix).

**Stacking is keyed by STAT, not effect** (deliberate deviation from `PetBuff`): Mark III Coord (+5) then Mark V Coord (+7) = +7, not +12; different stats coexist. Bounds widen (`cur == max` for primary attributes) and removal restores cur/min/max exactly.

**Lifecycle:** not persisted — lost on logout / gate travel / cross-world respawn (entity rebuilt; `InitPlayerState` also clears any ledger). Survives same-world death: stim rows carry `EF_Offline_Time_Counts` (2), not `EF_ClearOnDeath` (4); `resolve_death` strips only ClearOnDeath buffs.

**Deliberately unwired:** Stealth (3221, 4067-4075), Energy (3227, 4076-4084), Disguise (3187, 4056-4064) boosts — `STEALTH_RATING`, `ENERGY_POOL` (always [0,0,0]), `DISGUISE_RATING` have no server reader except the GM `.stats` display. They stay silent on use (UAT K20). 6245-6252/6254/6256 have no event-5 row.

**Magnitudes** are new `effect_nvps` rows 400-462, each the number in the effect's `effect_desc` (checked by `live_db_every_consumable_magnitude_is_its_effect_description`). `HealAmount` (flat) is read before `HealPercentage` by the one `HealHealth` / `HealFocus` script (`effects/heal.rs`). Stim NVP names: Coordination, Engagement, Fortitude, Intellect (→ INTELLIGENCE), Morale, Perception.

Also: event 5 is probed by `resolve.rs::is_ability_granted_by_active_weapon` for bandolier weapons — unrelated to consumables.
