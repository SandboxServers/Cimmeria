--
-- Playable default character for the bundled test account.
-- Values are derived from resources.char_creation char_def_id 2
-- (SGU male Soldier) and its first-choice visual groups.
--

INSERT INTO sgw_player (
    account_id,
    player_id,
    level,
    alignment,
    archetype,
    gender,
    player_name,
    extra_name,
    world_location,
    bodyset,
    title,
    pos_x,
    pos_y,
    pos_z,
    heading,
    naquadah,
    exp,
    first_login,
    world_id,
    known_stargates,
    components,
    abilities,
    access_level,
    skin_color_id,
    bandolier_slot,
    interaction_maps,
    training_points,
    discipline_ids,
    racial_paradigm_levels,
    applied_science_points,
    blueprint_ids,
    known_respawners
) VALUES (
    2,
    62,
    1,
    2,
    1,
    1,
    'Test Soldier',
    '',
    'SGC_W1',
    'BS_HumanMale.BS_HumanMale',
    0,
    201.5,
    1.31,
    49.724,
    0,
    0,
    0,
    1,
    58,
    ARRAY[2,3,4,5,6,7,8,10,15,20,22,23,25,27]::integer[],
    ARRAY[
        'BS_HumanMale.BS_HM_Torso_00',
        'BS_HumanMale.BS_HM_Legs_00',
        'BS_HumanMale.BS_HM_Head_00',
        'BS_HumanMale.BS_HM_Hands_00',
        'BS_HumanMale.BS_HM_Boots_00',
        'BS_HumanMale.BS_HM_Hair_01',
        'ACC_HumanMale.ACC_HM_FaceScar_01',
        'BS_HumanMale.BS_HM_FaceHair_01',
        'BS_HumanMale.BS_HM_FacePaint_01'
    ]::character varying(200)[],
    ARRAY[592,594,597]::integer[],
    0,
    0,
    0,
    NULL,
    0,
    ARRAY[]::integer[],
    ARRAY[]::integer[],
    0,
    ARRAY[]::integer[],
    ARRAY[]::integer[]
);

--
-- TOC entry 2710 (class 0 OID 0)
-- Dependencies: 271
-- Name: sgw_characters_character_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('sgw_characters_character_id_seq', 62, true);
