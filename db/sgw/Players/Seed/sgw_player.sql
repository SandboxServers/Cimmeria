--
-- The seeded playtest characters: one per dev account (2-9; account 9 has
-- two, Friendly and Annoying, for contact-list QA).
--
-- Every row is a Praxis Commando (char_def 3, male human) exactly as
-- createCharacter writes one for an account at access level 2 that takes the
-- first choice in every optional visual group and skin tint 0
-- (crates/base/src/base/character_create/). The column list is the
-- handler's INSERT plus player_id, so every other column takes the same
-- table default a created character gets (first_login = 1, no known
-- stargates, racial_paradigm_levels {5,1,1,1,1}, ...). Their starting
-- inventory, the starter pistol included, is in
-- db/sgw/Inventory/Seed/sgw_inventory.sql.
--
-- Do not hand-tune one row: the live-DB guard
-- character_create::seed_parity_live_db_tests creates a fresh char_def-3
-- character through the real handler and fails if any seeded row (names,
-- ids and account aside) or its inventory differs. To change what a seeded
-- character starts with, change character creation and its seeds
-- (char_creation*, in db/resources/Archetypes/Seed/) and copy the result
-- here.
--
-- Spawn: the Praxis start, Castle_CellBlock (-334.231, 73.472, -228.026),
-- where a new character starts and the Cellblock tutorial (mission 622)
-- begins.
--
-- training_points and applied_science_points follow the v2 economy
-- (D-AT02): 1 of each at level 1, as createCharacter binds them.
--

INSERT INTO sgw_player (account_id, player_id, player_name, extra_name, alignment, archetype, gender, world_location, bodyset, level, title, pos_x, pos_y, pos_z, skin_color_id, components, world_id, abilities, access_level, training_points, applied_science_points) VALUES
    (2, 62, 'Test Soldier', '', 1, 2, 1, 'Castle_CellBlock', 'BS_HumanMale.BS_HumanMale', 1, 0, -334.231, 73.472, -228.026, 0, ARRAY['BS_HumanMale.BS_HM_Torso_00','BS_HumanMale.BS_HM_Legs_00','BS_HumanMale.BS_HM_Head_00','BS_HumanMale.BS_HM_Hands_00','BS_HumanMale.BS_HM_Boots_00','BS_HumanMale.BS_HM_Hair_01','ACC_HumanMale.ACC_HM_FaceScar_01','BS_HumanMale.BS_HM_FaceHair_01','BS_HumanMale.BS_HM_FacePaint_01']::character varying(200)[], (SELECT world_id FROM resources.worlds WHERE world = 'Castle_CellBlock'), ARRAY[592,594,597,1218,1646]::integer[], 2, 1, 1),
    (3, 63, 'cady', '', 1, 2, 1, 'Castle_CellBlock', 'BS_HumanMale.BS_HumanMale', 1, 0, -334.231, 73.472, -228.026, 0, ARRAY['BS_HumanMale.BS_HM_Torso_00','BS_HumanMale.BS_HM_Legs_00','BS_HumanMale.BS_HM_Head_00','BS_HumanMale.BS_HM_Hands_00','BS_HumanMale.BS_HM_Boots_00','BS_HumanMale.BS_HM_Hair_01','ACC_HumanMale.ACC_HM_FaceScar_01','BS_HumanMale.BS_HM_FaceHair_01','BS_HumanMale.BS_HM_FacePaint_01']::character varying(200)[], (SELECT world_id FROM resources.worlds WHERE world = 'Castle_CellBlock'), ARRAY[592,594,597,1218,1646]::integer[], 2, 1, 1),
    (4, 64, 'jorsh', '', 1, 2, 1, 'Castle_CellBlock', 'BS_HumanMale.BS_HumanMale', 1, 0, -334.231, 73.472, -228.026, 0, ARRAY['BS_HumanMale.BS_HM_Torso_00','BS_HumanMale.BS_HM_Legs_00','BS_HumanMale.BS_HM_Head_00','BS_HumanMale.BS_HM_Hands_00','BS_HumanMale.BS_HM_Boots_00','BS_HumanMale.BS_HM_Hair_01','ACC_HumanMale.ACC_HM_FaceScar_01','BS_HumanMale.BS_HM_FaceHair_01','BS_HumanMale.BS_HM_FacePaint_01']::character varying(200)[], (SELECT world_id FROM resources.worlds WHERE world = 'Castle_CellBlock'), ARRAY[592,594,597,1218,1646]::integer[], 2, 1, 1),
    (5, 65, 'cake', '', 1, 2, 1, 'Castle_CellBlock', 'BS_HumanMale.BS_HumanMale', 1, 0, -334.231, 73.472, -228.026, 0, ARRAY['BS_HumanMale.BS_HM_Torso_00','BS_HumanMale.BS_HM_Legs_00','BS_HumanMale.BS_HM_Head_00','BS_HumanMale.BS_HM_Hands_00','BS_HumanMale.BS_HM_Boots_00','BS_HumanMale.BS_HM_Hair_01','ACC_HumanMale.ACC_HM_FaceScar_01','BS_HumanMale.BS_HM_FaceHair_01','BS_HumanMale.BS_HM_FacePaint_01']::character varying(200)[], (SELECT world_id FROM resources.worlds WHERE world = 'Castle_CellBlock'), ARRAY[592,594,597,1218,1646]::integer[], 2, 1, 1),
    (6, 66, 'lomiada1', '', 1, 2, 1, 'Castle_CellBlock', 'BS_HumanMale.BS_HumanMale', 1, 0, -334.231, 73.472, -228.026, 0, ARRAY['BS_HumanMale.BS_HM_Torso_00','BS_HumanMale.BS_HM_Legs_00','BS_HumanMale.BS_HM_Head_00','BS_HumanMale.BS_HM_Hands_00','BS_HumanMale.BS_HM_Boots_00','BS_HumanMale.BS_HM_Hair_01','ACC_HumanMale.ACC_HM_FaceScar_01','BS_HumanMale.BS_HM_FaceHair_01','BS_HumanMale.BS_HM_FacePaint_01']::character varying(200)[], (SELECT world_id FROM resources.worlds WHERE world = 'Castle_CellBlock'), ARRAY[592,594,597,1218,1646]::integer[], 2, 1, 1),
    (7, 67, 'nonwo1984', '', 1, 2, 1, 'Castle_CellBlock', 'BS_HumanMale.BS_HumanMale', 1, 0, -334.231, 73.472, -228.026, 0, ARRAY['BS_HumanMale.BS_HM_Torso_00','BS_HumanMale.BS_HM_Legs_00','BS_HumanMale.BS_HM_Head_00','BS_HumanMale.BS_HM_Hands_00','BS_HumanMale.BS_HM_Boots_00','BS_HumanMale.BS_HM_Hair_01','ACC_HumanMale.ACC_HM_FaceScar_01','BS_HumanMale.BS_HM_FaceHair_01','BS_HumanMale.BS_HM_FacePaint_01']::character varying(200)[], (SELECT world_id FROM resources.worlds WHERE world = 'Castle_CellBlock'), ARRAY[592,594,597,1218,1646]::integer[], 2, 1, 1),
    (8, 68, 'ishido972', '', 1, 2, 1, 'Castle_CellBlock', 'BS_HumanMale.BS_HumanMale', 1, 0, -334.231, 73.472, -228.026, 0, ARRAY['BS_HumanMale.BS_HM_Torso_00','BS_HumanMale.BS_HM_Legs_00','BS_HumanMale.BS_HM_Head_00','BS_HumanMale.BS_HM_Hands_00','BS_HumanMale.BS_HM_Boots_00','BS_HumanMale.BS_HM_Hair_01','ACC_HumanMale.ACC_HM_FaceScar_01','BS_HumanMale.BS_HM_FaceHair_01','BS_HumanMale.BS_HM_FacePaint_01']::character varying(200)[], (SELECT world_id FROM resources.worlds WHERE world = 'Castle_CellBlock'), ARRAY[592,594,597,1218,1646]::integer[], 2, 1, 1),
    (9, 69, 'Friendly', '', 1, 2, 1, 'Castle_CellBlock', 'BS_HumanMale.BS_HumanMale', 1, 0, -334.231, 73.472, -228.026, 0, ARRAY['BS_HumanMale.BS_HM_Torso_00','BS_HumanMale.BS_HM_Legs_00','BS_HumanMale.BS_HM_Head_00','BS_HumanMale.BS_HM_Hands_00','BS_HumanMale.BS_HM_Boots_00','BS_HumanMale.BS_HM_Hair_01','ACC_HumanMale.ACC_HM_FaceScar_01','BS_HumanMale.BS_HM_FaceHair_01','BS_HumanMale.BS_HM_FacePaint_01']::character varying(200)[], (SELECT world_id FROM resources.worlds WHERE world = 'Castle_CellBlock'), ARRAY[592,594,597,1218,1646]::integer[], 2, 1, 1),
    (9, 70, 'Annoying', '', 1, 2, 1, 'Castle_CellBlock', 'BS_HumanMale.BS_HumanMale', 1, 0, -334.231, 73.472, -228.026, 0, ARRAY['BS_HumanMale.BS_HM_Torso_00','BS_HumanMale.BS_HM_Legs_00','BS_HumanMale.BS_HM_Head_00','BS_HumanMale.BS_HM_Hands_00','BS_HumanMale.BS_HM_Boots_00','BS_HumanMale.BS_HM_Hair_01','ACC_HumanMale.ACC_HM_FaceScar_01','BS_HumanMale.BS_HM_FaceHair_01','BS_HumanMale.BS_HM_FacePaint_01']::character varying(200)[], (SELECT world_id FROM resources.worlds WHERE world = 'Castle_CellBlock'), ARRAY[592,594,597,1218,1646]::integer[], 2, 1, 1);

--
-- TOC entry 2710 (class 0 OID 0)
-- Dependencies: 271
-- Name: sgw_characters_character_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('sgw_characters_character_id_seq', 70, true);
