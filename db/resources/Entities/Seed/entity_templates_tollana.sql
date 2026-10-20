-- Tollana (World 19) cast templates, campaign tollana-continuation W1 (T-02a).
--
-- CIMMERIA_RECONSTRUCTION. Evidence and decisions:
-- docs/analysis/tollana-continuation/w1-packets.md (D-TW6, D-TW7) and
-- docs/analysis/tollana-continuation/evidence/T-01-offline-placement.md.
--   420 Tollana_Copplemann: identity copied from template 48 (CmdCenter
--       Copplemann). 48 is NOT reused: Harset bindings 6503/6504 key on its tag
--       and slot. event_set 570 is the universal NPC event set, kept as on 48.
--   421 Tollana food-duplication unit: one shared prop template for spawns
--       19002-19004, modelled on Dakara template 445 (flags 4,
--       INT_MissionWorldObject 1073741824, class being). Mesh ViewScreen02 is a
--       placeholder; the swap to the HT food-unit mesh is a UAT-time change.

SET search_path = resources, pg_catalog;

INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES
(420, NULL, 'BS_HumanFemale.BS_HumanFemale', '{AR_H_Clothing00.AR_HM_PL1_PL100AB100,AR_H_Clothing00.AR_HM_PT1_PT100,BS_HumanFemale.BS_HF_Base_Boots00_00,BS_HumanFemale.BS_HF_Base_Hands00_00,BS_HumanFemale.BS_HF_Base_Legs00_00,BS_HumanFemale.BS_HF_Base_Torso00_00,BS_HumanFemale.BS_HF_Boots_00,BS_HumanFemale.BS_HF_Head_00,NPC_Human.NPC_HF_Coppl_Hair_BC,NPC_Human.NPC_HF_Coppl_Head_BC,AR_Global.Prisoner_Boots}', 0, 0, 570, 50, 0, 1, 7035, NULL, NULL, NULL, 'Tollana - Capt. Copplemann', 'mob', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, -52773120, 21, '{}', NULL, 968, true, NULL, NULL, NULL, 300, NULL),
(421, 'Em-Props.EM-ViewScreen02', 'GLB_Components.WorldObject_Small', NULL, 4, 1073741824, NULL, 1, 0, 1, 7558, NULL, NULL, NULL, 'Tollana - Food Duplication Unit', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);
