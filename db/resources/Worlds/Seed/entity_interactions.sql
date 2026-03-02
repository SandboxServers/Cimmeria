--
-- TOC entry 3190 (class 0 OID 62896)
-- Dependencies: 207
-- Data for Name: entity_interactions; Type: TABLE DATA; Schema: resources; Owner: -
--

INSERT INTO entity_interactions (entity_interaction_id, template_id, interaction_set_map_id, min_level, factions, alignments, missions_completed, missions_not_accepted) VALUES (35, 43, 3127, 1, '{}', '{}', '{}', '{742}');

--
-- TOC entry 3315 (class 0 OID 0)
-- Dependencies: 208
-- Name: entity_interactions_entity_interaction_id_seq; Type: SEQUENCE SET; Schema: resources; Owner: -
--

SELECT pg_catalog.setval('entity_interactions_entity_interaction_id_seq', 35, true);

