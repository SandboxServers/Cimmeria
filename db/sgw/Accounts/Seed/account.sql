--
-- TOC entry 2688 (class 0 OID 63748)
-- Dependencies: 268
-- Data for Name: account; Type: TABLE DATA; Schema: public; Owner: -
--

-- Seed/dev accounts are GameMaster (accesslevel 2) so they can use GM
-- commands on a dev shard. AccessLevel: 0=Player, 1=Moderator,
-- 2=GameMaster, 3=Admin, 4=Developer (cimmeria_commands::permissions).
-- 2 clears the GM threshold (the cell gm_gate requires >= GameMaster and
-- the chat-command path gates GM commands at GameMaster) without granting
-- Admin-only commands (e.g. /shutdown). Bump an individual account to 3/4
-- locally if you need Admin/Developer-tier commands. (#64)
INSERT INTO account (account_id, account_name, password, accesslevel, enabled) VALUES (2, 'test',     'a94a8fe5ccb19ba61c4c0873d391e987982fbbd3', 2, true);
INSERT INTO account (account_id, account_name, password, accesslevel, enabled) VALUES (3, 'cady',     'a94a8fe5ccb19ba61c4c0873d391e987982fbbd3', 2, true);
INSERT INTO account (account_id, account_name, password, accesslevel, enabled) VALUES (4, 'jorsh',    'a94a8fe5ccb19ba61c4c0873d391e987982fbbd3', 2, true);
INSERT INTO account (account_id, account_name, password, accesslevel, enabled) VALUES (5, 'cake',     'a94a8fe5ccb19ba61c4c0873d391e987982fbbd3', 2, true);
INSERT INTO account (account_id, account_name, password, accesslevel, enabled) VALUES (6, 'lomiada1', 'a94a8fe5ccb19ba61c4c0873d391e987982fbbd3', 2, true);
INSERT INTO account (account_id, account_name, password, accesslevel, enabled) VALUES (7, 'nonwo1984','a94a8fe5ccb19ba61c4c0873d391e987982fbbd3', 2, true);
INSERT INTO account (account_id, account_name, password, accesslevel, enabled) VALUES (8, 'ishido972','a94a8fe5ccb19ba61c4c0873d391e987982fbbd3', 2, true);

--
-- TOC entry 2709 (class 0 OID 0)
-- Dependencies: 269
-- Name: accounts_account_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('accounts_account_id_seq', 8, true);

