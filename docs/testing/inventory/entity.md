# Tests — `entity`

> **Type**: reference  
> **Audience**: engineers  
> **Last updated**: 2026-05-04  
> **Total tests**: 151  
> **CI-gated**: yes  
> **Index**: [README](README.md) | **Playbook**: [TESTING.md](../../../TESTING.md)

Entity lifecycle management and property synchronization. Owns the entity registry, property containers, and ghost/real entity replication.

## All tests (151)

| Test | Kind | System / Feature | Added | What it tests | Notes |
|---|---|---|---|---|---|
| [new_manager_is_empty](../../../crates/entity/src/abilities.rs#L438) | unit | Abilities | 2026-03-06 | Asserts equality on `mgr.known_count()` |  |
| [add_and_check_abilities](../../../crates/entity/src/abilities.rs#L446) | unit | Abilities | 2026-03-06 | Asserts on `mgr.has_ability(597)` |  |
| [remove_ability](../../../crates/entity/src/abilities.rs#L457) | unit | Abilities | 2026-03-06 | Asserts equality on `mgr.known_count()` |  |
| [with_abilities_constructor](../../../crates/entity/src/abilities.rs#L466) | unit | Abilities | 2026-03-06 | Asserts equality on `mgr.known_count()` |  |
| [cooldown_tracking](../../../crates/entity/src/abilities.rs#L475) | unit | Abilities | 2026-03-06 | Asserts on `!mgr.is_on_cooldown(597)` |  |
| [can_use_ability_checks](../../../crates/entity/src/abilities.rs#L490) | unit | Abilities | 2026-03-06 | Asserts on `mgr.can_use_ability(&ability).is_ok()` |  |
| [can_use_ability_cooldown_check](../../../crates/entity/src/abilities.rs#L507) | unit | Abilities | 2026-03-06 | Asserts equality on `mgr.can_use_ability(&ability).unwrap_err()` |  |
| [next_effect_id_increments](../../../crates/entity/src/abilities.rs#L518) | unit | Abilities | 2026-03-06 | Asserts equality on `mgr.next_effect_id()` |  |
| [serialize_known_abilities](../../../crates/entity/src/abilities.rs#L526) | unit | Abilities | 2026-03-06 | Asserts equality on `count` |  |
| [ability_tree_empty](../../../crates/entity/src/abilities.rs#L535) | unit | Abilities | 2026-03-06 | Asserts equality on `data.len()` |  |
| [ability_tree_with_data](../../../crates/entity/src/abilities.rs#L552) | unit | Abilities | 2026-03-06 | Asserts equality on `data.len()` |  |
| [serialize_timer_update_format](../../../crates/entity/src/abilities.rs#L571) | unit | Abilities | 2026-03-06 | Asserts equality on `data.len()` |  |
| [serialize_effect_results_empty](../../../crates/entity/src/abilities.rs#L580) | unit | Abilities | 2026-03-06 | Asserts equality on `data.len()` |  |
| [serialize_effect_results_with_stats](../../../crates/entity/src/abilities.rs#L589) | unit | Abilities | 2026-03-06 | Asserts equality on `data.len()` |  |
| [client_effect_result_serialize](../../../crates/entity/src/abilities.rs#L606) | unit | Abilities | 2026-03-06 | Asserts equality on `buf.len()` |  |
| [cleanup_expired_removes_old](../../../crates/entity/src/abilities.rs#L624) | unit | Abilities | 2026-03-06 | Asserts equality on `mgr.ability_cooldowns.len()` |  |
| [first_known_ability_returns_some](../../../crates/entity/src/abilities.rs#L642) | unit | Abilities | 2026-03-17 | Asserts on `first.is_some()` |  |
| [first_known_ability_returns_none_when_empty](../../../crates/entity/src/abilities.rs#L651) | unit | Abilities | 2026-03-17 | Asserts on `mgr.first_known_ability().is_none()` |  |
| [first_known_ability_single_element](../../../crates/entity/src/abilities.rs#L657) | unit | Abilities | 2026-03-17 | Asserts equality on `mgr.first_known_ability()` |  |
| [new_entity_has_no_cell](../../../crates/entity/src/base_entity.rs#L163) | unit | Base Entity | 2026-03-03 | Asserts on `!entity.has_cell_entity()` |  |
| [new_entity_has_no_client](../../../crates/entity/src/base_entity.rs#L169) | unit | Base Entity | 2026-03-03 | Asserts on `entity.client_mailbox.is_none()` |  |
| [set_and_get_property](../../../crates/entity/src/base_entity.rs#L175) | unit | Base Entity | 2026-03-03 | Asserts equality on `entity.get_property("health")` |  |
| [get_missing_property_returns_none](../../../crates/entity/src/base_entity.rs#L185) | unit | Base Entity | 2026-03-03 | Asserts on `entity.get_property("nonexistent").is_none()` |  |
| [set_property_overwrites](../../../crates/entity/src/base_entity.rs#L191) | unit | Base Entity | 2026-03-03 | Asserts equality on `entity.get_property("level")` |  |
| [property_value_display](../../../crates/entity/src/base_entity.rs#L202) | unit | Base Entity | 2026-03-03 | Asserts equality on `format!("{}", PropertyValue::Int32(42))` |  |
| [new_entity_is_not_persistent](../../../crates/entity/src/base_entity.rs#L216) | unit | Base Entity | 2026-03-03 | Asserts on `!entity.is_persistent` |  |
| [has_cell_entity_with_mailbox](../../../crates/entity/src/base_entity.rs#L222) | unit | Base Entity | 2026-03-03 | Asserts on `entity.has_cell_entity()` |  |
| [new_entity_defaults](../../../crates/entity/src/cell_entity/tests.rs#L8) | unit | Cell Entity | 2026-05-02 | Asserts equality on `entity.entity_id` |  |
| [set_and_get_position](../../../crates/entity/src/cell_entity/tests.rs#L22) | unit | Cell Entity | 2026-05-02 | Asserts equality on `*entity.get_position()` |  |
| [add_and_remove_witness](../../../crates/entity/src/cell_entity/tests.rs#L30) | unit | Cell Entity | 2026-05-02 | Asserts equality on `entity.get_witnesses().len()` |  |
| [duplicate_witness_is_idempotent](../../../crates/entity/src/cell_entity/tests.rs#L43) | unit | Cell Entity | 2026-05-02 | Asserts equality on `entity.get_witnesses().len()` |  |
| [remove_absent_witness_is_noop](../../../crates/entity/src/cell_entity/tests.rs#L51) | unit | Cell Entity | 2026-05-02 | Asserts on `entity.get_witnesses().is_empty()` |  |
| [is_in_aoi_within_radius](../../../crates/entity/src/cell_entity/tests.rs#L58) | unit | Cell Entity | 2026-05-02 | Asserts on `entity.is_in_aoi(&nearby)` |  |
| [is_in_aoi_outside_radius](../../../crates/entity/src/cell_entity/tests.rs#L65) | unit | Cell Entity | 2026-05-02 | Asserts on `!entity.is_in_aoi(&far_away)` |  |
| [is_in_aoi_at_exact_boundary](../../../crates/entity/src/cell_entity/tests.rs#L72) | unit | Cell Entity | 2026-05-02 | Asserts on `entity.is_in_aoi(&boundary)` |  |
| [new_entity_ammo_defaults_empty](../../../crates/entity/src/cell_entity/tests.rs#L83) | unit | Cell Entity | 2026-05-02 | Asserts equality on `entity.active_ammo()` |  |
| [new_entity_ai_state_defaults_idle](../../../crates/entity/src/cell_entity/tests.rs#L95) | unit | Cell Entity | 2026-05-02 | Asserts equality on `entity.ai_state` |  |
| [new_entity_saved_missions_loaded_false](../../../crates/entity/src/cell_entity/tests.rs#L104) | unit | Cell Entity | 2026-05-02 | Asserts on `!entity.saved_missions_loaded` |  |
| [ai_state_equality](../../../crates/entity/src/cell_entity/tests.rs#L110) | unit | Cell Entity | 2026-05-02 | Asserts equality on `AiState::Idle` |  |
| [threat_list_operations](../../../crates/entity/src/cell_entity/tests.rs#L120) | unit | Cell Entity | 2026-05-02 | Asserts equality on `entity.threat_list.len()` |  |
| [spawn_position_stores_and_retrieves](../../../crates/entity/src/cell_entity/tests.rs#L144) | unit | Cell Entity | 2026-05-02 | Asserts on `entity.spawn_position.is_none()` |  |
| [active_ammo_helpers_with_no_item_return_zero](../../../crates/entity/src/cell_entity/tests.rs#L166) | unit | Cell Entity | 2026-05-02 | Asserts equality on `entity.active_ammo()` |  |
| [active_ammo_helpers_read_from_active_slot](../../../crates/entity/src/cell_entity/tests.rs#L174) | unit | Cell Entity | 2026-05-02 | Active ammo helpers read from active slot |  |
| [set_slot_ammo_clamps_and_marks_dirty](../../../crates/entity/src/cell_entity/tests.rs#L208) | unit | Cell Entity | 2026-05-02 | Asserts equality on `result` |  |
| [set_slot_ammo_unequipped_returns_none](../../../crates/entity/src/cell_entity/tests.rs#L230) | unit | Cell Entity | 2026-05-02 | Asserts equality on `result` |  |
| [refill_active_slot_fills_to_clip_size](../../../crates/entity/src/cell_entity/tests.rs#L238) | unit | Cell Entity | 2026-05-02 | Refill active slot fills to clip size |  |
| [refill_active_slot_unequipped_returns_none](../../../crates/entity/src/cell_entity/tests.rs#L260) | unit | Cell Entity | 2026-05-02 | Asserts equality on `result` |  |
| [known_constants_resolve_by_name](../../../crates/entity/src/interaction_flags.rs#L186) | unit | Interaction Flags | 2026-05-02 | Asserts equality on `mask_for_name("INT_MinigameLivewire")` |  |
| [unknown_name_returns_none](../../../crates/entity/src/interaction_flags.rs#L199) | unit | Interaction Flags | 2026-05-02 | Asserts equality on `mask_for_name("INT_Bogus")` |  |
| [bit_63_omitted](../../../crates/entity/src/interaction_flags.rs#L205) | unit | Interaction Flags | 2026-05-02 | Asserts equality on `mask_for_name("INT_MissionLoot")` |  |
| [high_bits_match_python_enum_values](../../../crates/entity/src/interaction_flags.rs#L212) | unit | Interaction Flags | 2026-05-02 | Asserts equality on `INT_NORMAL_LOOT` |  |
| [new_inventory_has_default_bags](../../../crates/entity/src/inventory.rs#L206) | unit | Inventory | 2026-03-06 | Asserts equality on `inv.bags.len()` |  |
| [empty_inventory_serialize_bag_info](../../../crates/entity/src/inventory.rs#L216) | unit | Inventory | 2026-03-06 | Asserts equality on `count as usize` |  |
| [empty_inventory_serialize_items](../../../crates/entity/src/inventory.rs#L226) | unit | Inventory | 2026-03-06 | Asserts equality on `count` |  |
| [serialize_cash](../../../crates/entity/src/inventory.rs#L235) | unit | Inventory | 2026-03-06 | Asserts equality on `i32::from_le_bytes([data[0], data[1], data[2], data[3]])` |  |
| [add_and_serialize_item](../../../crates/entity/src/inventory.rs#L245) | unit | Inventory | 2026-03-06 | Add and serialize item |  |
| [inv_item_serialize_with_ammo](../../../crates/entity/src/inventory.rs#L273) | unit | Inventory | 2026-03-06 | Inv item serialize with ammo |  |
| [bag_ids_match_python](../../../crates/entity/src/inventory.rs#L300) | unit | Inventory | 2026-03-06 | Asserts equality on `INV_MAIN` |  |
| [base_mailbox_new](../../../crates/entity/src/mailbox.rs#L122) | unit | Mailbox | 2026-03-03 | Asserts equality on `mb.entity_id` |  |
| [cell_mailbox_new](../../../crates/entity/src/mailbox.rs#L128) | unit | Mailbox | 2026-03-03 | Asserts equality on `mb.entity_id` |  |
| [client_mailbox_new](../../../crates/entity/src/mailbox.rs#L135) | unit | Mailbox | 2026-03-03 | Asserts equality on `mb.entity_id` |  |
| [new_manager_is_empty](../../../crates/entity/src/manager.rs#L119) | unit | Manager | 2026-03-03 | Asserts equality on `mgr.entity_count()` |  |
| [create_entity_assigns_sequential_ids](../../../crates/entity/src/manager.rs#L125) | unit | Manager | 2026-03-03 | Asserts equality on `id1` |  |
| [get_entity_returns_created_entity](../../../crates/entity/src/manager.rs#L135) | unit | Manager | 2026-03-03 | Asserts equality on `entity.entity_id` |  |
| [get_missing_entity_returns_none](../../../crates/entity/src/manager.rs#L144) | unit | Manager | 2026-03-03 | Asserts on `mgr.get_entity(EntityId(999)).is_none()` |  |
| [destroy_entity_removes_it](../../../crates/entity/src/manager.rs#L150) | unit | Manager | 2026-03-03 | Asserts equality on `mgr.entity_count()` |  |
| [destroy_entity_recycles_id](../../../crates/entity/src/manager.rs#L160) | unit | Manager | 2026-03-03 | Asserts equality on `id2` |  |
| [get_entity_mut_allows_modification](../../../crates/entity/src/manager.rs#L170) | unit | Manager | 2026-03-03 | Asserts on `entity.is_persistent` |  |
| [destroy_nonexistent_entity_is_safe](../../../crates/entity/src/manager.rs#L182) | unit | Manager | 2026-03-03 | Asserts equality on `mgr.entity_count()` |  |
| [allocate_id_starts_at_one](../../../crates/entity/src/manager.rs#L189) | unit | Manager | 2026-03-03 | Asserts equality on `id` |  |
| [default_is_same_as_new](../../../crates/entity/src/manager.rs#L196) | unit | Manager | 2026-03-03 | Asserts equality on `mgr.entity_count()` |  |
| [new_mission_is_active](../../../crates/entity/src/missions.rs#L246) | unit | Missions | 2026-03-06 | Asserts equality on `m.status` |  |
| [complete_mission](../../../crates/entity/src/missions.rs#L254) | unit | Missions | 2026-03-06 | Asserts equality on `m.status` |  |
| [fail_mission](../../../crates/entity/src/missions.rs#L271) | unit | Missions | 2026-03-06 | Asserts equality on `m.status` |  |
| [repeats_increments_across_multiple_completions](../../../crates/entity/src/missions.rs#L281) | unit | Missions | 2026-05-02 | Asserts equality on `m.repeats` |  |
| [new_mission_starts_at_zero_repeats](../../../crates/entity/src/missions.rs#L299) | unit | Missions | 2026-05-02 | Asserts equality on `m.repeats` |  |
| [complete_objective](../../../crates/entity/src/missions.rs#L306) | unit | Missions | 2026-03-06 | Asserts on `m.complete_objective(300)` |  |
| [complete_unknown_objective](../../../crates/entity/src/missions.rs#L314) | unit | Missions | 2026-03-06 | Asserts on `!m.complete_objective(999)` |  |
| [mission_manager_add_and_get](../../../crates/entity/src/missions.rs#L320) | unit | Missions | 2026-03-06 | Asserts equality on `mgr.count()` |  |
| [mission_manager_remove](../../../crates/entity/src/missions.rs#L329) | unit | Missions | 2026-03-06 | Asserts on `removed.is_some()` |  |
| [active_missions_filters_hidden](../../../crates/entity/src/missions.rs#L338) | unit | Missions | 2026-03-06 | Asserts equality on `active.len()` |  |
| [serialize_resend_format](../../../crates/entity/src/missions.rs#L355) | unit | Missions | 2026-03-06 | Asserts equality on `messages.len()` |  |
| [serialize_resend_empty_when_no_missions](../../../crates/entity/src/missions.rs#L406) | unit | Missions | 2026-03-06 | Asserts on `mgr.serialize_resend().is_empty()` |  |
| [serialize_resend_skips_completed](../../../crates/entity/src/missions.rs#L412) | unit | Missions | 2026-03-06 | Asserts on `mgr.serialize_resend().is_empty()` |  |
| [restore_active_mission_from_saved](../../../crates/entity/src/missions.rs#L425) | unit | Missions | 2026-03-17 | Restore active mission from saved |  |
| [restore_completed_mission_prevents_reacceptance](../../../crates/entity/src/missions.rs#L459) | unit | Missions | 2026-03-17 | Asserts equality on `mgr.count()` |  |
| [restore_multiple_missions_preserves_all](../../../crates/entity/src/missions.rs#L479) | unit | Missions | 2026-03-17 | Restore multiple missions preserves all |  |
| [all_missions_includes_every_status](../../../crates/entity/src/missions.rs#L524) | unit | Missions | 2026-03-17 | All missions includes every status |  |
| [player_controller_no_update_returns_none](../../../crates/entity/src/movement.rs#L165) | unit | Movement | 2026-03-03 | Asserts on `ctrl.update(0.016).is_none()` |  |
| [player_controller_apply_and_update](../../../crates/entity/src/movement.rs#L171) | unit | Movement | 2026-03-03 | Asserts equality on `pos` |  |
| [player_controller_update_consumed](../../../crates/entity/src/movement.rs#L179) | unit | Movement | 2026-03-03 | Asserts on `ctrl.update(0.016).is_none()` |  |
| [player_controller_never_completes](../../../crates/entity/src/movement.rs#L187) | unit | Movement | 2026-03-03 | Asserts on `!ctrl.is_complete()` |  |
| [waypoint_controller_moves_toward_target](../../../crates/entity/src/movement.rs#L193) | unit | Movement | 2026-03-03 | Asserts on `(pos.x - 5.0).abs() < 0.01` |  |
| [waypoint_controller_reaches_end](../../../crates/entity/src/movement.rs#L204) | unit | Movement | 2026-03-03 | Asserts on `ctrl.is_complete()` |  |
| [waypoint_controller_multiple_waypoints](../../../crates/entity/src/movement.rs#L214) | unit | Movement | 2026-03-03 | Asserts on `!ctrl.is_complete()` |  |
| [waypoint_controller_returns_none_when_complete](../../../crates/entity/src/movement.rs#L232) | unit | Movement | 2026-03-03 | Asserts on `ctrl.update(1.0).is_none()` |  |
| [waypoint_controller_empty_path_panics](../../../crates/entity/src/movement.rs#L241) | unit | Movement | 2026-03-03 | Waypoint controller empty path panics | smell: no_assert_or_question_mark |
| [waypoint_controller_position](../../../crates/entity/src/movement.rs#L246) | unit | Movement | 2026-03-03 | Asserts equality on `ctrl.position()` |  |
| [load_castle_cellblock_nav](../../../crates/entity/src/navigation.rs#L530) | unit | Navigation | 2026-03-22 | Asserts on `mesh.agent_height > 0.0` |  |
| [load_and_pathfind_castle_cellblock](../../../crates/entity/src/navigation.rs#L549) | unit | Navigation | 2026-03-24 | Asserts on `path_result.is_some()` |  |
| [load_and_raycast_castle_cellblock](../../../crates/entity/src/navigation.rs#L572) | unit | Navigation | 2026-03-24 | Load and raycast castle cellblock | smell: no_assert_or_question_mark |
| [load_and_height_query](../../../crates/entity/src/navigation.rs#L590) | unit | Navigation | 2026-03-24 | Asserts on `height.is_some()` |  |
| [new_flags_are_clean](../../../crates/entity/src/properties.rs#L105) | unit | Properties | 2026-03-03 | Asserts on `!flags.any_dirty()` |  |
| [mark_and_check_dirty](../../../crates/entity/src/properties.rs#L111) | unit | Properties | 2026-03-03 | Asserts on `flags.is_dirty(0)` |  |
| [mark_multiple_indices](../../../crates/entity/src/properties.rs#L120) | unit | Properties | 2026-03-03 | Asserts on `flags.is_dirty(0)` |  |
| [clear_resets_all_bits](../../../crates/entity/src/properties.rs#L135) | unit | Properties | 2026-03-03 | Asserts on `flags.any_dirty()` |  |
| [mark_dirty_is_idempotent](../../../crates/entity/src/properties.rs#L147) | unit | Properties | 2026-03-03 | Asserts on `flags.is_dirty(42)` |  |
| [index_out_of_range_panics](../../../crates/entity/src/properties.rs#L156) | unit | Properties | 2026-03-03 | Index out of range panics | smell: no_assert_or_question_mark |
| [default_is_clean](../../../crates/entity/src/properties.rs#L162) | unit | Properties | 2026-03-03 | Asserts on `!flags.any_dirty()` |  |
| [property_descriptor_construction](../../../crates/entity/src/properties.rs#L168) | unit | Properties | 2026-03-03 | Asserts equality on `desc.name` |  |
| [new_space_is_empty](../../../crates/entity/src/space.rs#L117) | unit | Space | 2026-03-03 | Asserts equality on `space.entity_count()` |  |
| [add_and_count_entities](../../../crates/entity/src/space.rs#L125) | unit | Space | 2026-03-03 | Asserts equality on `space.entity_count()` |  |
| [contains_entity](../../../crates/entity/src/space.rs#L133) | unit | Space | 2026-03-03 | Asserts on `space.contains_entity(EntityId(1))` |  |
| [remove_entity](../../../crates/entity/src/space.rs#L141) | unit | Space | 2026-03-03 | Asserts equality on `space.entity_count()` |  |
| [duplicate_add_is_idempotent](../../../crates/entity/src/space.rs#L153) | unit | Space | 2026-03-03 | Asserts equality on `space.entity_count()` |  |
| [remove_absent_entity_is_safe](../../../crates/entity/src/space.rs#L162) | unit | Space | 2026-03-03 | Remove absent entity is safe | smell: no_assert_or_question_mark |
| [get_entities_in_range](../../../crates/entity/src/space.rs#L168) | unit | Space | 2026-03-03 | Asserts on `nearby.contains(&EntityId(1))` |  |
| [stat_new_and_fields](../../../crates/entity/src/stats/tests.rs#L4) | unit | Stats | 2026-05-02 | Asserts equality on `s.min` |  |
| [stat_update_marks_dirty](../../../crates/entity/src/stats/tests.rs#L17) | unit | Stats | 2026-05-02 | Asserts on `s.dirty` |  |
| [stat_set_current_clamps](../../../crates/entity/src/stats/tests.rs#L27) | unit | Stats | 2026-05-02 | Asserts equality on `s.set_current(200)` |  |
| [stat_change_clamps](../../../crates/entity/src/stats/tests.rs#L36) | unit | Stats | 2026-05-02 | Asserts equality on `s.change(30)` |  |
| [stat_set_max_clamps_current](../../../crates/entity/src/stats/tests.rs#L47) | unit | Stats | 2026-05-02 | Asserts equality on `s.max` |  |
| [stat_set_min_clamps_current](../../../crates/entity/src/stats/tests.rs#L55) | unit | Stats | 2026-05-02 | Asserts equality on `s.min` |  |
| [stat_set_max_below_min_pulls_min_down](../../../crates/entity/src/stats/tests.rs#L63) | unit | Stats | 2026-05-02 | Asserts equality on `s.min` |  |
| [stat_set_min_above_max_pushes_max_up](../../../crates/entity/src/stats/tests.rs#L75) | unit | Stats | 2026-05-02 | Asserts equality on `s.min` |  |
| [stat_change_by_percent](../../../crates/entity/src/stats/tests.rs#L86) | unit | Stats | 2026-05-02 | Asserts equality on `delta` |  |
| [stat_change_by_max_percent](../../../crates/entity/src/stats/tests.rs#L94) | unit | Stats | 2026-05-02 | Asserts equality on `delta` |  |
| [stat_set_base_marks_base_dirty](../../../crates/entity/src/stats/tests.rs#L102) | unit | Stats | 2026-05-02 | Asserts on `s.base_dirty` |  |
| [stat_clear_dirty](../../../crates/entity/src/stats/tests.rs#L112) | unit | Stats | 2026-05-02 | Asserts on `!s.dirty` |  |
| [statlist_default_has_all_stats](../../../crates/entity/src/stats/tests.rs#L122) | unit | Stats | 2026-05-02 | Asserts equality on `health.cur` |  |
| [statlist_apply_archetype](../../../crates/entity/src/stats/tests.rs#L146) | unit | Stats | 2026-05-02 | Statlist apply archetype |  |
| [statlist_serialize_all_wire_format](../../../crates/entity/src/stats/tests.rs#L180) | unit | Stats | 2026-05-02 | Statlist serialize all wire format |  |
| [statlist_serialize_dirty_only](../../../crates/entity/src/stats/tests.rs#L237) | unit | Stats | 2026-05-02 | Asserts equality on `count` |  |
| [statlist_serialize_public](../../../crates/entity/src/stats/tests.rs#L255) | unit | Stats | 2026-05-02 | Asserts equality on `count as usize` |  |
| [statlist_clear_dirty](../../../crates/entity/src/stats/tests.rs#L263) | unit | Stats | 2026-05-02 | Asserts on `list.has_dirty()` |  |
| [statlist_serialize_base_values](../../../crates/entity/src/stats/tests.rs#L273) | unit | Stats | 2026-05-02 | Statlist serialize base values |  |
| [stat_ids_match_python_enums](../../../crates/entity/src/stats/tests.rs#L325) | unit | Stats | 2026-05-02 | Asserts equality on `COORDINATION` |  |
| [scale_for_level_increases_health_and_focus](../../../crates/entity/src/stats/tests.rs#L342) | unit | Stats | 2026-05-02 | Scale for level increases health and focus |  |
| [scale_for_level_1_is_base](../../../crates/entity/src/stats/tests.rs#L380) | unit | Stats | 2026-05-02 | Scale for level 1 is base |  |
| [scale_for_level_0_treated_as_1](../../../crates/entity/src/stats/tests.rs#L403) | unit | Stats | 2026-05-02 | Scale for level 0 treated as 1 |  |
| [cell_key_basic](../../../crates/entity/src/world_grid.rs#L147) | unit | World Grid | 2026-03-03 | Asserts equality on `g.cell_key(&Vector3::new(5.0, 0.0, 5.0))` |  |
| [cell_key_on_boundary](../../../crates/entity/src/world_grid.rs#L155) | unit | World Grid | 2026-03-03 | Asserts equality on `g.cell_key(&Vector3::new(10.0, 0.0, 10.0))` |  |
| [insert_and_query](../../../crates/entity/src/world_grid.rs#L162) | unit | World Grid | 2026-03-03 | Asserts on `nearby.contains(&EntityId(1))` |  |
| [remove_entity](../../../crates/entity/src/world_grid.rs#L176) | unit | World Grid | 2026-03-03 | Asserts on `!g.query_radius(&pos, 5.0).is_empty()` |  |
| [remove_cleans_up_empty_cells](../../../crates/entity/src/world_grid.rs#L187) | unit | World Grid | 2026-03-03 | Asserts on `g.cells.is_empty()` |  |
| [update_position_same_cell](../../../crates/entity/src/world_grid.rs#L196) | unit | World Grid | 2026-03-03 | Asserts on `results.contains(&EntityId(1))` |  |
| [update_position_different_cell](../../../crates/entity/src/world_grid.rs#L210) | unit | World Grid | 2026-03-03 | Asserts on `!old_results.contains(&EntityId(1))` |  |
| [query_empty_grid_returns_empty](../../../crates/entity/src/world_grid.rs#L228) | unit | World Grid | 2026-03-03 | Asserts on `results.is_empty()` |  |
| [negative_coordinates](../../../crates/entity/src/world_grid.rs#L235) | unit | World Grid | 2026-03-03 | Asserts on `results.contains(&EntityId(1))` |  |
| [zero_cell_size_panics](../../../crates/entity/src/world_grid.rs#L245) | unit | World Grid | 2026-03-03 | Zero cell size panics | smell: no_assert_or_question_mark |
| [query_results_are_deduplicated](../../../crates/entity/src/world_grid.rs#L250) | unit | World Grid | 2026-03-03 | Asserts equality on `results.iter().filter(\|&&id\| id == EntityId(1)).count()` |  |
