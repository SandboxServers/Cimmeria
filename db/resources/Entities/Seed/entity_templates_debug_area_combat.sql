--
-- Debug Area (world 1300) combat-zone templates: DA-04, templates 1370-1399.
-- Plan and decisions: docs/analysis/debug-area/README.md (D-DA8). Reference:
-- docs/content/debug-area.md. Spawns: Worlds/Seed/spawnlist_debug_area_combat.sql.
--
-- Loaded right after entity_templates.sql (db/database.sql). Looks, names and
-- ability sets are copied from templates already seen in the client: no new
-- moniker, body part or ability. Tests: crates/cell/src/cell/service/tests/
-- npc_ai/debug_area_combat/ (content tests on the real ihpet_crater_light.nav/.occ
-- and world-1300 cover; live-DB guards in live_db.rs).
--

-- Z6 NPC-vs-NPC arena, fight 1 (D-DA8): faction 3 (Praxis) against faction 10
-- (Straegis), the only seeded mutually HOSTILE pair. A player reacts as faction 3,
-- so the Praxis side is friendly and cannot be damaged, and a player joins the
-- fight by shooting the NID side. aggro_radius 30 covers the 24 u between the
-- squads (default 18 would not); the pit sits 25 u below the crater floor, so the
-- 4 u vertical band keeps rim spectators out of the scan. No loot on the squads:
-- the fight repeats every 30 s respawn and would litter the pit with corpses.
-- 1370 clones 187's look (Op-CORE prisoner clothes, SMG) with set 3 so it fires
-- the SMG it holds; 1371 clones 189 (Praxis Jaffa, set 4 staff) and adds the
-- staff it used to mime; 1372 is the NID Guard look of template 24.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, respawn_secs, use_cover, aggro_radius) VALUES (1370, NULL, 'BS_HumanMale.BS_HumanMale', '{AR_Global.Prisoner_Boots,AR_H_Clothing00.AR_HM_PL1_PL100,AR_H_Clothing00.AR_HM_PT1_PT100,BS_HumanMale.BS_HM_Boots_00,BS_HumanMale.BS_HM_Feet_00,BS_HumanMale.BS_HM_Hands_00,BS_HumanMale.BS_HM_Head_02,BS_HumanMale.BS_HM_Legs_00,BS_HumanMale.BS_HM_Torso_00,WP-Human.WP_SMG_1A}', 0, 0, 570, 5, 0, 3, 7552, NULL, NULL, NULL, 'DebugArea Arena Op-CORE Soldier', 'mob', NULL, NULL, NULL, NULL, 3, NULL, NULL, -256, -256, -52773120, 21, '{}', NULL, NULL, true, NULL, NULL, 30, false, 30);
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, respawn_secs, use_cover, aggro_radius) VALUES (1371, NULL, 'BS_JaffaMale.BS_JaffaMale', '{AR_J_Praxis.AR_JM_PB1_PH101,AR_J_Praxis.AR_JM_PG1_PG100PB100,AR_J_Praxis.AR_JM_PH1_PH100,AR_J_Praxis.AR_JM_PL1_PL101,AR_J_Praxis.AR_JM_PT1_PT100,BS_JaffaMale.BS_JM_Boots_00,BS_JaffaMale.BS_JM_Hands_00,BS_JaffaMale.BS_JM_Head_00,BS_JaffaMale.BS_JM_Legs_00,BS_JaffaMale.BS_JM_Torso_00,WP-Jaffa.WP_Staff_Plasma_4A}', 0, 0, 570, 5, 0, 3, 8917, NULL, NULL, NULL, 'DebugArea Arena Praxis Jaffa', 'mob', NULL, NULL, NULL, NULL, 4, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, 30, false, 30);
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, respawn_secs, use_cover, aggro_radius) VALUES (1372, NULL, 'BS_HumanMale.BS_HumanMale', '{AR_H_SGC.AR_HM_SB1_SH100,AR_H_SGC.AR_HM_SH1_SH100,AR_H_SGC.AR_HM_SL1_SL101SB100SH100,AR_H_SGC.AR_HM_ST1_ST103,BS_HumanMale.BS_HM_Hands_00,BS_HumanMale.BS_HM_Head_01,WP-Human.WP_SMG_1A}', 0, 0, 570, 5, 0, 10, 7417, NULL, NULL, NULL, 'DebugArea Arena NID Guard', 'mob', NULL, NULL, NULL, NULL, 3, 'Bullet_Default', NULL, 0, 0, -256076032, 21, '{}', NULL, NULL, true, NULL, NULL, 30, false, 30);

-- Z6 arena, fight 2 (D-DA8): a spectator-only pair, faction 27 (Lucia_Green)
-- against faction 29 (Lucia_Yellow). Mutually HOSTILE, FRIENDLY to faction 3 in
-- both directions, and neutral to 10, so neither side ever targets a player or
-- the fight-1 squads, and a player cannot damage either (only faction 10 is
-- damageable). An NPC-only kill rolls no loot, pays no XP and moves no mission
-- (#1009); both carry loot table 2 so the skip is visible as
-- `loot.drop event=skipped reason=npc_only_kill loot_table_id=2`. The names are
-- the shipped Lucia monikers "Green Sniper" and "Yellow Faction". 1374 keeps
-- template 219's look and pistol (set 1); WP_SMG_1A has no BS_HumanFemale row in
-- body_components. aggro_radius 26 covers their 20-22 u gap.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, respawn_secs, use_cover, aggro_radius) VALUES (1373, NULL, 'BS_HumanMale.BS_HumanMale', '{AR_H_Clothing00.AR_HM_WinterPants00,AR_H_Clothing00.AR_HM_WinterShirt00,BS_HumanMale.BS_HM_Boots_00,BS_HumanMale.BS_HM_Feet_00,BS_HumanMale.BS_HM_Hands_00,BS_HumanMale.BS_HM_Head_01,BS_HumanMale.BS_HM_Legs_00,BS_HumanMale.BS_HM_Torso_00,WP-Human.WP_SMG_1A}', 0, 0, 570, 5, 0, 27, 7916, NULL, NULL, NULL, 'DebugArea Arena Green Sniper', 'mob', NULL, NULL, NULL, NULL, 3, NULL, 2, -65536, -16777216, -256076032, NULL, '{}', NULL, NULL, true, NULL, NULL, 30, false, 26);
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, respawn_secs, use_cover, aggro_radius) VALUES (1374, NULL, 'BS_HumanFemale.BS_HumanFemale', '{BS_HumanFemale.BS_HF_Boots_00,BS_HumanFemale.BS_HF_Hair_01,BS_HumanFemale.BS_HF_Hands_00,BS_HumanFemale.BS_HF_Head_00,BS_HumanFemale.BS_HF_Legs_00,BS_HumanFemale.BS_HF_Torso_00,WP-Human.WP_Pistol_1A}', 0, 0, 570, 5, 0, 29, 7927, NULL, NULL, NULL, 'DebugArea Arena Yellow Faction', 'mob', NULL, NULL, NULL, NULL, 1, NULL, 2, 0, 0, -52773120, NULL, '{}', NULL, NULL, true, NULL, NULL, 30, false, 26);

-- Z8 cover course: hostile ranged riflemen with use_cover true (NA22/NA23, step
-- back per D-NA15). NID Guard look and SMG set 3 of template 24. aggro_radius 25
-- so they engage a tester coming through the wing before he is on top of them.
-- Loot table 2 (the Cellblock guards') so a kill also exercises a corpse.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, respawn_secs, use_cover, aggro_radius) VALUES (1375, NULL, 'BS_HumanMale.BS_HumanMale', '{AR_H_SGC.AR_HM_SB1_SH100,AR_H_SGC.AR_HM_SH1_SH100,AR_H_SGC.AR_HM_SL1_SL101SB100SH100,AR_H_SGC.AR_HM_ST1_ST103,BS_HumanMale.BS_HM_Hands_00,BS_HumanMale.BS_HM_Head_01,WP-Human.WP_SMG_1A}', 0, 0, 570, 5, 0, 10, 7417, NULL, NULL, NULL, 'DebugArea Cover Rifleman', 'mob', NULL, NULL, NULL, NULL, 3, 'Bullet_Default', 2, 0, 0, -256076032, 21, '{}', NULL, NULL, true, NULL, NULL, 30, true, 25);

-- Z9 death and respawn test. 1376 is the lethal hostile: four of them stand
-- together, each firing 559 (200 Focus / 20 Health, focus-gated bleed) every
-- AI tick, which strips a fresh character's 1,570 Focus and 760 Health in five
-- AI ticks (about 10 s). Level 31 (1,750 HP) so a low-level character cannot win.
-- aggro_radius 10: a tester has to walk up to them, and the respawner 131 is
-- 28 u away, so a respawned character is never pulled. Name "NID Operative"
-- (the Harset level-31 SMG hostile's moniker), look of template 24.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, respawn_secs, use_cover, aggro_radius) VALUES (1376, NULL, 'BS_HumanMale.BS_HumanMale', '{AR_H_SGC.AR_HM_SB1_SH100,AR_H_SGC.AR_HM_SH1_SH100,AR_H_SGC.AR_HM_SL1_SL101SB100SH100,AR_H_SGC.AR_HM_ST1_ST103,BS_HumanMale.BS_HM_Hands_00,BS_HumanMale.BS_HM_Head_01,WP-Human.WP_SMG_1A}', 0, 0, 570, 31, 0, 10, 7581, NULL, NULL, NULL, 'DebugArea Death Test NID Operative', 'mob', NULL, NULL, NULL, NULL, 3, 'Bullet_Default', 2, 0, 0, -256076032, 21, '{}', NULL, NULL, true, NULL, NULL, 60, false, 10);
-- 1377 is the NPC respawn-timer target: a level-1 Cellblock Guard (template 15's
-- look and pistol set), seeded NEUTRAL per spawn so it waits to be shot. Its
-- template respawn_secs is 30; one of its two spawns overrides that with 10, so
-- the pair shows spawnlist.respawn_secs taking precedence over the template.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, respawn_secs, use_cover, aggro_radius) VALUES (1377, NULL, 'BS_HumanMale.BS_HumanMale', '{AR_H_SGC.AR_HM_SB1_SH100,AR_H_SGC.AR_HM_SH1_SH100,AR_H_SGC.AR_HM_SL1_SL101SB100SH100,AR_H_SGC.AR_HM_ST1_ST103,BS_HumanMale.BS_HM_Hands_00,BS_HumanMale.BS_HM_Head_01,WP-Human.WP_Pistol_1A}', 0, 0, 570, 1, 0, 10, 6961, NULL, NULL, NULL, 'DebugArea Respawn Timer Target', 'mob', NULL, NULL, NULL, NULL, 1, 'Bullet_Default', 2, 0, 0, -256076032, 55, '{}', NULL, NULL, true, NULL, NULL, 30, false, NULL);

-- Past every seeded row and the whole Debug Area template reservation
-- (DA-02..DA-04, 1300-1399); never lowered. This file loads after
-- entity_templates.sql's own footer, so it repeats it with the higher floor.
-- live_db_seed_sequences.rs guards it.
SELECT pg_catalog.setval('entity_templates_template_id_seq', GREATEST((SELECT MAX(template_id) FROM entity_templates), (SELECT last_value FROM entity_templates_template_id_seq), 1399), true);
