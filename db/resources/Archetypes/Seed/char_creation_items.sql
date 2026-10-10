--
-- Data for Name: char_creation_items; Type: TABLE DATA; Schema: resources; Owner: -
--
-- Starter items per start profile (Class Start v6, CS-02), on top of the
-- items the visual choices carry. The universal loaded pistol is gone from
-- every canonical profile (OD-CS01): the first firearm comes from the
-- tutorial (M622 in the Cellblock, M1559 in the SGC).
--
-- SGU_FREE_JAFFA (8, 18): 2797 Serpent Staff. The other half of the Dakara
-- start gear of the v6 Gear matrix, 4342 Standard Chestplate, is the forced
-- Torso choice of both char_defs (char_creation_choices 540 and 1217), which
-- places it in the Chest slot already; a row here put a second one in the
-- bag (Class Start v6 CS-08 F4).
--
-- PRA_GOAULD (10, 19) and SGU_ASGARD (9): the holding states keep pistol 55,
-- SI 3 9mm Pistol, in bandolier slot 0, now with 0 rounds: guns given at
-- character creation start empty too (OD-CS13 amendment, 2026-10-05, the one
-- intended change to the holding states).
--

INSERT INTO char_creation_items (char_def_id, item_id, stack_size) VALUES (8, 2797, 1);
INSERT INTO char_creation_items (char_def_id, item_id, stack_size) VALUES (18, 2797, 1);

INSERT INTO char_creation_items (char_def_id, item_id, stack_size) VALUES (9, 55, 1);
INSERT INTO char_creation_items (char_def_id, item_id, stack_size) VALUES (10, 55, 1);
INSERT INTO char_creation_items (char_def_id, item_id, stack_size) VALUES (19, 55, 1);
