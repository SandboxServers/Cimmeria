--
-- NEW CONTENT (Debug Area, DA-02): templates for the services plaza (Z2) and
-- the dummies range (Z3) of world 1300 `DebugArea`. docs/content/debug-area.md
-- describes each NPC; docs/analysis/debug-area/README.md is the campaign plan.
--
-- Id block 1300-1329 (DA-02). Only the NPCs that differ from a stasis-room hub
-- template live here; every other plaza service spawns the hub's own template
-- (300-305, 310-314, 330, 331, 360, 370-372, 390), so the two stay the same NPC.
--
-- Names are monikers the client PAK ships: a new texts.sql id never renders.
--

SET search_path = resources, pg_catalog;

-- 1300 Ability granter. A right-click fires chain 13000 (debug_area_plaza_chains.sql),
--   whose `gm_ability_bulk` action (change grant_all) gives a GM every ability
--   of their archetype tree, capstones included, and clears their cooldowns. A
--   non-GM gets a refusal line and nothing else. INT_Trainer (128) gives the
--   trainer cursor; there is no trainer list, so the trainer window never
--   claims the click before the chain.
--   Body: General Hammond's (template 29). Faction 1, level 1, no ability set.
-- moniker 21666 `DN_Ds_Trn_JimTestTrainer_TT2122` ('Train Testing Abilities'),
--   the original developers' own test trainer.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, training_dummy) VALUES (1300, NULL, 'BS_HumanMale.BS_HumanMale', '{BS_HumanMale.BS_HM_Base_Hands00_00,NPC_Human.NPC_HM_Hammond_Boots_BC,NPC_Human.NPC_HM_Hammond_Head_BC,NPC_Human.NPC_HM_Hammond_Pants_BC,NPC_Human.NPC_HM_Hammond_Torso_BC}', 0, 128, 570, 1, 0, 1, 21666, NULL, NULL, NULL, 'Debug Area - Ability Granter', 'mob', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, -256076032, NULL, '{}', NULL, NULL, true, NULL, NULL, false, false);

-- 1301 Ability reset. Chain 13001's `gm_ability_bulk` (change reset) puts a
--   GM back to their archetype's character-creation starters, refunds the
--   tree points and clears the cooldowns: the clean slate a repeatable ability
--   UAT starts from. GM-gated like the granter. INT_Trainer (128), no list.
--   Body: Sam Carter's (template 33).
-- moniker 22555 `DN_DS_Trn_JayTestTrainer_TT3194` ('Jay Test Abilities'),
--   the developers' second test trainer.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, training_dummy) VALUES (1301, NULL, 'BS_HumanFemale.BS_HumanFemale', '{AR_H_SGC.AR_HM_SL1_SL100,AR_H_SGC.AR_HM_ST1_ST100,BS_HumanFemale.BS_HF_Base_Boots00_00,BS_HumanFemale.BS_HF_Base_Hands00_00,BS_HumanFemale.BS_HF_Base_Legs00_00,BS_HumanFemale.BS_HF_Base_Torso00_00,BS_HumanFemale.BS_HF_Boots_00,NPC_Human.NPC_HF_SamCarter_Head_BC}', 0, 128, 570, 1, 0, 1, 22555, NULL, NULL, NULL, 'Debug Area - Ability Reset', 'mob', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, -52773120, NULL, '{}', NULL, NULL, true, NULL, NULL, false, false);

-- 1302 Munitions vendor: the special-ammo, consumables and weapons source the
--   stasis hub never had. Buy list 1300 (item_lists_debug_area_plaza.sql) sells
--   all fifteen special-ammo reserve stacks, Health Slappacks and one pistol,
--   SMG and dart gun, at 1 naquadah each. Sell, repair and recharge use list 2,
--   as the hub vendor (300) does. INT_VendorWeapons | INT_VendorConsumables
--   (16384 | 32768) routes the click to the store.
--   Body: the Cellblock guards' SGC uniform (template 15) carrying an SMG.
-- moniker 27264 `DN_npc_ven_OmegaSite_Consumables` ('Consumables').
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, training_dummy) VALUES (1302, NULL, 'BS_HumanMale.BS_HumanMale', '{AR_H_SGC.AR_HM_SB1_SH100,AR_H_SGC.AR_HM_SH1_SH100,AR_H_SGC.AR_HM_SL1_SL101SB100SH100,AR_H_SGC.AR_HM_ST1_ST103,BS_HumanMale.BS_HM_Hands_00,BS_HumanMale.BS_HM_Head_01,WP-Human.WP_SMG_1A}', 0, 49152, 570, 1, 0, 1, 27264, NULL, NULL, NULL, 'Debug Area - Munitions Vendor', 'mob', 1300, 2, 2, 2, NULL, NULL, NULL, 0, 0, -256076032, NULL, '{}', NULL, NULL, true, NULL, NULL, false, false);

-- 1310-1313 Hostile training dummies, levels 1, 10, 25 and 50 (D-DA7).
--   `training_dummy = true`: the spawn puts the TrainingDummy mark on them, so
--   they get no AI turn and never fire back, chase or leash however much they
--   are shot, and gives them 1,000,000 Health. Faction 10, so a right-click
--   attacks and every player attack lands. A fight goes quiet 10 s after the
--   last hit and the shooter leaves combat. No ability set and no loot table.
--   respawn_secs 30 only matters if one is somehow killed.
--   Body: the SGC Jaffa of template 34, the `.dummy` default.
-- moniker 8168 `DN_MsMb_MenFa_Jaffa_Uni_11` ('Jaffa'), the client's name for
--   template 34's body.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, training_dummy) VALUES (1310, NULL, 'BS_JaffaMale.BS_JaffaMale', '{BS_JaffaMale.BS_JM_Base_Hands00_00,BS_JaffaMale.BS_JM_Base_Head00_00,BS_JaffaMale.BS_JM_Base_Boots00_00,BS_JaffaMale.BS_JM_Base_Legs00_00,BS_JaffaMale.BS_JM_Base_Torso00_00,AR_J_Standard.AR_JM_SB1_SH100,AR_J_Standard.AR_JM_SL1_SS100,AR_J_Standard.AR_JM_ST1_ST100SS100,AR_J_Standard.AR_JM_SH1_SH100}', 0, 0, 570, 1, 0, 10, 8168, NULL, NULL, NULL, 'Debug Area - Training Dummy L1', 'mob', NULL, NULL, NULL, NULL, NULL, 'Bullet_Default', NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, false, 30, true);
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, training_dummy) VALUES (1311, NULL, 'BS_JaffaMale.BS_JaffaMale', '{BS_JaffaMale.BS_JM_Base_Hands00_00,BS_JaffaMale.BS_JM_Base_Head00_00,BS_JaffaMale.BS_JM_Base_Boots00_00,BS_JaffaMale.BS_JM_Base_Legs00_00,BS_JaffaMale.BS_JM_Base_Torso00_00,AR_J_Standard.AR_JM_SB1_SH100,AR_J_Standard.AR_JM_SL1_SS100,AR_J_Standard.AR_JM_ST1_ST100SS100,AR_J_Standard.AR_JM_SH1_SH100}', 0, 0, 570, 10, 0, 10, 8168, NULL, NULL, NULL, 'Debug Area - Training Dummy L10', 'mob', NULL, NULL, NULL, NULL, NULL, 'Bullet_Default', NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, false, 30, true);
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, training_dummy) VALUES (1312, NULL, 'BS_JaffaMale.BS_JaffaMale', '{BS_JaffaMale.BS_JM_Base_Hands00_00,BS_JaffaMale.BS_JM_Base_Head00_00,BS_JaffaMale.BS_JM_Base_Boots00_00,BS_JaffaMale.BS_JM_Base_Legs00_00,BS_JaffaMale.BS_JM_Base_Torso00_00,AR_J_Standard.AR_JM_SB1_SH100,AR_J_Standard.AR_JM_SL1_SS100,AR_J_Standard.AR_JM_ST1_ST100SS100,AR_J_Standard.AR_JM_SH1_SH100}', 0, 0, 570, 25, 0, 10, 8168, NULL, NULL, NULL, 'Debug Area - Training Dummy L25', 'mob', NULL, NULL, NULL, NULL, NULL, 'Bullet_Default', NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, false, 30, true);
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, training_dummy) VALUES (1313, NULL, 'BS_JaffaMale.BS_JaffaMale', '{BS_JaffaMale.BS_JM_Base_Hands00_00,BS_JaffaMale.BS_JM_Base_Head00_00,BS_JaffaMale.BS_JM_Base_Boots00_00,BS_JaffaMale.BS_JM_Base_Legs00_00,BS_JaffaMale.BS_JM_Base_Torso00_00,AR_J_Standard.AR_JM_SB1_SH100,AR_J_Standard.AR_JM_SL1_SS100,AR_J_Standard.AR_JM_ST1_ST100SS100,AR_J_Standard.AR_JM_SH1_SH100}', 0, 0, 570, 50, 0, 10, 8168, NULL, NULL, NULL, 'Debug Area - Training Dummy L50', 'mob', NULL, NULL, NULL, NULL, NULL, 'Bullet_Default', NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, false, 30, true);

-- 1314 Friendly training dummy: the heal and buff target (D-DA7). Faction 9
--   (Friendly_Ambient, the `.dummy friendly` faction): players cannot attack
--   it and no NPC targets it. As a training dummy it is an ally for beneficial
--   casts, so a heal aimed at it lands on it, and it starts at half of its
--   1,000,000 Health so every heal shows its size. Level 10.
--   Body: the Cellblock guards' SGC uniform (template 15), unarmed.
-- moniker 7882 `DN_NPC_Int_SGC2_AnatsTankRoom_InjuredSGCGuard` ('Injured SGC Guard').
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, training_dummy) VALUES (1314, NULL, 'BS_HumanMale.BS_HumanMale', '{AR_H_SGC.AR_HM_SB1_SH100,AR_H_SGC.AR_HM_SH1_SH100,AR_H_SGC.AR_HM_SL1_SL101SB100SH100,AR_H_SGC.AR_HM_ST1_ST103,BS_HumanMale.BS_HM_Hands_00,BS_HumanMale.BS_HM_Head_01}', 0, 0, 570, 10, 0, 9, 7882, NULL, NULL, NULL, 'Debug Area - Friendly Training Dummy', 'mob', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, -256076032, NULL, '{}', NULL, NULL, true, NULL, NULL, false, 30, true);

-- These rows load after entity_templates.sql's own sequence footer, so they
-- raise the sequence themselves. Floor 1399 is the top of the whole Debug
-- Area reservation (DA-02..04, templates 1300-1399).
SELECT pg_catalog.setval('entity_templates_template_id_seq', GREATEST((SELECT MAX(template_id) FROM entity_templates), (SELECT last_value FROM entity_templates_template_id_seq), 1399), true);
