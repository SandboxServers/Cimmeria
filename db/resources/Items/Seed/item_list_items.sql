--
-- TOC entry 3201 (class 0 OID 62955)
-- Dependencies: 218
-- Data for Name: item_list_items; Type: TABLE DATA; Schema: resources; Owner: -
--

INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (1, 1, 5228, 1, 100);

INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (3, 1, 5192, 1, 0);

INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (4, 2, 21, 1, 1000);

INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (6, 2, 3437, 1, 100);

INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (5, 2, 55, 1, 300);

INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (7, 1, 3241, 1, 50);

--
-- TOC entry 3319 (class 0 OID 0)
-- Dependencies: 219
-- Name: item_list_items_item_id_seq; Type: SEQUENCE SET; Schema: resources; Owner: -
--

SELECT pg_catalog.setval('item_list_items_item_id_seq', 7, true);

