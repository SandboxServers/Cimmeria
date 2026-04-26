--
-- Starter equipment for the bundled test character (sgw_player.player_id 62).
-- These items come from resources.char_creation char_def_id 2 first-choice
-- visual groups and are placed using the same equipment containers as Account.py.
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
    (10235, 1, 0, 7,  0, -1, 4336, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 62, false, 0),
    (10236, 1, 0, 11, 0, -1, 4332, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 62, false, 0),
    (10237, 1, 0, 12, 0, -1, 4330, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 62, false, 0),
    (10238, 1, 0, 5,  0, -1, 3497, 0, 'AMMO_NONE', ARRAY[]::resources."EAmmoType"[], 62, false, 0);

--
-- TOC entry 2713 (class 0 OID 0)
-- Dependencies: 277
-- Name: sgw_inventory_item_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('sgw_inventory_item_id_seq', 10238, true);
