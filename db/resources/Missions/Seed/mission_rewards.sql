--
-- TOC entry 3251 (class 0 OID 64130)
-- Dependencies: 302
-- Data for Name: mission_rewards; Type: TABLE DATA; Schema: resources; Owner: -
--

INSERT INTO mission_rewards (reward_id, group_id, item_id) VALUES (1, 1, 5620);

INSERT INTO mission_rewards (reward_id, group_id, item_id) VALUES (2, 1, 5621);

INSERT INTO mission_rewards (reward_id, group_id, item_id) VALUES (3, 1, 5622);

INSERT INTO mission_rewards (reward_id, group_id, item_id) VALUES (4, 2, 3189);

INSERT INTO mission_rewards (reward_id, group_id, item_id) VALUES (5, 2, 3210);

INSERT INTO mission_rewards (reward_id, group_id, item_id) VALUES (6, 6, 3054);

INSERT INTO mission_rewards (reward_id, group_id, item_id) VALUES (7, 6, 2907);

INSERT INTO mission_rewards (reward_id, group_id, item_id) VALUES (8, 6, 2611);

--
-- TOC entry 3326 (class 0 OID 0)
-- Dependencies: 301
-- Name: mission_rewards_reward_id_seq; Type: SEQUENCE SET; Schema: resources; Owner: -
--

SELECT pg_catalog.setval('mission_rewards_reward_id_seq', 11, true);

