--
-- Union Omega (worlds 18 `Omega_Site` and 80 `Omega_Site_CmdCenter`): vendor
-- buy lists. Campaign: docs/analysis/union-zone-planning/omega-work-packets.md (packets UO-01 to UO-42).
--
-- Id block 63800-63999 for lists, 639999 for list rows (Omega's reservation,
-- D-UO1). Each packet writes only below its own marker. Provenance: each block
-- states its own tag (RESTORED_SOURCE, CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING).
--

SET search_path = resources, pg_catalog;

-- [UO-11] begin: lists 63800-63803
-- [UO-11] end

-- [UO-12] begin: lists 63808-63811
-- [UO-12] end

-- [UO-27] begin: lists 63804-63807
-- [UO-27] end

-- [UO-28] begin: lists 63812-63815
-- [UO-28] end

-- [UO-29] begin: lists 63817-63820
-- [UO-29] end

-- [UO-30] begin: lists 63816
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Consumables, T1 25/50/100, above T1 25 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63816, 'Omega Consumables');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638800, 63816, 2893, 1, 25); -- Health Slappack TC1, tier 1, tc 1
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638801, 63816, 6657, 1, 50); -- Health Antidote, tier 1, tc 10
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638802, 63816, 6659, 1, 50); -- Health Antidote, tier 1, tc 10
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638803, 63816, 6656, 1, 50); -- Kinetic Antidote, tier 1, tc 10
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638804, 63816, 6677, 1, 100); -- Mark III Stimpack: Coordination, tier 1, tc 10
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638805, 63816, 6678, 1, 100); -- Mark III Stimpack: Engagement, tier 1, tc 10
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638806, 63816, 6679, 1, 100); -- Mark III Stimpack: Fortitude, tier 1, tc 10
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638807, 63816, 6680, 1, 100); -- Mark III Stimpack: Intellect, tier 1, tc 10
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638808, 63816, 6681, 1, 100); -- Mark III Stimpack: Morale, tier 1, tc 10
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638809, 63816, 6682, 1, 100); -- Mark III Stimpack: Perception, tier 1, tc 10
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638810, 63816, 6577, 1, 500); -- Kinetic/Mental Antidote, tier 2, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638811, 63816, 6597, 1, 500); -- Health/Mental Antidote, tier 2, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638812, 63816, 6598, 1, 500); -- Health/Kinetic Antidote, tier 2, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638813, 63816, 6660, 1, 625); -- Double Health/Kinetic Antidote, tier 2, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638814, 63816, 6661, 1, 625); -- Double Health/Mental Antidote, tier 2, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638815, 63816, 6662, 1, 625); -- Double Kinetic/Mental Antidote, tier 2, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638816, 63816, 6697, 1, 625); -- Mark V Stimpack: Coordination/Engagement, tier 2, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638817, 63816, 6719, 1, 625); -- Mark V Stimpack: Engagement/Perception, tier 2, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638818, 63816, 6722, 1, 625); -- Mark V Stimpack: Fortitude/Coordination, tier 2, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638819, 63816, 6725, 1, 625); -- Mark V Stimpack: Intellect/Morale, tier 2, tc 25
-- [UO-30] end

-- [UO-39] begin: lists 63821-63824
-- [UO-39] end

-- [UO-40] begin: lists 63825-63828
-- [UO-40] end

--
-- The rows above load after item_lists.sql's own sequence footers, so they
-- raise the sequences themselves. Floors: 63999 for lists, 639999 for list
-- rows, the top of Omega's block.
SELECT pg_catalog.setval('item_lists_item_list_id_seq', GREATEST((SELECT MAX(item_list_id) FROM item_lists), (SELECT last_value FROM item_lists_item_list_id_seq), 63999), true);
SELECT pg_catalog.setval('item_list_items_item_id_seq', GREATEST((SELECT MAX(item_id) FROM item_list_items), (SELECT last_value FROM item_list_items_item_id_seq), 639999), true);
