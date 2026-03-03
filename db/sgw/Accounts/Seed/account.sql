--
-- TOC entry 2688 (class 0 OID 63748)
-- Dependencies: 268
-- Data for Name: account; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO account (account_id, account_name, password, accesslevel, enabled) VALUES (2, 'test', 'a94a8fe5ccb19ba61c4c0873d391e987982fbbd3', 1, true);

--
-- TOC entry 2709 (class 0 OID 0)
-- Dependencies: 269
-- Name: accounts_account_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('accounts_account_id_seq', 2, true);

