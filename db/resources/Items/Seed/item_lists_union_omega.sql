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
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Human Armor tier 1, 100 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63804, 'Omega Human Armor Tier 1');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638200, 63804, 4330, 1, 100); -- BDU Boots, Boots slot, tc 1
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638201, 63804, 2971, 1, 100); -- Titanium Ablative Vest, Vest slot, tc 1
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638202, 63804, 7374, 1, 300); -- Armored BDU Boots, Boots slot, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638203, 63804, 2967, 1, 300); -- Armored BDU Gloves, Gloves slot, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638204, 63804, 7376, 1, 300); -- Armored BDU Gloves, Gloves slot, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638205, 63804, 7375, 1, 300); -- Armored BDU Helmet, Helmet slot, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638206, 63804, 2949, 1, 300); -- Armored Prison Boots, Boots slot, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638207, 63804, 2925, 1, 300); -- Armored Prison Helmet, Helmet slot, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638208, 63804, 3401, 1, 300); -- Covert Stealth Boots, Boots slot, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638209, 63804, 3387, 1, 300); -- Covert Stealth Gloves, Gloves slot, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638210, 63804, 3346, 1, 300); -- Covert Stealth Helmet, Helmet slot, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638211, 63804, 3372, 1, 300); -- Covert Stealth Pants, Pants slot, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638212, 63804, 3359, 1, 300); -- Covert Stealth Vest, Vest slot, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638213, 63804, 3488, 1, 300); -- Prison Jumpsuit Ablative Vest, Vest slot, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638214, 63804, 7089, 1, 500); -- Aramid Fiber Ablative Gloves, Gloves slot, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638215, 63804, 6861, 1, 500); -- Aramid Fiber Ablative Helmet, Helmet slot, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638216, 63804, 7014, 1, 500); -- Aramid Fiber Ablative Pants, Pants slot, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638217, 63804, 6938, 1, 500); -- Aramid Fiber Ablative Vest, Vest slot, tc 5
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Human Armor tier 2, 100 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63805, 'Omega Human Armor Tier 2');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638250, 63805, 4168, 1, 2000); -- Boron-Carbide Ablative Gloves, Gloves slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638251, 63805, 4072, 1, 2000); -- Boron-Carbide Ablative Helmet, Helmet slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638252, 63805, 4152, 1, 2000); -- Boron-Carbide Ablative Pants, Pants slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638253, 63805, 4094, 1, 2000); -- Boron-Carbide Ablative Vest, Vest slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638254, 63805, 4190, 1, 2000); -- Boron-Carbide Ballistic Boots, Boots slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638255, 63805, 4163, 1, 2000); -- Boron-Carbide Ballistic Gloves, Gloves slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638256, 63805, 4077, 1, 2000); -- Boron-Carbide Ballistic Helmet, Helmet slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638257, 63805, 4130, 1, 2000); -- Boron-Carbide Ballistic Pants, Pants slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638258, 63805, 4112, 1, 2000); -- Boron-Carbide Ballistic Vest, Vest slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638259, 63805, 4284, 1, 2000); -- Boron-Carbide Contamination Boots, Boots slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638260, 63805, 4273, 1, 2000); -- Boron-Carbide Contamination Gloves, Gloves slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638261, 63805, 4222, 1, 2000); -- Boron-Carbide Contamination Helmet, Helmet slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638262, 63805, 4261, 1, 2000); -- Boron-Carbide Contamination Pants, Pants slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638263, 63805, 7171, 1, 2000); -- Carbon n-Tube Ablative Boots, Boots slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638264, 63805, 7096, 1, 2000); -- Carbon n-Tube Ablative Gloves, Gloves slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638265, 63805, 6868, 1, 2000); -- Carbon n-Tube Ablative Helmet, Helmet slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638266, 63805, 7021, 1, 2000); -- Carbon n-Tube Ablative Pants, Pants slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638267, 63805, 6945, 1, 2000); -- Carbon n-Tube Ablative Vest, Vest slot, tc 20
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Human Armor tier 3, 100 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63806, 'Omega Human Armor Tier 3');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638300, 63806, 7102, 1, 3300); -- Ceramic Polymer Ablative Gloves, Gloves slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638301, 63806, 6876, 1, 3300); -- Ceramic Polymer Ablative Helmet, Helmet slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638302, 63806, 7027, 1, 3300); -- Ceramic Polymer Ablative Pants, Pants slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638303, 63806, 6951, 1, 3300); -- Ceramic Polymer Ablative Vest, Vest slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638304, 63806, 7202, 1, 3300); -- Ceramic Polymer Ballistic Boots, Boots slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638305, 63806, 7127, 1, 3300); -- Ceramic Polymer Ballistic Gloves, Gloves slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638306, 63806, 6901, 1, 3300); -- Ceramic Polymer Ballistic Helmet, Helmet slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638307, 63806, 7052, 1, 3300); -- Ceramic Polymer Ballistic Pants, Pants slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638308, 63806, 6976, 1, 3300); -- Ceramic Polymer Ballistic Vest, Vest slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638309, 63806, 7227, 1, 3300); -- Ceramic Polymer Contamination Boots, Boots slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638310, 63806, 6926, 1, 3300); -- Ceramic Polymer Contamination Helmet, Helmet slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638311, 63806, 7077, 1, 3300); -- Ceramic Polymer Contamination Pants, Pants slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638312, 63806, 7002, 1, 3300); -- Ceramic Polymer Contamination Vest, Vest slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638313, 63806, 4379, 1, 3300); -- Refractive Stealth Boots, Boots slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638314, 63806, 4368, 1, 3300); -- Refractive Stealth Gloves, Gloves slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638315, 63806, 4297, 1, 3300); -- Refractive Stealth Helmet, Helmet slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638316, 63806, 4357, 1, 3300); -- Refractive Stealth Pants, Pants slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638317, 63806, 4322, 1, 3300); -- Refractive Stealth Vest, Vest slot, tc 33
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Human Armor tier 4, 100 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63807, 'Omega Human Armor Tier 4');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638350, 63807, 7108, 1, 4500); -- Bio-Silk Ablative Gloves, Gloves slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638351, 63807, 6882, 1, 4500); -- Bio-Silk Ablative Helmet, Helmet slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638352, 63807, 7033, 1, 4500); -- Bio-Silk Ablative Pants, Pants slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638353, 63807, 6957, 1, 4500); -- Bio-Silk Ablative Vest, Vest slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638354, 63807, 7208, 1, 4500); -- Bio-Silk Ballistic Boots, Boots slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638355, 63807, 7133, 1, 4500); -- Bio-Silk Ballistic Gloves, Gloves slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638356, 63807, 6907, 1, 4500); -- Bio-Silk Ballistic Helmet, Helmet slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638357, 63807, 7058, 1, 4500); -- Bio-Silk Ballistic Pants, Pants slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638358, 63807, 6982, 1, 4500); -- Bio-Silk Ballistic Vest, Vest slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638359, 63807, 7233, 1, 4500); -- Bio-Silk Contamination Boots, Boots slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638360, 63807, 7158, 1, 4500); -- Bio-Silk Contamination Gloves, Gloves slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638361, 63807, 6932, 1, 4500); -- Bio-Silk Contamination Helmet, Helmet slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638362, 63807, 7083, 1, 4500); -- Bio-Silk Contamination Pants, Pants slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638363, 63807, 7008, 1, 4500); -- Bio-Silk Contamination Vest, Vest slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638364, 63807, 4385, 1, 4500); -- Distortive Stealth Boots, Boots slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638365, 63807, 4303, 1, 4500); -- Distortive Stealth Helmet, Helmet slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638366, 63807, 4363, 1, 4500); -- Distortive Stealth Pants, Pants slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638367, 63807, 4333, 1, 4500); -- Distortive Stealth Vest, Vest slot, tc 45
-- [UO-27] end

-- [UO-28] begin: lists 63812-63815
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
