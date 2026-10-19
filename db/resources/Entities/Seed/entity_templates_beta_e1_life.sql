-- Union Beta E1 life templates (UB-40): the background figures that make the
-- base look inhabited. Ids 2308 and 2309 sit inside the Beta E1 block
-- 2300-2349 that entity_templates_beta_e1.sql reserves, and that file's footer
-- already keeps the template sequence past the block, so this file has none.
-- Plan: docs/analysis/union-zone-planning/beta-packets/UB-40-52.md.
--
-- Not interactive: interaction_type 0, flags 0, speaker NULL, and no chain or
-- bind names either id (the guards are in
-- crates/cell-world/src/cell/spawner_tests/beta_e1/life.rs). A life NPC with no
-- bind cannot be clicked, so it never offers a dead dialog.
--
-- Evidence labels: RESTORED_SOURCE is the client's moniker and its text;
-- CIMMERIA_RECONSTRUCTION is everything else.

SET search_path = resources, pg_catalog;

-- 2308 Beta background extra, the SGC personnel packing up the base.
-- name_id 7450 'DN_npc_Int_BetaE1_Admin_SGCPersonnel': RESTORED_SOURCE, "SGC Personnel".
-- look: CIMMERIA_RECONSTRUCTION, copied from template 302 (Debug Hub): body set,
--   components, colours and skin tint. Row shape copied from 2301.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (2308, NULL, 'BS_HumanMale.BS_HumanMale', '{AR_H_SGC.AR_HM_SB1_SH100,AR_H_SGC.AR_HM_SL1_SL100SB100,AR_H_SGC.AR_HM_ST1_ST103,BS_HumanMale.BS_HM_Hands_00,BS_HumanMale.BS_HM_Head_04}', 0, 0, 570, 50, 0, 1, 7450, NULL, NULL, NULL, 'Beta E1 - SGC Personnel', 'mob', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);

-- 2309 Beta non-combatant Jaffa at the Nine Peaks camp (the camp's figure, no staff).
-- name_id 7409 'DN_npc_Esc_BetaE1_RasNorsCamp_JaffaNonCombatant': RESTORED_SOURCE, "Jaffa Non Combatant".
-- look: CIMMERIA_RECONSTRUCTION, copied from 2306 (Ras'nor), without the staff.
--   Row copied from 2306 with the id, name_id, template_name and speaker changed.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (2309, NULL, 'BS_JaffaMale.BS_JaffaMale', '{AR_J_Standard.AR_JM_SB1_SH100,AR_J_Standard.AR_JM_SG1_SG100,AR_J_Standard.AR_JM_SH1_SH100,AR_J_Standard.AR_JM_SL1_SS100,AR_J_Standard.AR_JM_ST1_ST100SS100SP100,BS_JaffaMale.BS_JM_Boots_00,BS_JaffaMale.BS_JM_Hands_00,BS_JaffaMale.BS_JM_Head_00,BS_JaffaMale.BS_JM_Legs_00,BS_JaffaMale.BS_JM_Torso_00}', 0, 0, 570, 50, 0, 1, 7409, NULL, NULL, NULL, 'Beta E1 - Jaffa Non Combatant', 'mob', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);

-- Keep default-id inserts past the Beta E1 block 2300-2349 (its footer is in entity_templates_beta_e1.sql).
