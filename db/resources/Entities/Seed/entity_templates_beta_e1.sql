--
-- Beta E1 (UB-02a): the cast and the object templates of the Union Beta
-- campaign, ids 2300-2335. Templates only: no spawn row, no position and no
-- hostile. Plan: docs/analysis/union-zone-planning/beta-packets/UB-02a.md.
--
-- Id block 2300-2349. 2303 (Dr. Pierce, 7905), 2327 (Accelerant, 7381) and
-- 2336 are RESERVED and NOT created: they are for UB-52's twins 383 and 384
-- (D-UB7 ruled the twins out of the first ship, so these ids stay empty).
-- The footer at the end keeps the entity template sequence past the block.
--
-- Beta has no NPCs of its own: chains bind dialogs per player with `slot`
-- set to the template id (F14). Every template here is named by the client's
-- own Beta display-name string, so no other campaign's bind lights up on it.
--
-- Evidence labels, one per column group on every row:
--
-- * RESTORED_SOURCE: the `name_id` moniker and its text (the client's own
--   strings in `texts.sql`), and the `speaker_id` where the client's
--   speakers table names one.
-- * CIMMERIA_RECONSTRUCTION: everything else. The look of each row is copied
--   whole (body set, components, colours, skin tint, or static mesh and body
--   set for an object) from the template named in its comment, so the shape
--   of each actor is one the repo already renders. Each mesh is
--   CIMMERIA_RECONSTRUCTION until UB-90 row UB-U2c checks it in the lab.
--
-- Names. A row whose client string is empty keeps its `name_id` and sets
-- `display_name`, the literal nameplate the server sends in place of the
-- name id. A row whose string has text sets no `display_name`.
--
-- Every row: faction 1, alignment 0, respawn_secs 300, no loot table, no
-- vendor, trainer or weapon list, no patrol path, no wander radius and
-- `use_cover` NULL. NPCs: class 'mob', event set 570, interaction_type 0,
-- flags 0. Objects: class 'being', flags 4 (ENTITYFLAG_DoNotDrop),
-- interaction_type 1073741824 (INT_MissionWorldObject), event set NULL,
-- speaker NULL, level 1.
--
-- Guards: crates/cell-world/src/cell/spawner_tests/beta_e1/.
--

SET search_path = resources, pg_catalog;

--
-- The cast. Level 50 on the named characters is the level the Dakara and
-- Harset named NPCs carry; the client records no level for any Beta name.
--

-- 2300 Lt. Col. Carter, the Beta E1 administrator.
-- name_id 7366 'DN_npc_MG_BetaE1_Admin_Carter': RESTORED_SOURCE, "Lt. Col. Carter".
-- speaker 708: RESTORED_SOURCE, the speaker of the Beta E1 admin dialogs.
-- look: CIMMERIA_RECONSTRUCTION, copied from 33 Sam Carter (body, components, skin_tint).
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (2300, NULL, 'BS_HumanFemale.BS_HumanFemale', '{AR_H_SGC.AR_HM_SL1_SL100,AR_H_SGC.AR_HM_ST1_ST100,BS_HumanFemale.BS_HF_Base_Boots00_00,BS_HumanFemale.BS_HF_Base_Hands00_00,BS_HumanFemale.BS_HF_Base_Legs00_00,BS_HumanFemale.BS_HF_Base_Torso00_00,BS_HumanFemale.BS_HF_Boots_00,NPC_Human.NPC_HF_SamCarter_Head_BC}', 0, 0, 570, 50, 0, 1, 7366, NULL, NULL, NULL, 'Beta E1 - Lt. Col. Carter', 'mob', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, -52773120, NULL, '{}', NULL, 708, true, NULL, NULL, NULL, 300, NULL);

-- 2301 Dr. Felger, the Beta E1 scientist.
-- name_id 7368 'DN_npc_MG_BetaE1_Admin_Felger': RESTORED_SOURCE, text empty.
-- display_name: RESTORED_SOURCE, the speaker 710 name, "Dr. Felger".
-- speaker 710: RESTORED_SOURCE.
-- look: CIMMERIA_RECONSTRUCTION, copied from 31 Airman (body, components, skin_tint).
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (2301, NULL, 'BS_HumanMale.BS_HumanMale', '{ACC_HumanMale.ACC_HM_Accessory_09,AR_H_Ballistic00.AR_HM_BT1_BT101,AR_H_SGC.AR_HM_SB1_SH100,AR_H_SGC.AR_HM_SL1_SL100,BS_HumanMale.BS_HM_Boots_00,BS_HumanMale.BS_HM_Hair_01,BS_HumanMale.BS_HM_Hands_00,BS_HumanMale.BS_HM_Head_03,BS_HumanMale.BS_HM_Legs_00,BS_HumanMale.BS_HM_Torso_00}', 0, 0, 570, 50, 0, 1, 7368, NULL, NULL, NULL, 'Beta E1 - Dr. Felger', 'mob', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, -340158464, NULL, '{}', NULL, 710, true, NULL, NULL, NULL, 300, 'Dr. Felger');

-- 2302 Captain Satterfield, the Beta E1 officer.
-- name_id 7373 'DN_npc_MG_BetaE1_Admin_Satterfield': RESTORED_SOURCE, "Captain Satterfield".
-- speaker 715: RESTORED_SOURCE.
-- look: CIMMERIA_RECONSTRUCTION, copied from 31 Airman (body, components, skin_tint).
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (2302, NULL, 'BS_HumanMale.BS_HumanMale', '{ACC_HumanMale.ACC_HM_Accessory_09,AR_H_Ballistic00.AR_HM_BT1_BT101,AR_H_SGC.AR_HM_SB1_SH100,AR_H_SGC.AR_HM_SL1_SL100,BS_HumanMale.BS_HM_Boots_00,BS_HumanMale.BS_HM_Hair_01,BS_HumanMale.BS_HM_Hands_00,BS_HumanMale.BS_HM_Head_03,BS_HumanMale.BS_HM_Legs_00,BS_HumanMale.BS_HM_Torso_00}', 0, 0, 570, 50, 0, 1, 7373, NULL, NULL, NULL, 'Beta E1 - Captain Satterfield', 'mob', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, -340158464, NULL, '{}', NULL, 715, true, NULL, NULL, NULL, 300, NULL);

-- 2304 Lt. Debeaux, the Beta E1 officer. No speaker exists for him in the
-- client's speakers table.
-- name_id 8552 'DN_NPC_Int_BetaE1_Admin_Debeaux': RESTORED_SOURCE, "Lt. Debeaux".
-- look: CIMMERIA_RECONSTRUCTION, copied from 31 Airman (body, components, skin_tint).
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (2304, NULL, 'BS_HumanMale.BS_HumanMale', '{ACC_HumanMale.ACC_HM_Accessory_09,AR_H_Ballistic00.AR_HM_BT1_BT101,AR_H_SGC.AR_HM_SB1_SH100,AR_H_SGC.AR_HM_SL1_SL100,BS_HumanMale.BS_HM_Boots_00,BS_HumanMale.BS_HM_Hair_01,BS_HumanMale.BS_HM_Hands_00,BS_HumanMale.BS_HM_Head_03,BS_HumanMale.BS_HM_Legs_00,BS_HumanMale.BS_HM_Torso_00}', 0, 0, 570, 50, 0, 1, 8552, NULL, NULL, NULL, 'Beta E1 - Lt. Debeaux', 'mob', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, -340158464, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);

-- 2305 Mimir, at the Asgard base.
-- name_id 7388 'DN_npc_MG_BetaE1_AsgardBase_Mimir': RESTORED_SOURCE, "Mimir".
-- speaker 730: RESTORED_SOURCE.
-- look: CIMMERIA_RECONSTRUCTION, copied from 64 Thor (body and components, the Asgard look).
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (2305, NULL, 'BS_Asgard.BS_Asgard', '{BS_Asgard.BS_AS_Base_Boots00_00,BS_Asgard.BS_AS_Base_Hands00_00,BS_Asgard.BS_AS_Base_Head00_00,BS_Asgard.BS_AS_Base_Legs00_00,BS_Asgard.BS_AS_Base_Torso00_00}', 0, 0, 570, 50, 0, 1, 7388, NULL, NULL, NULL, 'Beta E1 - Mimir', 'mob', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, 730, true, NULL, NULL, NULL, 300, NULL);

-- 2306 Ras''nor, at his camp (the Beta E1 Jaffa leader).
-- name_id 7374 'DN_npc_MG_BetaE1_RasnorsCamp_Rasnor': RESTORED_SOURCE, "Ras'nor".
-- speaker 716: RESTORED_SOURCE.
-- look: CIMMERIA_RECONSTRUCTION, copied from 440 Loth'ta (the Standard Jaffa
--   armour), without the staff: nobody fights on a first visit. No ability set.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (2306, NULL, 'BS_JaffaMale.BS_JaffaMale', '{AR_J_Standard.AR_JM_SB1_SH100,AR_J_Standard.AR_JM_SG1_SG100,AR_J_Standard.AR_JM_SH1_SH100,AR_J_Standard.AR_JM_SL1_SS100,AR_J_Standard.AR_JM_ST1_ST100SS100SP100,BS_JaffaMale.BS_JM_Boots_00,BS_JaffaMale.BS_JM_Hands_00,BS_JaffaMale.BS_JM_Head_00,BS_JaffaMale.BS_JM_Legs_00,BS_JaffaMale.BS_JM_Torso_00}', 0, 0, 570, 50, 0, 1, 7374, NULL, NULL, NULL, 'Beta E1 - Ras''nor', 'mob', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, 716, true, NULL, NULL, NULL, 300, NULL);

-- 2307 A sick Nine Peaks warrior. Level 5, as Dakara's 444 Free Jaffa Warrior.
-- name_id 7414 'DN_npc_Int_BetaE1_Various_SickJaffa': RESTORED_SOURCE, the client's text.
-- look: CIMMERIA_RECONSTRUCTION, copied from 444 Free Jaffa Warrior, without
--   the staff. No ability set.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (2307, NULL, 'BS_JaffaMale.BS_JaffaMale', '{AR_J_Standard.AR_JM_SB1_SH100,AR_J_Standard.AR_JM_SG1_SG100,AR_J_Standard.AR_JM_SH1_SH100,AR_J_Standard.AR_JM_SL1_SS100,AR_J_Standard.AR_JM_ST1_ST100SS100SP100,BS_JaffaMale.BS_JM_Boots_00,BS_JaffaMale.BS_JM_Hands_00,BS_JaffaMale.BS_JM_Head_00,BS_JaffaMale.BS_JM_Legs_00,BS_JaffaMale.BS_JM_Torso_00}', 0, 0, 570, 5, 0, 1, 7414, NULL, NULL, NULL, 'Beta E1 - Sick Nine Peaks Warrior', 'mob', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);

--
-- The objects. class being, flags 4, interaction_type 1073741824: the bit
-- makes each one clickable from spawn; a mission packet binds the dialog per
-- player. Templates 2327 and 2336 are reserved (see the header).
--

-- 2320 Xenobiology sample cage (UB-11).
-- name_id 7376 'DN_Ob_It_Human_BetaE1_XenobiologyCage': RESTORED_SOURCE.
-- look: CIMMERIA_RECONSTRUCTION, flags and interaction bit as 445 (Dakara tent flap); mesh and body set from UB-00a2 Q10.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (2320, 'Em-Props.EM-Fauna_Cage00', 'GLB_Components.WorldObject_Small', NULL, 4, 1073741824, NULL, 1, 0, 1, 7376, NULL, NULL, NULL, 'Beta E1 - Xenobiology Sample Cage', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);

-- 2321 Artifact display case (UB-15).
-- name_id 7377 'DN_Ob_It_Human_BetaE1_FourPiecesCase': RESTORED_SOURCE.
-- look: CIMMERIA_RECONSTRUCTION, flags and interaction bit as 445 (Dakara tent flap); mesh and body set from UB-00a2 Q10.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (2321, 'LUS-Props.LUS-DisplayCase00', 'GLB_Components.WorldObject_Small', NULL, 4, 1073741824, NULL, 1, 0, 1, 7377, NULL, NULL, NULL, 'Beta E1 - Artifact Display Case', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);

-- 2322 Beta Site hub computer (UB-14, UB-15).
-- name_id 7383 'DN_Ob_D_Human_BetaE1_HubComputer': RESTORED_SOURCE.
-- look: CIMMERIA_RECONSTRUCTION, flags and interaction bit as 445 (Dakara tent flap); mesh and body set from UB-00a2 Q10.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (2322, 'SGC_Props.SGC-console05', 'GLB_Components.WorldObject_StandingTerminal_Med', NULL, 4, 1073741824, NULL, 1, 0, 1, 7383, NULL, NULL, NULL, 'Beta E1 - Beta Site Hub Computer', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);

-- 2323 Admin hub workstation (UB-13).
-- name_id 8493 'DN_Ob_It_Human_BetaE1_AdminWorkstation': RESTORED_SOURCE.
-- look: CIMMERIA_RECONSTRUCTION, flags and interaction bit as 445 (Dakara tent flap); mesh and body set from UB-00a2 Q10.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (2323, 'SGC_Props.SGC-console01', 'GLB_Components.WorldObject_StandingTerminal_Low', NULL, 4, 1073741824, NULL, 1, 0, 1, 8493, NULL, NULL, NULL, 'Beta E1 - Admin Hub Workstation', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);

-- 2324 Science center computer (UB-14).
-- name_id 8489 'DN_Ob_D_Human_BetaE1_ScienceCenterComputer_02SGUDatabase': RESTORED_SOURCE.
-- look: CIMMERIA_RECONSTRUCTION, flags and interaction bit as 445 (Dakara tent flap); mesh and body set from UB-00a2 Q10.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (2324, 'SGC_Props.SGC-console02', 'GLB_Components.WorldObject_StandingTerminal_Med', NULL, 4, 1073741824, NULL, 1, 0, 1, 8489, NULL, NULL, NULL, 'Beta E1 - Science Center Computer', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);

-- 2325 Wrecked SGC shipping crates (UB-12).
-- name_id 8490 'DN_Ob_It_Human_BetaE1_CrateWreckage': RESTORED_SOURCE.
-- look: CIMMERIA_RECONSTRUCTION, flags and interaction bit as 445 (Dakara tent flap); mesh and body set from UB-00a2 Q10.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (2325, 'Em-Props.EM-Wooden_Crate_open00', 'GLB_Components.WorldObject_Small', NULL, 4, 1073741824, NULL, 1, 0, 1, 8490, NULL, NULL, NULL, 'Beta E1 - Wrecked SGC Shipping Crates', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);

-- 2326 Box of Jaffa tools (UB-21).
-- name_id 7402 'DN_Ob_D_Jaffa_BetaE1_BoxOfJaffaTools': RESTORED_SOURCE.
-- look: CIMMERIA_RECONSTRUCTION, flags and interaction bit as 445 (Dakara tent flap); mesh and body set from UB-00a2 Q10.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (2326, 'GA-Props.GA-AmmoCrate00', 'GLB_Components.WorldObject_Small', NULL, 4, 1073741824, NULL, 1, 0, 1, 7402, NULL, NULL, NULL, 'Beta E1 - Box of Jaffa Tools', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);

-- 2328 Asgard fire suppressant manager (UB-34).
-- name_id 7397 'DN_Ob_it_Asgard_BetaE1_FireSuppressantManager': RESTORED_SOURCE.
-- look: CIMMERIA_RECONSTRUCTION, flags and interaction bit as 445 (Dakara tent flap); mesh and body set from UB-00a2 Q10.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (2328, 'ASG-Props.ASG-ComputerConsole01', 'GLB_Components.WorldObject_StandingTerminal_Low', NULL, 4, 1073741824, NULL, 1, 0, 1, 7397, NULL, NULL, NULL, 'Beta E1 - Asgard Fire Suppressant Manager', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);

-- 2329 Fire control hub (UB-34).
-- name_id 7957 'DN_Ob_G_Human_BetaE1_SprinklerSystemManularControl': RESTORED_SOURCE.
-- look: CIMMERIA_RECONSTRUCTION, flags and interaction bit as 445 (Dakara tent flap); mesh and body set from UB-00a2 Q10.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (2329, 'Em-Props.EM-SGC_Failsafe_Console00', 'GLB_Components.WorldObject_StandingTerminal_Med', NULL, 4, 1073741824, NULL, 1, 0, 1, 7957, NULL, NULL, NULL, 'Beta E1 - Fire Control Hub', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);

-- 2330 Environmental reading device (UB-31). The mesh name is the client's
-- own spelling, GP-Asgard_Terminal_Console00 (not the misspelt GP-Agard_...).
-- name_id 7392 'DN_Ob_It_Asgard_BetaE1_AsgardScanner': RESTORED_SOURCE.
-- look: CIMMERIA_RECONSTRUCTION, flags and interaction bit as 445 (Dakara tent flap); mesh and body set from UB-00a2 Q10.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (2330, 'GP-Props.GP-Asgard_Terminal_Console00', 'GLB_Components.WorldObject_StandingTerminal_Low', NULL, 4, 1073741824, NULL, 1, 0, 1, 7392, NULL, NULL, NULL, 'Beta E1 - Environmental Reading Device', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);

-- 2331 Analytical assistance machine (UB-33, UB-36).
-- name_id 7909 'DN_Ob_G_ASG_BetaE1_AnalyticalAssistanceMachine': RESTORED_SOURCE.
-- look: CIMMERIA_RECONSTRUCTION, flags and interaction bit as 445 (Dakara tent flap); mesh and body set from UB-00a2 Q10.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (2331, 'ASG-Props.ASG-ComputerConsole03', 'GLB_Components.WorldObject_StandingTerminal_Med', NULL, 4, 1073741824, NULL, 1, 0, 1, 7909, NULL, NULL, NULL, 'Beta E1 - Analytical Assistance Machine', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);

-- 2332 Missing Asgard hologram emitter (UB-32).
-- name_id 8164 'DN_Ob_It_Asgard_BetaE1_MissingAsgardHologramEmitter': RESTORED_SOURCE.
-- look: CIMMERIA_RECONSTRUCTION, flags and interaction bit as 445 (Dakara tent flap); mesh and body set from UB-00a2 Q10.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (2332, 'ASG-Props.ASG-Hologram_Platform00', 'GLB_Components.WorldObject_Small', NULL, 4, 1073741824, NULL, 1, 0, 1, 8164, NULL, NULL, NULL, 'Beta E1 - Missing Asgard Hologram Emitter', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);

-- 2333 Disabled malfunctioning drone (UB-32). The one object that draws a
-- body, not a mesh: the body set and components are template 69's
-- (Malfunctioning Drone, MOB_Goauld_Drone), so the prop is the drone itself.
-- The visual guard pins this row by id. Class being and the object bit.
-- name_id 8211 'DN_Ob_G_Human_BetaE1_DisabledMalfunctioningDrone': RESTORED_SOURCE.
-- look: CIMMERIA_RECONSTRUCTION, body set and components from 69.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (2333, NULL, 'MOB_Goauld_Drone.MOB_Goauld_Drone', '{MOB_Goauld_Drone.MOB_Goauld_Drone00}', 4, 1073741824, NULL, 1, 0, 1, 8211, NULL, NULL, NULL, 'Beta E1 - Disabled Malfunctioning Drone', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);

-- 2334 Consciousness simulator (UB-36).
-- name_id 8497 'DN_Ob_It_Asgard_BetaE1_ConsciousnessSimulator': RESTORED_SOURCE.
-- look: CIMMERIA_RECONSTRUCTION, flags and interaction bit as 445 (Dakara tent flap); mesh and body set from UB-00a2 Q10.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (2334, 'ASG-Props.ASG-Hologram_Console00', 'GLB_Components.WorldObject_StandingTerminal_Med', NULL, 4, 1073741824, NULL, 1, 0, 1, 8497, NULL, NULL, NULL, 'Beta E1 - Consciousness Simulator', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);

-- 2335 A note at the ruins (UB-16). The client has no moniker for this object:
-- dialog 4663 is the "post-it" object dialog, so name_id is NULL and the
-- nameplate is the literal "Note".
-- name_id: none (CIMMERIA_RECONSTRUCTION, no moniker in the client).
-- display_name: CIMMERIA_RECONSTRUCTION, "Note".
-- look: CIMMERIA_RECONSTRUCTION, flags and interaction bit as 445; mesh and body set from UB-00a2 Q10.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (2335, 'SGC_Props.SGC-paper_holder', 'GLB_Components.WorldObject_Small', NULL, 4, 1073741824, NULL, 1, 0, 1, NULL, NULL, NULL, NULL, 'Beta E1 - Note at the ruins', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, 'Note');

-- Reserved: 2303 (Dr. Pierce), 2327 (Accelerant) and 2336 have no row here.
-- They are UB-52's twins 383 and 384 and stay empty until UB-52 is shipped.

-- Keep default-id inserts past the Beta E1 block 2300-2349.
SELECT pg_catalog.setval('entity_templates_template_id_seq', GREATEST((SELECT MAX(template_id) FROM entity_templates), (SELECT last_value FROM entity_templates_template_id_seq), 2349), true);
