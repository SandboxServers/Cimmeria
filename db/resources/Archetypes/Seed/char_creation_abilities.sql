--
-- Data for Name: char_creation_abilities; Type: TABLE DATA; Schema: resources; Owner: -
--
-- Starter abilities per start profile (Class Start v6, CS-02). The universal
-- spawn kit (592 Pistol Shot, 594 Strike, 597 Heal Focus, 1218 Recuperation,
-- 1646 Health Heal on all 23 char_defs) is gone from every canonical profile
-- (OD-CS01, OD-CS04): Praxis humans, Loyalist Jaffa and SGU humans start with
-- no abilities and learn them in the tutorials (CS-03/04/05).
--
-- SGU_FREE_JAFFA (8, 18): 597 Heal Focus and 1218 Recuperation as racial_core
-- (OD-CS05 sustain), 1984 Staff Swing as the Jaffa signature (OD-CS06). Each
-- gets a provenance row at creation.
--
-- PRA_GOAULD (10, 19) and SGU_ASGARD (9): the legacy kit, kept literally by
-- the holding states (OD-CS08, OD-CS09); no provenance row, no branch credit.
--

INSERT INTO char_creation_abilities (char_def_id, ability_id, source_kind) VALUES (8, 597, 'racial_core');
INSERT INTO char_creation_abilities (char_def_id, ability_id, source_kind) VALUES (8, 1218, 'racial_core');
INSERT INTO char_creation_abilities (char_def_id, ability_id, source_kind) VALUES (8, 1984, 'signature');
INSERT INTO char_creation_abilities (char_def_id, ability_id, source_kind) VALUES (18, 597, 'racial_core');
INSERT INTO char_creation_abilities (char_def_id, ability_id, source_kind) VALUES (18, 1218, 'racial_core');
INSERT INTO char_creation_abilities (char_def_id, ability_id, source_kind) VALUES (18, 1984, 'signature');

INSERT INTO char_creation_abilities (char_def_id, ability_id, source_kind) VALUES (9, 592, 'legacy_kit');
INSERT INTO char_creation_abilities (char_def_id, ability_id, source_kind) VALUES (9, 594, 'legacy_kit');
INSERT INTO char_creation_abilities (char_def_id, ability_id, source_kind) VALUES (9, 597, 'legacy_kit');
INSERT INTO char_creation_abilities (char_def_id, ability_id, source_kind) VALUES (9, 1218, 'legacy_kit');
INSERT INTO char_creation_abilities (char_def_id, ability_id, source_kind) VALUES (9, 1646, 'legacy_kit');
INSERT INTO char_creation_abilities (char_def_id, ability_id, source_kind) VALUES (10, 592, 'legacy_kit');
INSERT INTO char_creation_abilities (char_def_id, ability_id, source_kind) VALUES (10, 594, 'legacy_kit');
INSERT INTO char_creation_abilities (char_def_id, ability_id, source_kind) VALUES (10, 597, 'legacy_kit');
INSERT INTO char_creation_abilities (char_def_id, ability_id, source_kind) VALUES (10, 1218, 'legacy_kit');
INSERT INTO char_creation_abilities (char_def_id, ability_id, source_kind) VALUES (10, 1646, 'legacy_kit');
INSERT INTO char_creation_abilities (char_def_id, ability_id, source_kind) VALUES (19, 592, 'legacy_kit');
INSERT INTO char_creation_abilities (char_def_id, ability_id, source_kind) VALUES (19, 594, 'legacy_kit');
INSERT INTO char_creation_abilities (char_def_id, ability_id, source_kind) VALUES (19, 597, 'legacy_kit');
INSERT INTO char_creation_abilities (char_def_id, ability_id, source_kind) VALUES (19, 1218, 'legacy_kit');
INSERT INTO char_creation_abilities (char_def_id, ability_id, source_kind) VALUES (19, 1646, 'legacy_kit');
