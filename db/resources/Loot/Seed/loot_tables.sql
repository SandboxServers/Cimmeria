--
-- TOC entry 3209 (class 0 OID 62992)
-- Dependencies: 226
-- Data for Name: loot_tables; Type: TABLE DATA; Schema: resources; Owner: -
--

INSERT INTO loot_tables (loot_table_id, description) VALUES (1, 'Cpl. Frost body - DEPRECATED');

INSERT INTO loot_tables (loot_table_id, description) VALUES (2, 'Cellblock NID guard default');

-- NEW CONTENT (debug hub): the stasis-room loot crate (template 304). Opened
-- by chain 7020's `open_loot` (no kill), re-rolled on every open. #1026 D-AM06
-- adds the special ammo, a pistol and an SMG to this table later.
INSERT INTO loot_tables (loot_table_id, description) VALUES (3, 'Debug hub loot crate');

-- NEW CONTENT (Castle population, docs/analysis/castle-population/README.md,
-- D-CP09): Castle hostile drops. Rows and rates are in loot.sql.
INSERT INTO loot_tables (loot_table_id, description) VALUES (4, 'Castle NID guard');

INSERT INTO loot_tables (loot_table_id, description) VALUES (5, 'Castle NID veteran');

INSERT INTO loot_tables (loot_table_id, description) VALUES (6, 'Castle PRU salvage');

-- Decision (@Cadacious, 2026-09-28): the NID guards in the Castle hall before
-- the Interrogation Block roll this table instead of 4/5, bound per spawn
-- (spawnlist.loot_table_id) because templates 148/181/182/183 are shared
-- with the rest of the Castle. Richer than table 4 so a player who dies
-- there on the way to Romney has something to recover with.
INSERT INTO loot_tables (loot_table_id, description) VALUES (7, 'Castle hall NID guard');

-- Decision (@Cadacious, 2026-09-28): live containers opened by the content
-- `open_loot` action, every row at probability 1. 8/9 are the Castle chest
-- before the Interrogation Block (chains 1274/1275), 10/11 the Cellblock
-- weapon/armor crate (chains 1098/1099).
INSERT INTO loot_tables (loot_table_id, description) VALUES (8, 'Castle pre-Romney chest (non-Jaffa)');
INSERT INTO loot_tables (loot_table_id, description) VALUES (9, 'Castle pre-Romney chest (Jaffa)');
INSERT INTO loot_tables (loot_table_id, description) VALUES (10, 'Cellblock crate (non-Jaffa)');
INSERT INTO loot_tables (loot_table_id, description) VALUES (11, 'Cellblock crate (Jaffa)');

--
-- TOC entry 3324 (class 0 OID 0)
-- Dependencies: 227
-- Name: loot_tables_loot_table_id_seq; Type: SEQUENCE SET; Schema: resources; Owner: -
--

SELECT pg_catalog.setval('loot_tables_loot_table_id_seq', 11, true);

