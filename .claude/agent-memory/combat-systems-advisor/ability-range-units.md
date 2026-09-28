---
name: ability-range-units
description: resources.abilities min/max_range are UE3 units (100/m), converted to metres at load since #919; weapon item ranges are already metres; client addresses that prove it
metadata:
  type: reference
---

Verified 2026-09-28 (#919), headless Ghidra on SGW.exe plus the 2009 `SourceCache.en-us/CookedDataAbilities.pak`.

- **Ability ranges = UE3 units, 100 per BigWorld metre.** The cooked PAK ships the seed's numbers (1652 `MaxRange="3000"`); every non-zero seeded value is a multiple of 100. `load_ability_defs` divides by `ABILITY_RANGE_UNITS_PER_METRE`; `AbilityDef::min_range/max_range` are `f32` metres (`crates/entity/src/abilities/range.rs`). Resolve reach with `ability_max_range(def)` / `max_range_or_default()` (0 sentinel -> 30 m), never re-derive the 30.0 fallback inline.
- **Weapon ranges (`resources.items.*_range`, cooked `RangeRanges`/`MeleeRanges`) are metres** (30/35/40, melee 2/3). Do not convert them.
- Client proof chain: `FUN_00d2a470` copies cooked +0x60/+0x64 raw to runtime +0x8c/+0x90; getters `0x00d29e00`/`0x00d29e30` (weapon pair via `0x00d29da0` when flags&4); ground reticule `0x00dea330` -> `0x00eadf00` -> tick `0x00eae080` clamps in UE3 space (pawn Location +0xdc, trace to -262144) beside AE radii from `0x00d29e90` (Medium = 1000). Serializer `0x015d51c0` formats ranges `%lu` (ints, not floats).
- The client does NOT range-check targeted `useAbility` (`0x00d2ae40` emits id+target only), so the server check is the only gate.
- **#1016/#1017 (2026-09-28): one choke point.** Every targeted-cast range check calls `caster_range_bounds(def, caster, &space_mgr.weapon_ranges)` -> `RangeBounds { min, max, source }`, then `bounds.refusal(dist, caster.is_player)`. Players only are held to `min`; NPC fight tick owns its own min behaviour. Refusal = `onErrorCode 42` + DEBUG `event=cast_refused reason=target_too_close|target_out_of_range` (`use_ability/cast_range.rs`). `ability_max_range` now takes `(def, weapon)`.
- **UseWeaponRange facts.** 579 seeded abilities carry flag 4, incl. NPC default 592 and pet 1652. `SpaceManager::weapon_ranges` (from `load_weapon_ranges`, all items with a range) is NOT `item_defs` (clip>0 only; staffs have clip 0). No weapon / no reach of that kind -> ability's own range (NPCs and pets have no weapon item; refusing would disarm them). Pistol 55 reaches 20 m; most guns have a 2 m ranged min. What client `0x00d29da0` returns unarmed is unverified.

Full write-up: `docs/reverse-engineering/findings/ability-resolution-pipeline.md` § Range units; ADR decision 27. Related: [[npc-ability-sets]] (its "max_range poisoning" trap is fixed by this conversion).
