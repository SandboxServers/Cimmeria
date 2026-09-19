--
-- TOC entry 3167 (class 0 OID 62794)
-- Dependencies: 184
-- Data for Name: ability_sets; Type: TABLE DATA; Schema: resources; Owner: -
--

INSERT INTO ability_sets (ability_set_id, description) VALUES (1, 'NID guard (pistol) ability set');

INSERT INTO ability_sets (ability_set_id, description) VALUES (2, 'Prisoner retrieval unit ability set');

INSERT INTO ability_sets (ability_set_id, description) VALUES (3, 'NID guard (SMG) ability set');

-- Harset rebuild packet H11 (defect H-B8): before these two rows, every
-- template without an `ability_set_id` fell back to `NPC_DEFAULT_ABILITY = 592`
-- (Pistol Shot, crates/services/src/cell/combat/threat/aggro.rs:19) -- Jaffa and
-- Goa'uld NPCs firing a Tau'ri pistol.
--
-- H11 gave each of these sets exactly one ability, because
-- `ability_set_abilities` then had `PRIMARY KEY (ability_set_id)` and
-- db/database.sql applies the primary keys before the seed data, so a second row
-- for the same set aborted the whole load. **Packet H09 widened that key to
-- `(ability_set_id, ability_id)`**, and sets 4 and 5 now carry two abilities
-- each. The one-row cap is gone; see
-- docs/analysis/harset-rebuild/worknotes/H09.md.
--
-- Membership rule, unchanged by H09: an ability needs a non-NULL `event_set_id`,
-- which is what gates the Ability_Begin/Ability_End `onSequence` broadcast
-- (crates/services/src/cell/abilities/use_ability/handle.rs:524). An ability with
-- a NULL one deals damage and plays no animation -- which rules out 594 Strike,
-- 540 Staff Strike, 479 Staff Blast, 1482 Ground Blast and 1768 Double Blast.
--
-- That rule is far more restrictive than it looks: only 35 of the 1,886 rows in
-- `abilities` have a non-NULL `event_set_id`, and every one of the 35 is an auto
-- attack or a melee auto attack. There is no animatable "special" staff or
-- ribbon ability anywhere in the seed, so a set's real membership is the
-- ranged/melee auto-attack pair that `items_event_sets` binds to the weapon --
-- not a spell list. H09 checked; the H09 worknote records the query.

INSERT INTO ability_sets (ability_set_id, description) VALUES (4, 'Jaffa staff ability set');

INSERT INTO ability_sets (ability_set_id, description) VALUES (5, 'Goa''uld ribbon device ability set');

--
-- TOC entry 3306 (class 0 OID 0)
-- Dependencies: 185
-- Name: ability_sets2_ability_set_id_seq; Type: SEQUENCE SET; Schema: resources; Owner: -
--

SELECT pg_catalog.setval('ability_sets2_ability_set_id_seq', 5, true);

