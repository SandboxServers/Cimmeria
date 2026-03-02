--
-- TOC entry 3167 (class 0 OID 62794)
-- Dependencies: 184
-- Data for Name: ability_sets; Type: TABLE DATA; Schema: resources; Owner: -
--

INSERT INTO ability_sets (ability_set_id, description) VALUES (1, 'NID guard (pistol) ability set');

INSERT INTO ability_sets (ability_set_id, description) VALUES (2, 'Prisoner retrieval unit ability set');

INSERT INTO ability_sets (ability_set_id, description) VALUES (3, 'NID guard (SMG) ability set');

--
-- TOC entry 3306 (class 0 OID 0)
-- Dependencies: 185
-- Name: ability_sets2_ability_set_id_seq; Type: SEQUENCE SET; Schema: resources; Owner: -
--

SELECT pg_catalog.setval('ability_sets2_ability_set_id_seq', 3, true);

