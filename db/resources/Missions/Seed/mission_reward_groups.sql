--
-- TOC entry 3212 (class 0 OID 63004)
-- Dependencies: 229
-- Data for Name: mission_reward_groups; Type: TABLE DATA; Schema: resources; Owner: -
--

INSERT INTO mission_reward_groups (mission_id, group_id, choices) VALUES (1001, 1, 2);

INSERT INTO mission_reward_groups (mission_id, group_id, choices) VALUES (1001, 2, 1);

INSERT INTO mission_reward_groups (mission_id, group_id, choices) VALUES (40, 6, 6);

--
-- TOC entry 3325 (class 0 OID 0)
-- Dependencies: 230
-- Name: mission_reward_groups_group_id_seq; Type: SEQUENCE SET; Schema: resources; Owner: -
--

SELECT pg_catalog.setval('mission_reward_groups_group_id_seq', 7, true);

