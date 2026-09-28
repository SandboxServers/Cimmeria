# Tests — `game`

> **Type**: reference  
> **Audience**: engineers  
> **Last updated**: 2026-09-28 *(#803 removed the 7 `commands/` and 9 `missions/` tests; #614 removed the 9 `social/` tests; other rows still the 2026-05-04 snapshot, so 4 newer `player.rs` tests are unlisted)*  
> **Total tests**: 44 listed (48 in the crate)  
> **CI-gated**: yes  
> **Index**: [README](README.md) | **Playbook**: [TESTING.md](../../../TESTING.md)

Game mechanics — combat, abilities, stats, effects, inventory, and world simulation.

## All tests (44 listed)

| Test | Kind | System / Feature | Added | What it tests | Notes |
|---|---|---|---|---|---|
| [new_being_at_full_health](../../../crates/game/src/being.rs#L79) | unit | Being | 2026-03-03 | Asserts equality on `b.health` |  |
| [take_damage_applies_armor](../../../crates/game/src/being.rs#L87) | unit | Being | 2026-03-03 | Asserts equality on `dealt` |  |
| [take_damage_minimum_one](../../../crates/game/src/being.rs#L95) | unit | Being | 2026-03-03 | Asserts equality on `dealt` |  |
| [take_damage_clamps_at_zero](../../../crates/game/src/being.rs#L103) | unit | Being | 2026-03-03 | Asserts equality on `b.health` |  |
| [heal_clamps_at_max](../../../crates/game/src/being.rs#L111) | unit | Being | 2026-03-03 | Asserts equality on `healed` |  |
| [health_percentage](../../../crates/game/src/being.rs#L120) | unit | Being | 2026-03-03 | Asserts on `(b.health_percentage() - 0.5).abs() < f32::EPSILON` |  |
| [new_ability_is_ready](../../../crates/game/src/combat/abilities.rs#L82) | unit | Combat / Abilities | 2026-03-03 | Asserts on `a.is_ready()` |  |
| [cooldown_ticks_down](../../../crates/game/src/combat/abilities.rs#L88) | unit | Combat / Abilities | 2026-03-03 | Asserts on `!a.is_ready()` |  |
| [damage_event_minimum_one](../../../crates/game/src/combat/damage.rs#L83) | unit | Combat / Damage | 2026-03-03 | Asserts equality on `result.final_amount` |  |
| [damage_event_no_armor](../../../crates/game/src/combat/damage.rs#L90) | unit | Combat / Damage | 2026-03-03 | Asserts equality on `result.final_amount` |  |
| [effect_expires_after_duration](../../../crates/game/src/combat/effects.rs#L83) | unit | Combat / Effects | 2026-03-03 | Asserts on `!effect.is_expired()` |  |
| [tick_fires_at_interval](../../../crates/game/src/combat/effects.rs#L93) | unit | Combat / Effects | 2026-03-03 | Asserts equality on `ticked.len()` |  |
| [base_values_returned_without_modifiers](../../../crates/game/src/combat/stats.rs#L114) | unit | Combat / Stats | 2026-03-03 | Asserts on `(block.get(Stat::MaxHealth) - 100.0).abs() < f32::EPSILON` |  |
| [flat_modifier_adds_to_base](../../../crates/game/src/combat/stats.rs#L121) | unit | Combat / Stats | 2026-03-03 | Asserts on `(block.get(Stat::Damage) - 15.0).abs() < f32::EPSILON` |  |
| [multiplier_scales_total](../../../crates/game/src/combat/stats.rs#L133) | unit | Combat / Stats | 2026-03-03 | Asserts on `(block.get(Stat::Damage) - 15.0).abs() < f32::EPSILON` |  |
| [remove_modifiers_by_source](../../../crates/game/src/combat/stats.rs#L145) | unit | Combat / Stats | 2026-03-03 | Asserts on `(block.get(Stat::Armor) - 20.0).abs() < f32::EPSILON` |  |
| [empty_container](../../../crates/game/src/inventory/containers.rs#L89) | unit | Inventory / Containers | 2026-03-03 | Asserts equality on `c.free_slots()` |  |
| [add_and_retrieve_item](../../../crates/game/src/inventory/containers.rs#L96) | unit | Inventory / Containers | 2026-03-03 | Asserts equality on `slot` |  |
| [full_container_rejects_item](../../../crates/game/src/inventory/containers.rs#L105) | unit | Inventory / Containers | 2026-03-03 | Asserts on `result.is_err()` |  |
| [remove_item_frees_slot](../../../crates/game/src/inventory/containers.rs#L113) | unit | Inventory / Containers | 2026-03-03 | Asserts on `removed.is_some()` |  |
| [single_stack_not_stackable](../../../crates/game/src/inventory/items.rs#L81) | unit | Inventory / Items | 2026-03-03 | Asserts on `!item.is_stackable()` |  |
| [stackable_item_accepts_units](../../../crates/game/src/inventory/items.rs#L94) | unit | Inventory / Items | 2026-03-03 | Asserts equality on `overflow` |  |
| [stack_overflow_returns_remainder](../../../crates/game/src/inventory/items.rs#L109) | unit | Inventory / Items | 2026-03-03 | Asserts equality on `overflow` |  |
| [empty_table_drops_nothing](../../../crates/game/src/inventory/loot.rs#L85) | unit | Inventory / Loot | 2026-03-03 | Asserts on `drops.is_empty()` |  |
| [guaranteed_drop](../../../crates/game/src/inventory/loot.rs#L92) | unit | Inventory / Loot | 2026-03-03 | Asserts equality on `drops.len()` |  |
| [zero_chance_never_drops](../../../crates/game/src/inventory/loot.rs#L107) | unit | Inventory / Loot | 2026-03-03 | Asserts on `drops.is_empty()` |  |
| `aggro_range_check` | unit | Mob | 2026-03-03 | Asserts on `mob.is_in_aggro_range(&mob_pos, &close)` | not found in the tree as of 2026-07-25 — location unknown |
| [npc_roles](../../../crates/game/src/npc.rs#L64) | unit | Npc | 2026-03-03 | Asserts on `!npc.has_dialog()` |  |
| [new_player_starts_at_level_1](../../../crates/game/src/player.rs#L112) | unit | Player | 2026-03-03 | Asserts equality on `p.level` |  |
| [xp_for_next_level_uses_table](../../../crates/game/src/player.rs#L120) | unit | Player | 2026-03-09 | Asserts equality on `p.xp_for_next_level()` |  |
| [grant_xp_below_threshold_no_level_up](../../../crates/game/src/player.rs#L126) | unit | Player | 2026-03-09 | Asserts equality on `p.level` |  |
| [grant_xp_at_threshold_triggers_level_up](../../../crates/game/src/player.rs#L134) | unit | Player | 2026-03-09 | Asserts equality on `p.level` |  |
| [grant_xp_multi_level_up](../../../crates/game/src/player.rs#L142) | unit | Player | 2026-03-09 | Asserts equality on `p.level` |  |
| [grant_xp_at_max_level_no_overflow](../../../crates/game/src/player.rs#L149) | unit | Player | 2026-03-09 | Asserts equality on `p.level` |  |
| [xp_for_next_level_at_max_returns_max](../../../crates/game/src/player.rs#L159) | unit | Player | 2026-03-09 | Asserts equality on `p.xp_for_next_level()` |  |
| [new_player_has_zero_training_points](../../../crates/game/src/player.rs#L166) | unit | Player | 2026-03-09 | Asserts equality on `p.training_points` |  |
| [grant_xp_grants_training_points_on_level_up](../../../crates/game/src/player.rs#L172) | unit | Player | 2026-03-09 | Asserts equality on `p.training_points` |  |
| [multi_level_up_grants_cumulative_training_points](../../../crates/game/src/player.rs#L179) | unit | Player | 2026-03-09 | Asserts equality on `p.training_points` |  |
| [full_level_progression_1_to_20](../../../crates/game/src/player.rs#L186) | unit | Player | 2026-03-09 | Asserts equality on `p.level` |  |
| [xp_table_is_monotonically_nondecreasing](../../../crates/game/src/player.rs#L200) | unit | Player | 2026-03-09 | Asserts on `LEVEL_XP[i] <= LEVEL_XP[i + 1]` |  |
| [sphere_containment](../../../crates/game/src/world/regions.rs#L95) | unit | World / Regions | 2026-03-03 | Asserts on `r.contains(&Vector3::new(5.0, 0.0, 0.0))` |  |
| [box_containment](../../../crates/game/src/world/regions.rs#L102) | unit | World / Regions | 2026-03-03 | Asserts on `r.contains(&Vector3::zero())` |  |
| [crossing_detection](../../../crates/game/src/world/regions.rs#L115) | unit | World / Regions | 2026-03-03 | Asserts equality on `r.check_crossing(&outside, &inside)` |  |
| [initial_spawn_respects_max](../../../crates/game/src/world/spawning.rs#L105) | unit | World / Spawning | 2026-03-03 | Initial spawn respects max |  |
| [death_triggers_respawn_timer](../../../crates/game/src/world/spawning.rs#L130) | unit | World / Spawning | 2026-03-03 | Asserts equality on `set.current_alive` |  |
