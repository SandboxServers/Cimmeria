--
-- TOC entry 3209 (class 0 OID 62992)
-- Dependencies: 226
-- Data for Name: loot_tables; Type: TABLE DATA; Schema: resources; Owner: -
--

INSERT INTO loot_tables (loot_table_id, description) VALUES (1, 'Cpl. Frost body - DEPRECATED');

INSERT INTO loot_tables (loot_table_id, description) VALUES (2, 'Cellblock NID guard default');

-- NEW CONTENT (debug hub): the stasis-room loot crate (template 304).
INSERT INTO loot_tables (loot_table_id, description) VALUES (3, 'Debug hub loot crate');

-- NEW CONTENT (Castle population, docs/analysis/castle-population/README.md,
-- D-CP09): Castle hostile drops. Rows and rates are in loot.sql.
INSERT INTO loot_tables (loot_table_id, description) VALUES (4, 'Castle NID guard');

INSERT INTO loot_tables (loot_table_id, description) VALUES (5, 'Castle NID veteran');

INSERT INTO loot_tables (loot_table_id, description) VALUES (6, 'Castle PRU salvage');

--
-- TOC entry 3324 (class 0 OID 0)
-- Dependencies: 227
-- Name: loot_tables_loot_table_id_seq; Type: SEQUENCE SET; Schema: resources; Owner: -
--

SELECT pg_catalog.setval('loot_tables_loot_table_id_seq', 6, true);

