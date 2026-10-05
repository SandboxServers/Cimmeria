--
-- Starting inventory of the seeded playtest characters (player ids 62-70,
-- db/sgw/Players/Seed/sgw_player.sql): what createCharacter gives a Praxis
-- Commando (char_def 3) with the first choice in every optional visual group.
--
-- In placement order, per character:
--   3440 Prison Jacket   chest (7)      visual group 29, forced
--   3437 prison legs     legs (11)      visual group 31, forced
--   3438 Prison Boots    feet (12)      visual group 36, forced
--   3497 Eyeglasses      face (5)       visual group 38, first choice
--   4343 Accessory       backpack (1)   visual group 40, first choice; face
--                                       is taken, so it overflows to main
--   55   SI 3 9mm Pistol bandolier (3)  char_creation_items starter pistol,
--                                       slot 0 (the active slot), magazine
--                                       loaded: ammo = clip_size = 15
--
-- The clothing keeps its char_creation_choices durability (-1) and bound
-- flag. The pistol row is written the way the item-grant path writes one:
-- durability 100, ammo_type / ammo_types from the design (default
-- Bullet_Default). ammo_types is read from resources.items rather than
-- spelled out, because the ammo seeds widen item 55's list (special rounds).
--
-- character_create::seed_parity_live_db_tests compares these rows with a
-- freshly created character's (instance item_id aside).
--

INSERT INTO sgw_inventory (
    item_id,
    stack_size,
    charges,
    container_id,
    slot_id,
    durability,
    type_id,
    flags,
    ammo_type,
    ammo_types,
    character_id,
    bound,
    ammo
) VALUES
    (10235, 1, 0, 7,  0, -1, 3440, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 62, false, 0),
    (10236, 1, 0, 11, 0, -1, 3437, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 62, false, 0),
    (10237, 1, 0, 12, 0, -1, 3438, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 62, false, 0),
    (10238, 1, 0, 5,  0, -1, 3497, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 62, false, 0),
    (10239, 1, 0, 1,  0, -1, 4343, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 62, false, 0),
    (10240, 1, 0, 3,  0, 100, 55, 0, 'Bullet_Default', (SELECT ri.ammo_types FROM resources.items ri WHERE ri.item_id = 55), 62, false, 15),
    (10241, 1, 0, 7,  0, -1, 3440, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 63, false, 0),
    (10242, 1, 0, 11, 0, -1, 3437, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 63, false, 0),
    (10243, 1, 0, 12, 0, -1, 3438, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 63, false, 0),
    (10244, 1, 0, 5,  0, -1, 3497, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 63, false, 0),
    (10245, 1, 0, 1,  0, -1, 4343, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 63, false, 0),
    (10246, 1, 0, 3,  0, 100, 55, 0, 'Bullet_Default', (SELECT ri.ammo_types FROM resources.items ri WHERE ri.item_id = 55), 63, false, 15),
    (10247, 1, 0, 7,  0, -1, 3440, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 64, false, 0),
    (10248, 1, 0, 11, 0, -1, 3437, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 64, false, 0),
    (10249, 1, 0, 12, 0, -1, 3438, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 64, false, 0),
    (10250, 1, 0, 5,  0, -1, 3497, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 64, false, 0),
    (10251, 1, 0, 1,  0, -1, 4343, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 64, false, 0),
    (10252, 1, 0, 3,  0, 100, 55, 0, 'Bullet_Default', (SELECT ri.ammo_types FROM resources.items ri WHERE ri.item_id = 55), 64, false, 15),
    (10253, 1, 0, 7,  0, -1, 3440, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 65, false, 0),
    (10254, 1, 0, 11, 0, -1, 3437, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 65, false, 0),
    (10255, 1, 0, 12, 0, -1, 3438, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 65, false, 0),
    (10256, 1, 0, 5,  0, -1, 3497, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 65, false, 0),
    (10257, 1, 0, 1,  0, -1, 4343, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 65, false, 0),
    (10258, 1, 0, 3,  0, 100, 55, 0, 'Bullet_Default', (SELECT ri.ammo_types FROM resources.items ri WHERE ri.item_id = 55), 65, false, 15),
    (10259, 1, 0, 7,  0, -1, 3440, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 66, false, 0),
    (10260, 1, 0, 11, 0, -1, 3437, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 66, false, 0),
    (10261, 1, 0, 12, 0, -1, 3438, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 66, false, 0),
    (10262, 1, 0, 5,  0, -1, 3497, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 66, false, 0),
    (10263, 1, 0, 1,  0, -1, 4343, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 66, false, 0),
    (10264, 1, 0, 3,  0, 100, 55, 0, 'Bullet_Default', (SELECT ri.ammo_types FROM resources.items ri WHERE ri.item_id = 55), 66, false, 15),
    (10265, 1, 0, 7,  0, -1, 3440, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 67, false, 0),
    (10266, 1, 0, 11, 0, -1, 3437, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 67, false, 0),
    (10267, 1, 0, 12, 0, -1, 3438, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 67, false, 0),
    (10268, 1, 0, 5,  0, -1, 3497, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 67, false, 0),
    (10269, 1, 0, 1,  0, -1, 4343, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 67, false, 0),
    (10270, 1, 0, 3,  0, 100, 55, 0, 'Bullet_Default', (SELECT ri.ammo_types FROM resources.items ri WHERE ri.item_id = 55), 67, false, 15),
    (10271, 1, 0, 7,  0, -1, 3440, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 68, false, 0),
    (10272, 1, 0, 11, 0, -1, 3437, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 68, false, 0),
    (10273, 1, 0, 12, 0, -1, 3438, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 68, false, 0),
    (10274, 1, 0, 5,  0, -1, 3497, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 68, false, 0),
    (10275, 1, 0, 1,  0, -1, 4343, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 68, false, 0),
    (10276, 1, 0, 3,  0, 100, 55, 0, 'Bullet_Default', (SELECT ri.ammo_types FROM resources.items ri WHERE ri.item_id = 55), 68, false, 15),
    (10277, 1, 0, 7,  0, -1, 3440, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 69, false, 0),
    (10278, 1, 0, 11, 0, -1, 3437, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 69, false, 0),
    (10279, 1, 0, 12, 0, -1, 3438, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 69, false, 0),
    (10280, 1, 0, 5,  0, -1, 3497, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 69, false, 0),
    (10281, 1, 0, 1,  0, -1, 4343, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 69, false, 0),
    (10282, 1, 0, 3,  0, 100, 55, 0, 'Bullet_Default', (SELECT ri.ammo_types FROM resources.items ri WHERE ri.item_id = 55), 69, false, 15),
    (10283, 1, 0, 7,  0, -1, 3440, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 70, false, 0),
    (10284, 1, 0, 11, 0, -1, 3437, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 70, false, 0),
    (10285, 1, 0, 12, 0, -1, 3438, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 70, false, 0),
    (10286, 1, 0, 5,  0, -1, 3497, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 70, false, 0),
    (10287, 1, 0, 1,  0, -1, 4343, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 70, false, 0),
    (10288, 1, 0, 3,  0, 100, 55, 0, 'Bullet_Default', (SELECT ri.ammo_types FROM resources.items ri WHERE ri.item_id = 55), 70, false, 15);

--
-- TOC entry 2713 (class 0 OID 0)
-- Dependencies: 277
-- Name: sgw_inventory_item_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('sgw_inventory_item_id_seq', 10288, true);
