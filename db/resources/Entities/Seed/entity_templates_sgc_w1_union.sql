-- SGC_W1 Union world objects (US-11). CIMMERIA_RECONSTRUCTION: copies of the
-- seeded world-object templates 36 (crate) and 37 (gate control), plus the
-- Failsafe Device mesh US-02 read from the cooked SGC_W1 map.
SET search_path = resources, pg_catalog;

-- CIMMERIA_RECONSTRUCTION: Failsafe Device, copy of 37; mesh and name_id from
-- docs/analysis/union-sgc-continuation/meshes.md (name_id 10726).
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed) VALUES (5800, 'Em-Props.EM-SGC_Failsafe00', 'GLB_Components.WorldObject_Small', NULL, 4, 0, NULL, 1, 0, 1, 10726, NULL, NULL, NULL, 'SGC Failsafe Device', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL);

-- 5801 reserved (F12: Carter hands over her notes herself).

-- CIMMERIA_RECONSTRUCTION: pressable crate, copy of 36 with the table's flags
-- and faction; name_id 8368 (moniker text is empty in the seed).
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed) VALUES (5802, 'EM_Earth_Military.EM-Crate_Wooden01', 'GLB_Components.WorldObject_Small', NULL, 4, 0, NULL, 1, 0, 1, 8368, NULL, NULL, NULL, 'SGC Ordinance Crate', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL);

-- CIMMERIA_RECONSTRUCTION: Carter's virus terminal, copy of 37; mesh as the
-- seeded ViewScreen02 templates, name_id 7550 "Terminal" (meshes.md).
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed) VALUES (5803, 'Em-Props.EM-ViewScreen02', 'GLB_Components.WorldObject_Small', NULL, 4, 0, NULL, 1, 0, 1, 7550, NULL, NULL, NULL, 'SGC Carter Terminal', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL);

SELECT pg_catalog.setval('entity_templates_template_id_seq', GREATEST((SELECT MAX(template_id) FROM entity_templates), (SELECT last_value FROM entity_templates_template_id_seq), 5803), true);
