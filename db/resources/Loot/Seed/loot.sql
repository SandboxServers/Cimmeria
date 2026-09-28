--
-- TOC entry 3207 (class 0 OID 62985)
-- Dependencies: 224
-- Data for Name: loot; Type: TABLE DATA; Schema: resources; Owner: -
--

INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (10, 1, 3730, 1, 1, 1);

INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (11, 1, 55, 1, 1, 1);

INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (12, 2, NULL, 5, 0.800000012, 50);

-- Health Slappack TC1 (item 2893, +500 HP) as a guaranteed drop on the
-- shared Cellblock guard table. Pairs with the out-of-combat regen tick:
-- regen restores HP between fights, slappacks cover the burst recovery
-- for back-to-back encounters where regen alone can't keep up.
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (13, 2, 2893, 1, 1, 1);

-- NEW CONTENT (debug hub): table 3, the stasis-room loot crate. Every row is
-- probability 1: a roll that drops nothing never sets INT_NormalLoot, and the
-- corpse is then unclickable, which would read as a broken loot path. The
-- mix covers each branch Loot All walks: a stackable consumable with a
-- quantity range (Health Slappack TC1, max stack 10), two single items
-- (Processor (Electronics), Cell (Bio-Medical)) and naquadah (design_id
-- NULL). 5192 is also the item cost of vendor list 1's second row.
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (14, 3, 2893, 2, 1, 3);

INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (15, 3, 5228, 1, 1, 1);

INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (16, 3, 5192, 1, 1, 1);

INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (17, 3, NULL, 25, 1, 75);

-- NEW CONTENT (debug hub): crafting knowledge items on the crate, each at a
-- one-in-five chance so a few kills turn them up. The four rows above still
-- drop every time, so the corpse always has loot. These are the five Racial
-- Paradigm Guides (7805 Human, 7806 Common, 7807 Asgard, 7808 Goa'uld,
-- 7809 Ancient) and the Blueprint: Steel Plating item (6483). All six are
-- `{17,15}` items with a stack limit of 1, so the quantity is exactly 1 and
-- a pickup lands in the crafting bag; using one raises the paradigm or
-- teaches the blueprint.
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (18, 3, 7805, 1, 0.2, 1);

INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (19, 3, 7806, 1, 0.2, 1);

INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (20, 3, 7807, 1, 0.2, 1);

INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (21, 3, 7808, 1, 0.2, 1);

INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (22, 3, 7809, 1, 0.2, 1);

INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (23, 3, 6483, 1, 0.2, 1);

-- NEW CONTENT (Castle population, docs/analysis/castle-population/README.md,
-- D-CP09): Castle hostiles drop something sometimes, never guaranteed. Each row
-- rolls independently (cell-combat abilities/loot_drop.rs); if every row misses,
-- the corpse gets no loot cursor, which is the intended "nothing dropped" outcome.
--
-- Only items that do something today:
--   design_id NULL = naquadah (currency).
--   Consumables with a working use path. Since #1021 an item whose
--   items_event_sets event-5 ability heals or buffs (effect scripts HealHealth,
--   HealFocus, StatBuff) works natively, with no chain:
--     2893 Health Slappack TC1 (+500 HP), 6106 Focus Heal Consumable, and the
--     six Mark III stimpacks 6677-6682 (Coordination, Engagement, Fortitude,
--     Intellect, Morale, Perception; a timed attribute buff).
--   Unwired bag consumables (the Stealth, Energy and Disguise boosts, the
--   antidotes) answer "This item has no effect yet.", so they never drop, and
--   stimpacks stop at Mark III: Mark V and up are too strong for a level 3-4 zone.
--   5224 Integrated Circuit (Electronics), 5188 Protein Complex (Bio-Medical),
--   5257 Wave Guide (Power Systems): tier-1 crafting components, the most-used
--        base components of their applied sciences (85, 53 and 42 blueprints in
--        blueprints_components). A guard carries electronics and a field medkit's
--        worth of bio-medical stock; a drone salvages to electronics and power.
-- There is no grenade item in the client item table, so no grenades.
--
-- Chance a corpse drops nothing (product of the misses):
--   table 4 guard    0.50 * 0.85 * 0.92^2 * 0.90 * 0.99^6 = 30.5%
--   table 5 veteran  0.40 * 0.75 * 0.88^2 * 0.85 * 0.98^6 = 17.5%
--   table 6 PRU      0.70 * 0.80                          = 56%
-- Any stimpack at all: 5.9% of guard corpses (1 - 0.99^6), 11.4% of veteran
-- corpses (1 - 0.98^6).
-- Table 5 is on templates 169 (Romney), 170 (Muelbach) and 171 (the Bravo
-- officers), whose deaths also drive missions 703 and 708. Those grants are
-- `add_item` actions in the entity_dead_tag chains (1272/1273, 1346-1349), not
-- loot rows, and no chain reads the corpse's interaction flags, so a loot roll
-- neither duplicates nor blocks Romney's 2135, the Control Crystal (2790) or item 2136.
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (24, 4, NULL, 5, 0.5, 20);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (25, 4, 2893, 1, 0.15, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (26, 4, 5224, 1, 0.08, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (27, 4, 5188, 1, 0.08, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (28, 5, NULL, 10, 0.6, 35);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (29, 5, 2893, 1, 0.25, 2);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (30, 5, 5224, 1, 0.12, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (31, 5, 5188, 1, 0.12, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (32, 6, 5224, 1, 0.3, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (33, 6, 5257, 1, 0.2, 1);
-- Added after #1021 made event-5 consumables work natively (owner, 2026-09-28).
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (34, 4, 6106, 1, 0.1, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (35, 4, 6677, 1, 0.01, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (36, 4, 6678, 1, 0.01, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (37, 4, 6679, 1, 0.01, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (38, 4, 6680, 1, 0.01, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (39, 4, 6681, 1, 0.01, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (40, 4, 6682, 1, 0.01, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (41, 5, 6106, 1, 0.15, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (42, 5, 6677, 1, 0.02, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (43, 5, 6678, 1, 0.02, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (44, 5, 6679, 1, 0.02, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (45, 5, 6680, 1, 0.02, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (46, 5, 6681, 1, 0.02, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (47, 5, 6682, 1, 0.02, 1);

-- Table 7, Castle hall NID guard (Decision (@Cadacious, 2026-09-28)):
-- naquadah 5-25 half the time, a Health Slappack three kills in ten.
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (48, 7, NULL, 5, 0.5, 25);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (49, 7, 2893, 1, 0.3, 1);

-- Tables 8/9, the Castle pre-Romney chest (Decision (@Cadacious, 2026-09-28)):
-- the archetype weapon, 2-3 Health Slappacks, two Focus Heals and 25-75
-- naquadah. TODO(#1026): Hollow Point ammo joins both tables when #1026 lands.
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (50, 8, 3127, 1, 1, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (51, 8, 2893, 2, 1, 3);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (52, 8, 6106, 2, 1, 2);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (53, 8, NULL, 25, 1, 75);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (54, 9, 3472, 1, 1, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (55, 9, 2893, 2, 1, 3);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (56, 9, 6106, 2, 1, 2);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (57, 9, NULL, 25, 1, 75);

-- Tables 10/11, the Cellblock weapon/armor crate: chain 1098's and 1099's former
-- add_item lists, moved into a loot window (Decision (@Cadacious, 2026-09-28)).
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (58, 10, 3347, 1, 1, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (59, 10, 3359, 1, 1, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (60, 10, 3372, 1, 1, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (61, 10, 3387, 1, 1, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (62, 10, 3401, 1, 1, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (63, 10, 3325, 1, 1, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (64, 11, 3482, 1, 1, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (65, 11, 2797, 1, 1, 1);

--
-- TOC entry 3323 (class 0 OID 0)
-- Dependencies: 225
-- Name: loot_loot_id_seq; Type: SEQUENCE SET; Schema: resources; Owner: -
--

SELECT pg_catalog.setval('loot_loot_id_seq', 65, true);

