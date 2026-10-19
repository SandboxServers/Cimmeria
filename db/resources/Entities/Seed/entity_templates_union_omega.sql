--
-- Union Omega (worlds 18 `Omega_Site` and 80 `Omega_Site_CmdCenter`): entity
-- templates. Campaign: docs/analysis/union-zone-planning/omega-work-packets.md (packets UO-01 to UO-14).
--
-- Id block 63800-63999 (Omega's reservation, D-UO1). Each packet writes only
-- below its own marker. Provenance: each block states its own tag
-- (RESTORED_SOURCE, CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING).
--

SET search_path = resources, pg_catalog;

-- [UO-05] Command Center cast templates 63800-63803 (CIMMERIA_RECONSTRUCTION; clones of 51, 59, 30, 64)
-- UO-05 (D-UO5): CIMMERIA_RECONSTRUCTION clones of the stock Command Center cast, source ids 51, 59, 30 and 64 (source lines 127, 123, 41 and 111).
-- Only template_id and template_name change. name_id, speaker_id and every other column stay as in the stock row, so the moniker and voice are RESTORED_SOURCE.
-- The stock rows stay untouched; sibling packets place them elsewhere.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed) VALUES (63800, NULL, 'BS_HumanMale.BS_HumanMale', '{ACC_HumanMale.ACC_HM_Accessory_09,AR_H_Ballistic00.AR_HM_BT1_BT101,AR_H_SGC.AR_HM_SB1_SH100,AR_H_SGC.AR_HM_SL1_SL100,BS_HumanMale.BS_HM_Boots_00,BS_HumanMale.BS_HM_Hands_00,BS_HumanMale.BS_HM_Legs_00,BS_HumanMale.BS_HM_Torso_00,NPC_Human.NPC_HM_DanielJackson_Head_BC,NPC_Human.NPC_HM_DJGlasses_BC}', 0, 0, 570, 50, 0, 1, 7907, NULL, NULL, NULL, 'Union Omega - Daniel Jackson', 'mob', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, -340158464, NULL, '{}', NULL, 431, true, NULL, NULL);
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed) VALUES (63801, NULL, 'BS_JaffaMale.BS_JaffaMale', '{NPC_Jaffa.AR_JM_Bratac_Head_BC,AR_JF_Clothing00.AR_JM_CH1_CH100,AR_JF_Clothing00.AR_JM_CT1_CT100,AR_JF_Clothing00.AR_JM_JB1_JH100,AR_JF_Clothing00.AR_JM_JL1_JL100,BS_JaffaMale.BS_JM_Base_Boots00_00,BS_JaffaMale.BS_JM_Base_Hands00_00,BS_JaffaMale.BS_JM_Base_Legs00_00,BS_JaffaMale.BS_JM_Base_Torso00_00,BS_JaffaMale.BS_JM_Base_Torso00_00}', 0, 0, 570, 50, 0, 1, 7429, NULL, NULL, NULL, 'Union Omega - Bra''tac', 'mob', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, 2323, true, NULL, NULL);
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed) VALUES (63802, NULL, 'BS_JaffaMale.BS_JaffaMale', '{AR_H_SGC.AR_HM_SB1_SH100,AR_H_SGC.AR_HM_SL1_SL100SB100,AR_H_SGC.AR_HM_ST1_ST103,NPC_Jaffa.AR_JM_Tealc_Hands_BC,NPC_Jaffa.AR_JM_Tealc_Head_BC,BS_JaffaMale.BS_JM_Base_Boots00_00,BS_JaffaMale.BS_JM_Base_Legs00_00,BS_JaffaMale.BS_JM_Base_Torso00_00}', 0, 0, 570, 1, 0, 1, 7908, NULL, NULL, NULL, 'Union Omega - Teal''c', 'mob', NULL, NULL, NULL, NULL, NULL, 'Bullet_Default', NULL, 0, 0, 0, NULL, '{}', NULL, 2316, true, NULL, NULL);
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed) VALUES (63803, NULL, 'BS_Asgard.BS_Asgard', '{BS_Asgard.BS_AS_Base_Boots00_00,BS_Asgard.BS_AS_Base_Hands00_00,BS_Asgard.BS_AS_Base_Head00_00,BS_Asgard.BS_AS_Base_Legs00_00,BS_Asgard.BS_AS_Base_Torso00_00}', 0, 0, 570, 50, 0, 1, 7435, NULL, NULL, NULL, 'Union Omega - Thor', 'mob', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, 777, true, NULL, NULL);

-- [UO-10] banker 63810, registrars 63811 (Team) and 63830 (Command) (provenance set by UO-10's block)

-- [UO-11] Tau'ri vendors 63812-63816 (provenance set by UO-11's block)

-- [UO-12] Jaffa vendors 63817-63819 (provenance set by UO-12's block)

-- [UO-14] ambient 63860-63899 (CIMMERIA_AUTHORING, droppable)

--
-- The rows above load after entity_templates.sql's own sequence footer, so
-- they raise the sequence themselves. Floor 63999 is the top of Omega's block,
-- so a `.savespawn` or GM-made template never takes a reserved id.
SELECT pg_catalog.setval('entity_templates_template_id_seq', GREATEST((SELECT MAX(template_id) FROM entity_templates), (SELECT last_value FROM entity_templates_template_id_seq), 63999), true);
