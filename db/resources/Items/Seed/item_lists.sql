--
-- TOC entry 3204 (class 0 OID 62963)
-- Dependencies: 221
-- Data for Name: item_lists; Type: TABLE DATA; Schema: resources; Owner: -
--

INSERT INTO item_lists (item_list_id, name) VALUES (2, 'Test vendor sell/repair/recharge list');

INSERT INTO item_lists (item_list_id, name) VALUES (1, 'Test vendor buy list');

-- NEW CONTENT (debug hub, crafting): the crafting supplies vendor's buy list
--   (template 314), in the crafting campaign's 310 id block.
INSERT INTO item_lists (item_list_id, name) VALUES (310, 'Debug hub crafting supplies');

--
-- TOC entry 3320 (class 0 OID 0)
-- Dependencies: 222
-- Name: item_lists_item_list_id_seq; Type: SEQUENCE SET; Schema: resources; Owner: -
--

-- Past every seeded row and the end of the crafting campaign's list block
-- (310-329), and never lowered, so a list created without an id never takes a
-- seeded or reserved one. Raise the floor when a block is reserved above it;
-- live_db_seed_sequences.rs guards it.
SELECT pg_catalog.setval('item_lists_item_list_id_seq', GREATEST((SELECT MAX(item_list_id) FROM item_lists), (SELECT last_value FROM item_lists_item_list_id_seq), 329), true);

