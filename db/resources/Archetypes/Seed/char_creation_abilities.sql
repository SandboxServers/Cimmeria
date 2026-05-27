--
-- TOC entry 3178 (class 0 OID 62841)
-- Dependencies: 195
-- Data for Name: char_creation_abilities; Type: TABLE DATA; Schema: resources; Owner: -
--

INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (3, 597);

INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (4, 597);

INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (13, 597);

INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (14, 597);

INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (3, 592);

INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (4, 592);

INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (13, 592);

INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (14, 592);

INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (3, 594);

INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (4, 594);

INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (13, 594);

INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (14, 594);

INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (1, 592);

INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (2, 592);

INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (11, 592);

INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (12, 592);

INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (1, 594);

INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (2, 594);

INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (11, 594);

INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (12, 594);

INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (1, 597);

INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (2, 597);

INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (11, 597);

INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (12, 597);


-- Starter abilities for the 15 char_def_ids that were shipping with EMPTY
-- ability lists. Before this change only char_def_ids 1-4 and 11-14
-- (Soldier M/F and Commando M/F across both alignments) had starter
-- abilities. The 15 covered here are Scientist, Archeologist, Asgard,
-- Goa'uld, Sholva, and Jaffa — note that Asgard is M-only (SGU) and
-- Goa'uld/Jaffa are Praxis-only, matching the char_creation seed shape.
-- Baseline matches Soldier/Commando: 592 Pistol Shot + 594 Strike + 597
-- Heal Focus. See db/scripts/seed_starter_abilities_all_archetypes.sql
-- for the post-deploy migration that lifts these into deployed DBs.

INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (5, 592);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (5, 594);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (5, 597);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (6, 592);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (6, 594);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (6, 597);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (7, 592);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (7, 594);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (7, 597);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (8, 592);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (8, 594);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (8, 597);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (9, 592);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (9, 594);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (9, 597);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (10, 592);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (10, 594);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (10, 597);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (15, 592);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (15, 594);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (15, 597);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (16, 592);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (16, 594);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (16, 597);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (17, 592);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (17, 594);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (17, 597);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (18, 592);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (18, 594);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (18, 597);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (19, 592);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (19, 594);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (19, 597);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (20, 592);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (20, 594);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (20, 597);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (21, 592);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (21, 594);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (21, 597);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (22, 592);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (22, 594);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (22, 597);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (23, 592);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (23, 594);
INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (23, 597);
