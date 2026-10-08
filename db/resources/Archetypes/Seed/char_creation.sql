--
-- Data for Name: char_creation; Type: TABLE DATA; Schema: resources; Owner: -
--
-- The 23 start profiles (Class Start v6, CS-02; ledger
-- docs/analysis/class-start-v6/README.md, Profiles matrix):
--
--   PRA_OPCORE_* (1, 11, 3, 13, 20, 22, 5, 15) and PRA_LOYALIST_JAFFA (7, 17):
--       Castle_CellBlock stasis room, level 1, no abilities, no weapon.
--   SGU_HUMAN_* (2, 12, 4, 14, 21, 23, 6, 16): SGC_W1, level 1, nothing.
--   SGU_FREE_JAFFA (8, 18): Dakara_E1 (100, -17.4, 230), level 1. The point is
--       on dakara_e1.nav component 279 (the gate plaza), 7.6 m south of the
--       DHD (spawn 38) and 23 m from the gate (stargate 25), outside the gate
--       volume (point set 1005); world entry sends facing 0 (+Z), toward the
--       gate. Same point as respawner 610. Guarded by
--       dakara_free_jaffa_start_is_on_the_gate_plaza_live_db.
--   PRA_GOAULD (10, 19) and SGU_ASGARD (9): NON_CANONICAL_BLOCKED_LEGACY
--       holding states (OD-CS08, OD-CS09): today's worlds and positions and
--       the legacy universal kit, removed as one unit when Egypt / the Asgard
--       start become playable.
--
-- start_level is 1 everywhere (the Dakara missions' seeded level 3 is not a
-- start level); debug_kit is false on every row (lock L2).
--

INSERT INTO char_creation (char_def_id, alignment, archetype, body_set, gender, starting_world, starting_x, starting_y, starting_z, profile_id, start_level, debug_kit, start_state) VALUES (13, 'ALIGNMENT_Praxis', 'ARCHETYPE_Commando', 'BS_HumanFemale.BS_HumanFemale', 'GENDER_Female', 'Castle_CellBlock', -334.230988, 73.4720001, -228.026001, 'PRA_OPCORE_COMMANDO', 1, false, 'CANONICAL');

INSERT INTO char_creation (char_def_id, alignment, archetype, body_set, gender, starting_world, starting_x, starting_y, starting_z, profile_id, start_level, debug_kit, start_state) VALUES (5, 'ALIGNMENT_Praxis', 'ARCHETYPE_Archeologist', 'BS_HumanMale.BS_HumanMale', 'GENDER_Male', 'Castle_CellBlock', -334.230988, 73.4720001, -228.026001, 'PRA_OPCORE_ARCHAEOLOGIST', 1, false, 'CANONICAL');

INSERT INTO char_creation (char_def_id, alignment, archetype, body_set, gender, starting_world, starting_x, starting_y, starting_z, profile_id, start_level, debug_kit, start_state) VALUES (15, 'ALIGNMENT_Praxis', 'ARCHETYPE_Archeologist', 'BS_HumanFemale.BS_HumanFemale', 'GENDER_Female', 'Castle_CellBlock', -334.230988, 73.4720001, -228.026001, 'PRA_OPCORE_ARCHAEOLOGIST', 1, false, 'CANONICAL');

INSERT INTO char_creation (char_def_id, alignment, archetype, body_set, gender, starting_world, starting_x, starting_y, starting_z, profile_id, start_level, debug_kit, start_state) VALUES (3, 'ALIGNMENT_Praxis', 'ARCHETYPE_Commando', 'BS_HumanMale.BS_HumanMale', 'GENDER_Male', 'Castle_CellBlock', -334.230988, 73.4720001, -228.026001, 'PRA_OPCORE_COMMANDO', 1, false, 'CANONICAL');

INSERT INTO char_creation (char_def_id, alignment, archetype, body_set, gender, starting_world, starting_x, starting_y, starting_z, profile_id, start_level, debug_kit, start_state) VALUES (17, 'ALIGNMENT_Praxis', 'ARCHETYPE_Jaffa', 'BS_JaffaFemale.BS_JaffaFemale', 'GENDER_Female', 'Castle_CellBlock', -334.230988, 73.4720001, -228.026001, 'PRA_LOYALIST_JAFFA', 1, false, 'CANONICAL');

INSERT INTO char_creation (char_def_id, alignment, archetype, body_set, gender, starting_world, starting_x, starting_y, starting_z, profile_id, start_level, debug_kit, start_state) VALUES (10, 'ALIGNMENT_Praxis', 'ARCHETYPE_Goauld', 'BS_GoauldMale.BS_GoauldMale', 'GENDER_Male', 'Castle_CellBlock', -334.230988, 73.4720001, -228.026001, 'PRA_GOAULD', 1, false, 'NON_CANONICAL_BLOCKED_LEGACY');

INSERT INTO char_creation (char_def_id, alignment, archetype, body_set, gender, starting_world, starting_x, starting_y, starting_z, profile_id, start_level, debug_kit, start_state) VALUES (19, 'ALIGNMENT_Praxis', 'ARCHETYPE_Goauld', 'BS_GoauldFemale.BS_GoauldFemale', 'GENDER_Female', 'Castle_CellBlock', -334.230988, 73.4720001, -228.026001, 'PRA_GOAULD', 1, false, 'NON_CANONICAL_BLOCKED_LEGACY');

INSERT INTO char_creation (char_def_id, alignment, archetype, body_set, gender, starting_world, starting_x, starting_y, starting_z, profile_id, start_level, debug_kit, start_state) VALUES (20, 'ALIGNMENT_Praxis', 'ARCHETYPE_Scientist', 'BS_HumanMale.BS_HumanMale', 'GENDER_Male', 'Castle_CellBlock', -334.230988, 73.4720001, -228.026001, 'PRA_OPCORE_SCIENTIST', 1, false, 'CANONICAL');

INSERT INTO char_creation (char_def_id, alignment, archetype, body_set, gender, starting_world, starting_x, starting_y, starting_z, profile_id, start_level, debug_kit, start_state) VALUES (11, 'ALIGNMENT_Praxis', 'ARCHETYPE_Soldier', 'BS_HumanFemale.BS_HumanFemale', 'GENDER_Female', 'Castle_CellBlock', -334.230988, 73.4720001, -228.026001, 'PRA_OPCORE_SOLDIER', 1, false, 'CANONICAL');

INSERT INTO char_creation (char_def_id, alignment, archetype, body_set, gender, starting_world, starting_x, starting_y, starting_z, profile_id, start_level, debug_kit, start_state) VALUES (22, 'ALIGNMENT_Praxis', 'ARCHETYPE_Scientist', 'BS_HumanFemale.BS_HumanFemale', 'GENDER_Female', 'Castle_CellBlock', -334.230988, 73.4720001, -228.026001, 'PRA_OPCORE_SCIENTIST', 1, false, 'CANONICAL');

INSERT INTO char_creation (char_def_id, alignment, archetype, body_set, gender, starting_world, starting_x, starting_y, starting_z, profile_id, start_level, debug_kit, start_state) VALUES (1, 'ALIGNMENT_Praxis', 'ARCHETYPE_Soldier', 'BS_HumanMale.BS_HumanMale', 'GENDER_Male', 'Castle_CellBlock', -334.230988, 73.4720001, -228.026001, 'PRA_OPCORE_SOLDIER', 1, false, 'CANONICAL');

INSERT INTO char_creation (char_def_id, alignment, archetype, body_set, gender, starting_world, starting_x, starting_y, starting_z, profile_id, start_level, debug_kit, start_state) VALUES (7, 'ALIGNMENT_Praxis', 'ARCHETYPE_Jaffa', 'BS_JaffaMale.BS_JaffaMale', 'GENDER_Male', 'Castle_CellBlock', -334.230988, 73.4720001, -228.026001, 'PRA_LOYALIST_JAFFA', 1, false, 'CANONICAL');

INSERT INTO char_creation (char_def_id, alignment, archetype, body_set, gender, starting_world, starting_x, starting_y, starting_z, profile_id, start_level, debug_kit, start_state) VALUES (23, 'ALIGNMENT_SGU', 'ARCHETYPE_Scientist', 'BS_HumanFemale.BS_HumanFemale', 'GENDER_Female', 'SGC_W1', 201.5, 1.30999994, 49.723999, 'SGU_HUMAN_SCIENTIST', 1, false, 'CANONICAL');

INSERT INTO char_creation (char_def_id, alignment, archetype, body_set, gender, starting_world, starting_x, starting_y, starting_z, profile_id, start_level, debug_kit, start_state) VALUES (2, 'ALIGNMENT_SGU', 'ARCHETYPE_Soldier', 'BS_HumanMale.BS_HumanMale', 'GENDER_Male', 'SGC_W1', 201.5, 1.30999994, 49.723999, 'SGU_HUMAN_SOLDIER', 1, false, 'CANONICAL');

INSERT INTO char_creation (char_def_id, alignment, archetype, body_set, gender, starting_world, starting_x, starting_y, starting_z, profile_id, start_level, debug_kit, start_state) VALUES (4, 'ALIGNMENT_SGU', 'ARCHETYPE_Commando', 'BS_HumanMale.BS_HumanMale', 'GENDER_Male', 'SGC_W1', 201.5, 1.30999994, 49.723999, 'SGU_HUMAN_COMMANDO', 1, false, 'CANONICAL');

INSERT INTO char_creation (char_def_id, alignment, archetype, body_set, gender, starting_world, starting_x, starting_y, starting_z, profile_id, start_level, debug_kit, start_state) VALUES (6, 'ALIGNMENT_SGU', 'ARCHETYPE_Archeologist', 'BS_HumanMale.BS_HumanMale', 'GENDER_Male', 'SGC_W1', 201.5, 1.30999994, 49.723999, 'SGU_HUMAN_ARCHAEOLOGIST', 1, false, 'CANONICAL');

INSERT INTO char_creation (char_def_id, alignment, archetype, body_set, gender, starting_world, starting_x, starting_y, starting_z, profile_id, start_level, debug_kit, start_state) VALUES (8, 'ALIGNMENT_SGU', 'ARCHETYPE_Sholva', 'BS_JaffaMale.BS_JaffaMale', 'GENDER_Male', 'Dakara_E1', 100, -17.3999996, 230, 'SGU_FREE_JAFFA', 1, false, 'CANONICAL');

INSERT INTO char_creation (char_def_id, alignment, archetype, body_set, gender, starting_world, starting_x, starting_y, starting_z, profile_id, start_level, debug_kit, start_state) VALUES (9, 'ALIGNMENT_SGU', 'ARCHETYPE_Asgard', 'BS_Asgard.BS_Asgard', 'GENDER_Male', 'SGC_W1', 201.5, 1.30999994, 49.723999, 'SGU_ASGARD', 1, false, 'NON_CANONICAL_BLOCKED_LEGACY');

INSERT INTO char_creation (char_def_id, alignment, archetype, body_set, gender, starting_world, starting_x, starting_y, starting_z, profile_id, start_level, debug_kit, start_state) VALUES (12, 'ALIGNMENT_SGU', 'ARCHETYPE_Soldier', 'BS_HumanFemale.BS_HumanFemale', 'GENDER_Female', 'SGC_W1', 201.5, 1.30999994, 49.723999, 'SGU_HUMAN_SOLDIER', 1, false, 'CANONICAL');

INSERT INTO char_creation (char_def_id, alignment, archetype, body_set, gender, starting_world, starting_x, starting_y, starting_z, profile_id, start_level, debug_kit, start_state) VALUES (14, 'ALIGNMENT_SGU', 'ARCHETYPE_Commando', 'BS_HumanFemale.BS_HumanFemale', 'GENDER_Female', 'SGC_W1', 201.5, 1.30999994, 49.723999, 'SGU_HUMAN_COMMANDO', 1, false, 'CANONICAL');

INSERT INTO char_creation (char_def_id, alignment, archetype, body_set, gender, starting_world, starting_x, starting_y, starting_z, profile_id, start_level, debug_kit, start_state) VALUES (16, 'ALIGNMENT_SGU', 'ARCHETYPE_Archeologist', 'BS_HumanFemale.BS_HumanFemale', 'GENDER_Female', 'SGC_W1', 201.5, 1.30999994, 49.723999, 'SGU_HUMAN_ARCHAEOLOGIST', 1, false, 'CANONICAL');

INSERT INTO char_creation (char_def_id, alignment, archetype, body_set, gender, starting_world, starting_x, starting_y, starting_z, profile_id, start_level, debug_kit, start_state) VALUES (18, 'ALIGNMENT_SGU', 'ARCHETYPE_Sholva', 'BS_JaffaFemale.BS_JaffaFemale', 'GENDER_Female', 'Dakara_E1', 100, -17.3999996, 230, 'SGU_FREE_JAFFA', 1, false, 'CANONICAL');

INSERT INTO char_creation (char_def_id, alignment, archetype, body_set, gender, starting_world, starting_x, starting_y, starting_z, profile_id, start_level, debug_kit, start_state) VALUES (21, 'ALIGNMENT_SGU', 'ARCHETYPE_Scientist', 'BS_HumanMale.BS_HumanMale', 'GENDER_Male', 'SGC_W1', 201.5, 1.30999994, 49.723999, 'SGU_HUMAN_SCIENTIST', 1, false, 'CANONICAL');

