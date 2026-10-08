--
-- Dakara_E1 rebuild, DK-03: the cast and the interactable props of worlds 61
-- `Dakara_E1` and 62 `Dakara_E1_StoryRm`. Templates only: no spawn row, no
-- position and no hostile. Plan and evidence:
-- docs/analysis/dakara-e1-rebuild/ (work-packets.md#dk-03, audit.md,
-- worknotes/DK-03.md).
--
-- Id block 440-459 (cast and props). 460-479 is reserved for DK-20's hostiles
-- and defenders. Both are below the floor the sequence footers keep
-- (entity_templates.sql, 1399), so this file needs no footer of its own.
-- Bra'tac (59) and Moh'katan (54) are not here: they have templates already,
-- shared with Harset, and this campaign does not edit them.
--
-- Evidence labels (docs/analysis/dakara-e1-rebuild/README.md#evidence-labels),
-- one per column group on every row:
--
-- * ORIGINAL_DATA: the `name_id`. Each is the client's own display-name string
--   for that Dakara_E1 actor (`DN_npc_*_DakaraE1*`, `DN_ob_DakaraE1*`, ids
--   26719-26729 and 26731-26737 in the client's TextStrings.pak). The
--   `speaker_id` of 440 and 443 is the speaker the client's own dialogs give
--   that character.
-- * RECONSTRUCTION: everything else. No template, look, level, faction or
--   flag for any of these actors survives in the client or the seed.
--
-- Names. The client leaves the text of six of these strings empty. A row
-- whose string is empty keeps the `name_id` (it is still the client's id for
-- the actor) and sets `display_name`, the literal nameplate the server sends
-- as onBeingNameUpdate in place of the name id
-- (crates/wire/src/mercury/aoi/create.rs). The literal is RECONSTRUCTION: the
-- words of the string's own moniker name in the client's spelling, or for
-- 448 the text of its sibling string. A row whose string has text sets no
-- `display_name`, so the client draws its own text.
--
-- Looks. The client ships no costume kit and no mesh assignment for any of
-- these actors (NPC_Jaffa.upk holds kits for Bra'tac, Moh'katan and Teal'c
-- only). Every look below is copied whole (body set, components, colours,
-- skin tint, static mesh) from an existing template, named in the row
-- comment, so the Debug Area lineup needs no new actor
-- (live_db_debug_area_lineup counts looks, not templates). Each source has a
-- lower id than its copy: the lineup tags an actor with the lowest template
-- id that wears its look, so a copy below its source would rename the actor.
--
-- Every row: faction 1 (World Object: a player cannot attack it, it is
-- friendly to every faction in the reaction table and no faction is hostile
-- to it), alignment 0, no patrol path and no wander radius (it stands where
-- its spawn row puts it; `is_stationary` is the spawn row's column, DK-05),
-- respawn_secs 300 (the Harset default, D-H17), no loot table, no vendor or
-- trainer list.
--
-- Guards: crates/cell-world/src/cell/spawner_tests/dakara_e1/.
--

SET search_path = resources, pg_catalog;

--
-- The cast. class mob, event set 570 (the one the other seeded NPCs carry),
-- interaction_type 0: a mission packet binds the dialog set and its flags per
-- player (docs/content/interaction-flags.md). Level 50 on the four named
-- characters is the level templates 54 and 59 carry; the client records no
-- level for any Dakara_E1 name.
--

-- 440 Loth'ta, at his camp by the Naquadah Repository (missions 1570, 1645,
--   1648). Male: mission steps 4904 and 4919 say "his camp" and "he".
-- name_id 26721 `DN_npc_int_Lothta_DakaraE1`: ORIGINAL_DATA, text empty.
-- display_name: RECONSTRUCTION, the client's spelling (speaker 781; string
--   7439 `DN_MsMb_DakaraE2_Lothta_Uni_28`).
-- speaker 781: ORIGINAL_DATA, the speaker of dialogs 5808, 5812, 5813, 5829.
-- look: RECONSTRUCTION, copied from 206 Angry Jaffa (the Standard Jaffa
--   armour and a staff: the look Harset's Free Jaffa 206-208 wear).
-- ability set 4 (Jaffa staff): the set 206 carries, so the staff he holds is
--   what he would fire. He never fights: faction 1 has no enemy.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (440, NULL, 'BS_JaffaMale.BS_JaffaMale', '{AR_J_Standard.AR_JM_SB1_SH100,AR_J_Standard.AR_JM_SG1_SG100,AR_J_Standard.AR_JM_SH1_SH100,AR_J_Standard.AR_JM_SL1_SS100,AR_J_Standard.AR_JM_ST1_ST100SS100SP100,BS_JaffaMale.BS_JM_Boots_00,BS_JaffaMale.BS_JM_Hands_00,BS_JaffaMale.BS_JM_Head_00,BS_JaffaMale.BS_JM_Legs_00,BS_JaffaMale.BS_JM_Torso_00,WP-Jaffa.WP_Staff_Plasma_4A}', 0, 0, 570, 50, 0, 1, 26721, NULL, NULL, NULL, 'Dakara E1 - Loth''ta', 'mob', NULL, NULL, NULL, NULL, 4, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, 781, true, NULL, NULL, NULL, 300, 'Loth''ta');

-- 441 Rak'nor, the greeter at the gate plaza (mission 1570, objective 5936).
-- name_id 26720 `DN_npc_int_Raknor_DakaraE1`: ORIGINAL_DATA, "Rak'nor".
-- speaker 2956: RECONSTRUCTION. The client leaves the speaker's name empty;
--   it speaks dialogs 6110 and 6111, the lines that send the player to the
--   command tent, and objective 5936 names Rak'nor as who says where it is.
-- look: RECONSTRUCTION, copied from 100 Standard Jaffa (the same armour,
--   unarmed). No ability set.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (441, NULL, 'BS_JaffaMale.BS_JaffaMale', '{AR_J_Standard.AR_JM_SB1_SH100,AR_J_Standard.AR_JM_SG1_SG100,AR_J_Standard.AR_JM_SH1_SH100,AR_J_Standard.AR_JM_SL1_SS100,AR_J_Standard.AR_JM_ST1_ST100SS100SP100,BS_JaffaMale.BS_JM_Boots_00,BS_JaffaMale.BS_JM_Hands_00,BS_JaffaMale.BS_JM_Head_00,BS_JaffaMale.BS_JM_Legs_00,BS_JaffaMale.BS_JM_Torso_00}', 0, 0, 570, 50, 0, 1, 26720, NULL, NULL, NULL, 'Dakara E1 - Rak''nor', 'mob', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, 2956, true, NULL, NULL, NULL, 300, NULL);

-- 442 Jaffa Captain, at the Western Gate (mission 1647).
-- name_id 26722 `DN_npc_int_JaffaCaptain_DakaraE1`: ORIGINAL_DATA, text
--   empty. display_name: RECONSTRUCTION, the moniker's own words.
-- speaker 2959: RECONSTRUCTION. Empty name in the client; it speaks dialogs
--   6106 and 6107 ("Moh'katan sends one of the Tau'ri to command us?").
--   Speaker 2958 (dialog 5823, the commander at the Eastern tents) has no
--   template: see worknotes/DK-03.md, open questions.
-- look and ability set: RECONSTRUCTION, as 440 (copied from 206).
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (442, NULL, 'BS_JaffaMale.BS_JaffaMale', '{AR_J_Standard.AR_JM_SB1_SH100,AR_J_Standard.AR_JM_SG1_SG100,AR_J_Standard.AR_JM_SH1_SH100,AR_J_Standard.AR_JM_SL1_SS100,AR_J_Standard.AR_JM_ST1_ST100SS100SP100,BS_JaffaMale.BS_JM_Boots_00,BS_JaffaMale.BS_JM_Hands_00,BS_JaffaMale.BS_JM_Head_00,BS_JaffaMale.BS_JM_Legs_00,BS_JaffaMale.BS_JM_Torso_00,WP-Jaffa.WP_Staff_Plasma_4A}', 0, 0, 570, 50, 0, 1, 26722, NULL, NULL, NULL, 'Dakara E1 - Jaffa Captain', 'mob', NULL, NULL, NULL, NULL, 4, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, 2959, true, NULL, NULL, NULL, 300, 'Jaffa Captain');

-- 443 Ba'al's hologram, in Moh'katan's tent (mission 1650, dialog 5840).
-- name_id 26719 `DN_npc_int_Baal_DakaraE1_Hologram`: ORIGINAL_DATA,
--   "Baal (Hologram)".
-- speaker 942: ORIGINAL_DATA, the speaker of Ba'al's line in dialog 5840.
-- look: RECONSTRUCTION, copied from 167 Sandbox Ba'al (his head and the
--   Praxis Goa'uld armour; template 42 is the head on a bare body). It is a
--   solid body: nothing here makes it look like a projection.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (443, NULL, 'BS_GoauldMale.BS_GoauldMale', '{AR_G_Praxis.AR_GM_PB1_PH100,AR_G_Praxis.AR_GM_PL1_PB100,AR_G_Praxis.AR_GM_PL1_PL100,AR_G_Praxis.AR_GM_PT1_PT100,AR_G_Praxis.AR_GM_PT1_PT102,AR_G_Praxis.AR_GM_PT1_PT105,BS_GoauldMale.BS_GM_Base_Boots00_00,BS_GoauldMale.BS_GM_Base_Hands00_00,BS_GoauldMale.BS_GM_Base_Legs00_00,BS_GoauldMale.BS_GM_Base_Torso00_00,NPC_Goauld.NPC_GM_Baal_Head_BC}', 0, 0, 570, 50, 0, 1, 26719, NULL, NULL, NULL, 'Dakara E1 - Ba''al (Hologram)', 'mob', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, -1014470144, NULL, '{}', NULL, 942, true, NULL, NULL, NULL, 300, NULL);

-- 444 Free Jaffa Warrior, not hostile. Template 208 is Harset's hostile
--   bearer of the same name.
-- name_id 26723 `DN_npc_esc_Jaffa_DakaraE1_Escort`: ORIGINAL_DATA,
--   "Free Jaffa Warrior".
-- look and ability set: RECONSTRUCTION, as 440 (copied from 206, the look
--   208 wears). Level 5: RECONSTRUCTION, the top of the arc's missions
--   (levels 3 to 5).
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (444, NULL, 'BS_JaffaMale.BS_JaffaMale', '{AR_J_Standard.AR_JM_SB1_SH100,AR_J_Standard.AR_JM_SG1_SG100,AR_J_Standard.AR_JM_SH1_SH100,AR_J_Standard.AR_JM_SL1_SS100,AR_J_Standard.AR_JM_ST1_ST100SS100SP100,BS_JaffaMale.BS_JM_Boots_00,BS_JaffaMale.BS_JM_Hands_00,BS_JaffaMale.BS_JM_Head_00,BS_JaffaMale.BS_JM_Legs_00,BS_JaffaMale.BS_JM_Torso_00,WP-Jaffa.WP_Staff_Plasma_4A}', 0, 0, 570, 5, 0, 1, 26723, NULL, NULL, NULL, 'Dakara E1 - Free Jaffa Warrior', 'mob', NULL, NULL, NULL, NULL, 4, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);

--
-- Tent flaps: the four entrances between world 61 and the tent interior,
-- world 62. class being (a plain spawnable is sent no name), flags 4
-- (ENTITYFLAG_DoNotDrop, as every seeded world object).
-- look: RECONSTRUCTION, copied from 22 Door control panel (the wall switch
--   mesh). The client has no tent flap mesh of its own; the flap is part of
--   the tent prefab on the map.
-- interaction_type 1073741824 (INT_MissionWorldObject): RECONSTRUCTION. A
--   flap is usable by everyone at any time, so the bit that makes it
--   clickable is on the template, as on the DHD (1) and the ring switch (3).
--   The client's own interaction sets for them (dialog sets 1912-1915,
--   "Enter Tent" and "Exit Tent") carry no flag at all. DK-04 writes the
--   chains that answer the click.
--

-- 445 name_id 26734 `DN_ob_DakaraE1_sc_TentFlap_ToCommand`: ORIGINAL_DATA,
--   "Command Tent Entrance".
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (445, 'GLB-Global.GLB-RingTransporterSwitch_00', 'GLB_Components.WorldObject_WallTerminal', NULL, 4, 1073741824, NULL, 1, 0, 1, 26734, NULL, NULL, NULL, 'Dakara E1 - Tent Flap (to Command Tent)', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);
-- 446 name_id 26735 `DN_ob_DakaraE1_sc_TentFlap_ToMohkatan`: ORIGINAL_DATA,
--   "Moh'katan's Tent Entrance".
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (446, 'GLB-Global.GLB-RingTransporterSwitch_00', 'GLB_Components.WorldObject_WallTerminal', NULL, 4, 1073741824, NULL, 1, 0, 1, 26735, NULL, NULL, NULL, 'Dakara E1 - Tent Flap (to Moh''katan''s Tent)', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);
-- 447 name_id 26736 `DN_ob_DakaraE1_sc_TentFlap_FromCommand`: ORIGINAL_DATA,
--   "Return to Dakara".
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (447, 'GLB-Global.GLB-RingTransporterSwitch_00', 'GLB_Components.WorldObject_WallTerminal', NULL, 4, 1073741824, NULL, 1, 0, 1, 26736, NULL, NULL, NULL, 'Dakara E1 - Tent Flap (from Command Tent)', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);
-- 448 name_id 26737 `DN_ob_DakaraE1_sc_TentFlap_FromMohkatan`: ORIGINAL_DATA,
--   text empty. display_name: RECONSTRUCTION, the text of its sibling 26736.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (448, 'GLB-Global.GLB-RingTransporterSwitch_00', 'GLB_Components.WorldObject_WallTerminal', NULL, 4, 1073741824, NULL, 1, 0, 1, 26737, NULL, NULL, NULL, 'Dakara E1 - Tent Flap (from Moh''katan''s Tent)', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, 'Return to Dakara');

--
-- Mission props. class being, flags 4, interaction_type 0: each is clickable
-- only for a player on the step that uses it, so the mission packet raises
-- the bit per player with a dialog-set bind (interaction-flags.md). Every
-- mesh is a placeholder some seeded template already uses; none is the mesh
-- the 2009 designers chose, because no assignment survives.
--

-- 449 Drop Location: the five drop sites of mission 1646, searched again in
--   1649. name_id 26727 `DN_ob_DakaraE1_int_DropLoc`: ORIGINAL_DATA.
-- look: RECONSTRUCTION, copied from 244 Petbe's Quarters Search Object.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (449, 'Em-Props.EM-ShelfBox13', 'GLB_Components.WorldObject_Small', NULL, 4, 0, NULL, 1, 0, 1, 26727, NULL, NULL, NULL, 'Dakara E1 - Drop Location', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);

-- 450-452 The remains of SG-18 (mission 1570, step 4901: collect the dog
--   tags). name_ids 26724-26726 `DN_ob_DakaraE1_int_SG18Corpse01..03`:
--   ORIGINAL_DATA, "Maj. Louis", "Lt. Nguyn", "Lt. Waters".
-- look: RECONSTRUCTION, all three copied from 40 SGC_W1 Airman corpse.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (450, 'GP-Props.GP-ScientistCorpse02', 'GLB_Components.WorldObject_Small', NULL, 4, 0, NULL, 1, 0, 1, 26724, NULL, NULL, NULL, 'Dakara E1 - SG-18 Remains (Maj. Louis)', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (451, 'GP-Props.GP-ScientistCorpse02', 'GLB_Components.WorldObject_Small', NULL, 4, 0, NULL, 1, 0, 1, 26725, NULL, NULL, NULL, 'Dakara E1 - SG-18 Remains (Lt. Nguyn)', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (452, 'GP-Props.GP-ScientistCorpse02', 'GLB_Components.WorldObject_Small', NULL, 4, 0, NULL, 1, 0, 1, 26726, NULL, NULL, NULL, 'Dakara E1 - SG-18 Remains (Lt. Waters)', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);

-- 453 Command Terminal: the terminal mission 1649 has the player hack
--   (objective 5638). name_id 26728 `DN_ob_DakaraE1_int_CommandTerminal`:
--   ORIGINAL_DATA, text empty. display_name: RECONSTRUCTION.
-- look: RECONSTRUCTION, copied from 303 Debug Hub - Livewire Terminal.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (453, 'Em-Props.EM-ViewScreen02', 'GLB_Components.WorldObject_WallTerminal', NULL, 4, 0, NULL, 1, 0, 1, 26728, NULL, NULL, NULL, 'Dakara E1 - Command Terminal', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, 'Command Terminal');

-- 454 Moh'katan's Terminal, in the tent interior (mission 1650, step 4926).
--   name_id 26729 `DN_ob_DakaraE1StoryRm_int_MohkatanTerminal`:
--   ORIGINAL_DATA, text empty. display_name: RECONSTRUCTION.
-- look: RECONSTRUCTION, as 453.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (454, 'Em-Props.EM-ViewScreen02', 'GLB_Components.WorldObject_WallTerminal', NULL, 4, 0, NULL, 1, 0, 1, 26729, NULL, NULL, NULL, 'Dakara E1 - Moh''katan''s Terminal', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, 'Moh''katan''s Terminal');

-- 455 Ring Control: the controls of the two ring transporters under the
--   Ha'taks (mission 1652). name_id 26731 `DN_ob_DakaraE1_int_RingControl`:
--   ORIGINAL_DATA, text empty. display_name: RECONSTRUCTION.
-- look: RECONSTRUCTION, copied from 3 Ring Transporter Switch, without its
--   INT_RingNetwork bit: this is a mission device, not a ring network node.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (455, 'GP-Props.GP-Ring_Trans_Console00', 'GLB_Components.WorldObject_WallTerminal', NULL, 4, 0, NULL, 1, 0, 1, 26731, NULL, NULL, NULL, 'Dakara E1 - Ring Control', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, 'Ring Control');

-- 456 Power Supply: the two turret power supplies of mission 1653
--   (objectives 5641, 5642). name_id 26732
--   `DN_ob_DakaraE1_int_TurretPowerSupply`: ORIGINAL_DATA, "Power Supply".
-- look: RECONSTRUCTION, copied from 38 SGC_W1 Bomb.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (456, 'GA-Props.GA-Naq_Bomb00', 'GLB_Components.WorldObject_Small', NULL, 4, 0, NULL, 1, 0, 1, 26732, NULL, NULL, NULL, 'Dakara E1 - Turret Power Supply', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);

-- 457 Vocuum: the projector of Ba'al's image (mission 1653, objective 5643).
--   name_id 26733 `DN_ob_DakaraE1_int_Vocuum`: ORIGINAL_DATA, "Vocuum".
-- look: RECONSTRUCTION, copied from 245 Anat's Symbiote Tank.
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, respawn_secs, display_name) VALUES (457, 'Ga-Props.GA-PuzzleStation00', 'GLB_Components.WorldObject_Small', NULL, 4, 0, NULL, 1, 0, 1, 26733, NULL, NULL, NULL, 'Dakara E1 - Vocuum', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 300, NULL);
