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

-- [UO-10] banker 63810, registrars 63811 (Team) and 63830 (Command) (provenance set by UO-10's block)
-- CIMMERIA_RECONSTRUCTION clones of the debug-hub banker 370 and registrars 330/331 (D-UO14). Monikers 29462 and 29068 are RESTORED_SOURCE (`DN_npc_OmegaSite_Banker`, `DN_npc_reg_OmegaSite_TeamCommandRegistrar`). One registrar per type, because a template with both sets has no type.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, respawn_secs, vault_scope) VALUES (63810, NULL, 'BS_HumanFemale.BS_HumanFemale', '{AR_H_SGC.AR_HM_SL1_SL100,AR_H_SGC.AR_HM_ST1_ST100,BS_HumanFemale.BS_HF_Base_Boots00_00,BS_HumanFemale.BS_HF_Base_Hands00_00,BS_HumanFemale.BS_HF_Base_Legs00_00,BS_HumanFemale.BS_HF_Base_Torso00_00,BS_HumanFemale.BS_HF_Head_00}', 0, 2, 570, 1, 0, 1, 29462, NULL, NULL, NULL, 'Union Omega - Storage Officer', 'mob', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, -256076032, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 'personal');
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, respawn_secs) VALUES (63811, NULL, 'BS_HumanMale.BS_HumanMale', '{AR_H_SGC.AR_HM_SB1_SH100,AR_H_SGC.AR_HM_SL1_SL100SB100,AR_H_SGC.AR_HM_ST1_ST103,BS_HumanMale.BS_HM_Hands_00,BS_HumanMale.BS_HM_Head_04}', 0, 64, 570, 1, 0, 1, 29068, NULL, NULL, NULL, 'Union Omega - Team Registrar', 'mob', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{7447}', NULL, NULL, true, NULL, NULL, NULL);
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, respawn_secs) VALUES (63830, NULL, 'BS_HumanMale.BS_HumanMale', '{AR_H_Ablative.AR_HM_AT3_AT300,AR_H_Ballistic00.AR_HM_BG3_BG300,NPC_Human.NPC_HM_Marsh_Head_BC,AR_H_Ablative.AR_HM_AL3_AL300AH300}', 0, 64, 570, 1, 0, 1, 29068, NULL, NULL, NULL, 'Union Omega - Command Registrar', 'mob', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{7448}', NULL, NULL, true, NULL, NULL, NULL);

-- [UO-11] Tau'ri vendors 63812-63816 (provenance set by UO-11's block)

-- [UO-12] Jaffa vendors 63817-63819 (provenance set by UO-12's block)

-- [UO-14] ambient 63860-63899 (CIMMERIA_AUTHORING, droppable)

--
-- The rows above load after entity_templates.sql's own sequence footer, so
-- they raise the sequence themselves. Floor 63999 is the top of Omega's block,
-- so a `.savespawn` or GM-made template never takes a reserved id.
SELECT pg_catalog.setval('entity_templates_template_id_seq', GREATEST((SELECT MAX(template_id) FROM entity_templates), (SELECT last_value FROM entity_templates_template_id_seq), 63999), true);
