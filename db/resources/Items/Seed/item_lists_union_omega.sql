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

-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Jaffa Armor tier 1, 100 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63812, 'Omega Jaffa Armor Tier 1');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638600, 63812, 4340, 1, 100); -- Standard Boots, Standard armor Boots, tc 1
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638601, 63812, 4341, 1, 100); -- Standard Leggings, Standard armor Leggings, tc 1
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638602, 63812, 6375, 1, 500); -- Falcon Ranged Boots, Falcon Ranged armor Boots, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638603, 63812, 6349, 1, 500); -- Falcon Ranged Breastplate, Falcon Ranged armor Breastplate, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638604, 63812, 6401, 1, 500); -- Falcon Ranged Gauntlets, Falcon Ranged armor Gauntlets, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638605, 63812, 6427, 1, 500); -- Falcon Ranged Helmet, Falcon Ranged armor Helmet, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638606, 63812, 6454, 1, 500); -- Falcon Ranged Leggings, Falcon Ranged armor Leggings, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638607, 63812, 6235, 1, 500); -- Jackal Melee Boots, Jackal Melee armor Boots, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638608, 63812, 6208, 1, 500); -- Jackal Melee Breastplate, Jackal Melee armor Breastplate, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638609, 63812, 6270, 1, 500); -- Jackal Melee Gauntlets, Jackal Melee armor Gauntlets, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638610, 63812, 6297, 1, 500); -- Jackal Melee Helmet, Jackal Melee armor Helmet, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638611, 63812, 6323, 1, 500); -- Jackal Melee Leggings, Jackal Melee armor Leggings, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638612, 63812, 6059, 1, 500); -- Praxis Defensive Boots, Praxis Defensive armor Boots, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638613, 63812, 5976, 1, 500); -- Praxis Defensive Breastplate, Praxis Defensive armor Breastplate, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638614, 63812, 6107, 1, 500); -- Praxis Defensive Gauntlets, Praxis Defensive armor Gauntlets, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638615, 63812, 6140, 1, 500); -- Praxis Defensive Helmet, Praxis Defensive armor Helmet, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638616, 63812, 6172, 1, 500); -- Praxis Defensive Leggings, Praxis Defensive armor Leggings, tc 5

-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Jaffa Armor tier 2, 100 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63813, 'Omega Jaffa Armor Tier 2');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638650, 63813, 6382, 1, 2000); -- Asian Ranged Boots, Asian Ranged armor Boots, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638651, 63813, 6356, 1, 2000); -- Asian Ranged Breastplate, Asian Ranged armor Breastplate, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638652, 63813, 6408, 1, 2000); -- Asian Ranged Gauntlets, Asian Ranged armor Gauntlets, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638653, 63813, 6434, 1, 2000); -- Asian Ranged Helmet, Asian Ranged armor Helmet, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638654, 63813, 6461, 1, 2000); -- Asian Ranged Leggings, Asian Ranged armor Leggings, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638655, 63813, 6246, 1, 2000); -- Cat Melee Boots, Cat Melee armor Boots, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638656, 63813, 6216, 1, 2000); -- Cat Melee Breastplate, Cat Melee armor Breastplate, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638657, 63813, 6277, 1, 2000); -- Cat Melee Gauntlets, Cat Melee armor Gauntlets, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638658, 63813, 6304, 1, 2000); -- Cat Melee Helmet, Cat Melee armor Helmet, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638659, 63813, 6330, 1, 2000); -- Cat Melee Leggings, Cat Melee armor Leggings, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638660, 63813, 6073, 1, 2000); -- Horse Defensive Boots, Horse Defensive armor Boots, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638661, 63813, 6025, 1, 2000); -- Horse Defensive Breastplate, Horse Defensive armor Breastplate, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638662, 63813, 6121, 1, 2000); -- Horse Defensive Gauntlets, Horse Defensive armor Gauntlets, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638663, 63813, 6154, 1, 2000); -- Horse Defensive Helmet, Horse Defensive armor Helmet, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638664, 63813, 6186, 1, 2000); -- Horse Defensive Leggings, Horse Defensive armor Leggings, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638665, 63813, 4377, 1, 2000); -- Veiled Stealth Boots, Veiled Stealth armor Boots, tc 20

-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Jaffa Armor tier 3, 100 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63814, 'Omega Jaffa Armor Tier 3');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638700, 63814, 6079, 1, 3300); -- Bull Defensive Boots, Bull Defensive armor Boots, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638701, 63814, 6046, 1, 3300); -- Bull Defensive Breastplate, Bull Defensive armor Breastplate, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638702, 63814, 6127, 1, 3300); -- Bull Defensive Gauntlets, Bull Defensive armor Gauntlets, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638703, 63814, 6160, 1, 3300); -- Bull Defensive Helmet, Bull Defensive armor Helmet, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638704, 63814, 6193, 1, 3300); -- Bull Defensive Leggings, Bull Defensive armor Leggings, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638705, 63814, 6388, 1, 3300); -- Dragon Ranged Boots, Dragon Ranged armor Boots, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638706, 63814, 6362, 1, 3300); -- Dragon Ranged Breastplate, Dragon Ranged armor Breastplate, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638707, 63814, 6414, 1, 3300); -- Dragon Ranged Gauntlets, Dragon Ranged armor Gauntlets, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638708, 63814, 6440, 1, 3300); -- Dragon Ranged Helmet, Dragon Ranged armor Helmet, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638709, 63814, 6256, 1, 3300); -- Mayan Melee Boots, Mayan Melee armor Boots, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638710, 63814, 6222, 1, 3300); -- Mayan Melee Breastplate, Mayan Melee armor Breastplate, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638711, 63814, 6283, 1, 3300); -- Mayan Melee Gauntlets, Mayan Melee armor Gauntlets, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638712, 63814, 6310, 1, 3300); -- Mayan Melee Helmet, Mayan Melee armor Helmet, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638713, 63814, 6336, 1, 3300); -- Mayan Melee Leggings, Mayan Melee armor Leggings, tc 33

-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Jaffa Armor tier 4, 100 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63815, 'Omega Jaffa Armor Tier 4');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638750, 63815, 6263, 1, 4500); -- Hyena Melee Boots, Hyena Melee armor Boots, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638751, 63815, 6228, 1, 4500); -- Hyena Melee Breastplate, Hyena Melee armor Breastplate, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638752, 63815, 6289, 1, 4500); -- Hyena Melee Gauntlets, Hyena Melee armor Gauntlets, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638753, 63815, 6316, 1, 4500); -- Hyena Melee Helmet, Hyena Melee armor Helmet, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638754, 63815, 6342, 1, 4500); -- Hyena Melee Leggings, Hyena Melee armor Leggings, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638755, 63815, 6394, 1, 4500); -- Naga Ranged Boots, Naga Ranged armor Boots, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638756, 63815, 6368, 1, 4500); -- Naga Ranged Breastplate, Naga Ranged armor Breastplate, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638757, 63815, 6420, 1, 4500); -- Naga Ranged Gauntlets, Naga Ranged armor Gauntlets, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638758, 63815, 6446, 1, 4500); -- Naga Ranged Helmet, Naga Ranged armor Helmet, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638759, 63815, 6473, 1, 4500); -- Naga Ranged Leggings, Naga Ranged armor Leggings, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638760, 63815, 6085, 1, 4500); -- Viking Defensive Boots, Viking Defensive armor Boots, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638761, 63815, 6053, 1, 4500); -- Viking Defensive Breastplate, Viking Defensive armor Breastplate, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638762, 63815, 6134, 1, 4500); -- Viking Defensive Gauntlets, Viking Defensive armor Gauntlets, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638763, 63815, 6166, 1, 4500); -- Viking Defensive Helmet, Viking Defensive armor Helmet, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638764, 63815, 6200, 1, 4500); -- Viking Defensive Leggings, Viking Defensive armor Leggings, tc 45

-- [UO-28] end

-- [UO-29] begin: lists 63817-63820
-- [UO-29] end

-- [UO-30] begin: lists 63816
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
