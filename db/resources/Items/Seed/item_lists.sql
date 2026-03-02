--
-- TOC entry 3204 (class 0 OID 62963)
-- Dependencies: 221
-- Data for Name: item_lists; Type: TABLE DATA; Schema: resources; Owner: -
--

INSERT INTO item_lists (item_list_id, name) VALUES (2, 'Test vendor sell/repair/recharge list');

INSERT INTO item_lists (item_list_id, name) VALUES (1, 'Test vendor buy list');

--
-- TOC entry 3320 (class 0 OID 0)
-- Dependencies: 222
-- Name: item_lists_item_list_id_seq; Type: SEQUENCE SET; Schema: resources; Owner: -
--

SELECT pg_catalog.setval('item_lists_item_list_id_seq', 2, true);

