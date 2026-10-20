--
-- NEW CONTENT (minigames campaign, MG-74, D-MG23): Debug Area minigame terminals, templates 1600-1609
--
-- One template per ported game plus one shell, so a GM can launch each game by
-- hand from world 1300 `DebugArea` (spawnlist_debug_area_minigames.sql, spawns
-- 14100-14105). Each copies the Debug Hub Livewire terminal (template 303) value
-- for value, changing only the id, the name and display_name. Every row keeps
-- interaction_type 256 (INT_MinigameLivewire), the minigame cursor bit with a
-- Confirmed click path. The chains that answer them are
-- debug_area_minigames_chains.sql (24050-24055).
--

SET search_path = resources, pg_catalog;

-- 1600 CrystalGame terminal.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, display_name) VALUES (1600, 'Em-Props.EM-ViewScreen02', 'GLB_Components.WorldObject_WallTerminal', NULL, 4, 256, NULL, 1, 0, 1, 7550, NULL, NULL, NULL, 'Debug Area - Minigame Terminal CrystalGame', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 'Minigame: CrystalGame');

-- 1601 Alignment terminal.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, display_name) VALUES (1601, 'Em-Props.EM-ViewScreen02', 'GLB_Components.WorldObject_WallTerminal', NULL, 4, 256, NULL, 1, 0, 1, 7550, NULL, NULL, NULL, 'Debug Area - Minigame Terminal Alignment', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 'Minigame: Alignment');

-- 1602 GoauldCrystals terminal.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, display_name) VALUES (1602, 'Em-Props.EM-ViewScreen02', 'GLB_Components.WorldObject_WallTerminal', NULL, 4, 256, NULL, 1, 0, 1, 7550, NULL, NULL, NULL, 'Debug Area - Minigame Terminal GoauldCrystals', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 'Minigame: GoauldCrystals');

-- 1603 ConverseBasicHumanoid terminal.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, display_name) VALUES (1603, 'Em-Props.EM-ViewScreen02', 'GLB_Components.WorldObject_WallTerminal', NULL, 4, 256, NULL, 1, 0, 1, 7550, NULL, NULL, NULL, 'Debug Area - Minigame Terminal ConverseBasicHumanoid', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 'Minigame: ConverseBasicHumanoid');

-- 1604 Livewire terminal.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, display_name) VALUES (1604, 'Em-Props.EM-ViewScreen02', 'GLB_Components.WorldObject_WallTerminal', NULL, 4, 256, NULL, 1, 0, 1, 7550, NULL, NULL, NULL, 'Debug Area - Minigame Terminal Livewire', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 'Minigame: Livewire');

-- 1605 Hack terminal (the shell).
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, display_name) VALUES (1605, 'Em-Props.EM-ViewScreen02', 'GLB_Components.WorldObject_WallTerminal', NULL, 4, 256, NULL, 1, 0, 1, 7550, NULL, NULL, NULL, 'Debug Area - Minigame Terminal Hack (shell)', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 'Minigame: Hack (shell)');

-- These rows load after entity_templates.sql's own sequence footer, so they
-- raise the sequence themselves. Floor 1609 is the top of this block (1600-1609).
SELECT pg_catalog.setval('entity_templates_template_id_seq', GREATEST((SELECT MAX(template_id) FROM entity_templates), (SELECT last_value FROM entity_templates_template_id_seq), 1609), true);
