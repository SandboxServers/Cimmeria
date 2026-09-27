--
-- TOC entry 3207 (class 0 OID 62985)
-- Dependencies: 224
-- Data for Name: loot; Type: TABLE DATA; Schema: resources; Owner: -
--

INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (10, 1, 3730, 1, 1, 1);

INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (11, 1, 55, 1, 1, 1);

INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (12, 2, NULL, 5, 0.800000012, 50);

-- Health Slappack TC1 (item 2893, +500 HP) as a guaranteed drop on the
-- shared Cellblock guard table. Pairs with the out-of-combat regen tick:
-- regen restores HP between fights, slappacks cover the burst recovery
-- for back-to-back encounters where regen alone can't keep up.
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (13, 2, 2893, 1, 1, 1);

-- NEW CONTENT (debug hub): table 3, the stasis-room loot crate. Every row is
-- probability 1: a roll that drops nothing never sets INT_NormalLoot, and the
-- corpse is then unclickable, which would read as a broken loot path. The
-- mix covers each branch Loot All walks: a stackable consumable with a
-- quantity range (Health Slappack TC1, max stack 10), two single items
-- (Processor (Electronics), Cell (Bio-Medical)) and naquadah (design_id
-- NULL). 5192 is also the item cost of vendor list 1's second row.
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (14, 3, 2893, 2, 1, 3);

INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (15, 3, 5228, 1, 1, 1);

INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (16, 3, 5192, 1, 1, 1);

INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (17, 3, NULL, 25, 1, 75);

--
-- TOC entry 3323 (class 0 OID 0)
-- Dependencies: 225
-- Name: loot_loot_id_seq; Type: SEQUENCE SET; Schema: resources; Owner: -
--

SELECT pg_catalog.setval('loot_loot_id_seq', 17, true);

