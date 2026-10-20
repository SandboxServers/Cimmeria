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
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Jaffa Weapons tier 1, 200 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63808, 'Omega Jaffa Weapons Tier 1');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638400, 63808, 2797, 1, 200); -- Serpent Staff, staff, tc 1
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638401, 63808, 5141, 1, 200); -- Discharge Zat'ni'katel, zat, tc 1
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638402, 63808, 4566, 1, 600); -- Serpent Ribbon Device, ribbon, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638403, 63808, 3470, 1, 600); -- Serpent Staff, staff, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638404, 63808, 5140, 1, 600); -- Discharge Zat'ni'katel, zat, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638405, 63808, 4458, 1, 1000); -- Discharge Ribbon Device, ribbon, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638406, 63808, 3471, 1, 1000); -- Serpent Staff, staff, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638407, 63808, 3904, 1, 1000); -- Shriek Zat'ni'katel, zat, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638408, 63808, 4487, 1, 1000); -- Radiant Ribbon Device, ribbon, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638409, 63808, 3744, 1, 1000); -- Decay Staff, staff, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638410, 63808, 3908, 1, 1000); -- Discharge Zat'ni'katel, zat, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638411, 63808, 4513, 1, 1000); -- Boreal Ribbon Device, ribbon, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638412, 63808, 3773, 1, 1000); -- Discharge Staff, staff, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638413, 63808, 3940, 1, 1000); -- Radiant Zat'ni'katel, zat, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638414, 63808, 4540, 1, 1000); -- Pilgrim Ribbon Device, ribbon, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638415, 63808, 3792, 1, 1000); -- Shriek Staff, staff, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638416, 63808, 3965, 1, 1000); -- Boreal Zat'ni'katel, zat, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638417, 63808, 4567, 1, 1000); -- Serpent Ribbon Device, ribbon, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638418, 63808, 3818, 1, 1000); -- Radiant Staff, staff, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638419, 63808, 3977, 1, 1000); -- Decay Zat'ni'katel, zat, tc 5

-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Jaffa Weapons tier 2, 200 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63809, 'Omega Jaffa Weapons Tier 2');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638450, 63809, 3746, 1, 4000); -- Venom Staff, staff, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638451, 63809, 3916, 1, 4000); -- Clarion Zat'ni'katel, zat, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638452, 63809, 4466, 1, 4000); -- Spark Ribbon Device, ribbon, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638453, 63809, 6638, 1, 4000); -- Venom Jaffa Cannon, heavy, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638454, 63809, 3755, 1, 4000); -- Disjunct Staff, staff, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638455, 63809, 3925, 1, 4000); -- Spark Zat'ni'katel, zat, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638456, 63809, 4494, 1, 4000); -- Flame Ribbon Device, ribbon, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638457, 63809, 6639, 1, 4600); -- Venom Jaffa Cannon, heavy, tc 23
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638458, 63809, 3785, 1, 4000); -- Spark Staff, staff, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638459, 63809, 3947, 1, 4000); -- Flame Zat'ni'katel, zat, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638460, 63809, 4522, 1, 4000); -- Gelid Ribbon Device, ribbon, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638461, 63809, 6640, 1, 5000); -- Venom Jaffa Cannon, heavy, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638462, 63809, 3808, 1, 4000); -- Clarion Staff, staff, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638463, 63809, 3978, 1, 4000); -- Gelid Zat'ni'katel, zat, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638464, 63809, 4547, 1, 4000); -- Warrior Ribbon Device, ribbon, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638465, 63809, 6641, 1, 5600); -- Venom Jaffa Cannon, heavy, tc 28
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638466, 63809, 3826, 1, 4000); -- Flame Staff, staff, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638467, 63809, 3993, 1, 4000); -- Disjunct Zat'ni'katel, zat, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638468, 63809, 4583, 1, 4000); -- Clarion Ribbon Device, ribbon, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638469, 63809, 6642, 1, 6000); -- Venom Jaffa Cannon, heavy, tc 30

-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Jaffa Weapons tier 3, 200 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63810, 'Omega Jaffa Weapons Tier 3');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638500, 63810, 3753, 1, 6600); -- Cobra Staff, staff, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638501, 63810, 3924, 1, 6600); -- Stentorian Zat'ni'katel, zat, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638502, 63810, 4472, 1, 6600); -- Surge Ribbon Device, ribbon, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638503, 63810, 6644, 1, 6600); -- Cobra Jaffa Cannon, heavy, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638504, 63810, 3768, 1, 6600); -- Degenerate Staff, staff, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638505, 63810, 3931, 1, 6600); -- Surge Zat'ni'katel, zat, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638506, 63810, 4500, 1, 6600); -- Star Ribbon Device, ribbon, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638507, 63810, 6645, 1, 7000); -- Cobra Jaffa Cannon, heavy, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638508, 63810, 3793, 1, 6600); -- Surge Staff, staff, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638509, 63810, 3953, 1, 6600); -- Star Zat'ni'katel, zat, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638510, 63810, 4528, 1, 6600); -- Negation Ribbon Device, ribbon, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638511, 63810, 6646, 1, 7600); -- Cobra Jaffa Cannon, heavy, tc 38
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638512, 63810, 3814, 1, 6600); -- Stentorian Staff, staff, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638513, 63810, 3991, 1, 6600); -- Negation Zat'ni'katel, zat, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638514, 63810, 4553, 1, 6600); -- Battle Ribbon Device, ribbon, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638515, 63810, 6647, 1, 8000); -- Cobra Jaffa Cannon, heavy, tc 40
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638516, 63810, 3832, 1, 6600); -- Star Staff, staff, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638517, 63810, 4004, 1, 6600); -- Degeneration Zat'ni'katel, zat, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638518, 63810, 4589, 1, 6600); -- Stentorian Ribbon Device, ribbon, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638519, 63810, 6648, 1, 8600); -- Cobra Jaffa Cannon, heavy, tc 43

-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Jaffa Weapons tier 4, 200 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63811, 'Omega Jaffa Weapons Tier 4');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638550, 63811, 3760, 1, 9000); -- Asp Staff, staff, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638551, 63811, 3937, 1, 9000); -- Bolt Zat'ni'katel, zat, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638552, 63811, 4478, 1, 9000); -- Bolt Ribbon Device, ribbon, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638553, 63811, 6650, 1, 9000); -- Asp Jaffa Cannon, heavy, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638554, 63811, 3782, 1, 9000); -- Toxic Staff, staff, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638555, 63811, 3959, 1, 9000); -- Sun Zat'ni'katel, zat, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638556, 63811, 4506, 1, 9000); -- Sun Ribbon Device, ribbon, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638557, 63811, 6651, 1, 9600); -- Asp Jaffa Cannon, heavy, tc 48
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638558, 63811, 3803, 1, 9000); -- Bolt Staff, staff, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638559, 63811, 3970, 1, 9000); -- Thunder Zat'ni'katel, zat, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638560, 63811, 4534, 1, 9000); -- Vitiate Ribbon Device, ribbon, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638561, 63811, 6652, 1, 10000); -- Asp Jaffa Cannon, heavy, tc 50
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638562, 63811, 3837, 1, 9000); -- Sun Staff, staff, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638563, 63811, 4003, 1, 9000); -- Vitiate Zat'ni'katel, zat, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638564, 63811, 4560, 1, 9000); -- Master Ribbon Device, ribbon, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638565, 63811, 3844, 1, 9000); -- Thunder Staff, staff, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638566, 63811, 4012, 1, 9000); -- Toxic Zat'ni'katel, zat, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638567, 63811, 4595, 1, 9000); -- Thunder Ribbon Device, ribbon, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638568, 63811, 3874, 1, 9000); -- Vitiate Staff, staff, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638569, 63811, 4058, 1, 9000); -- Asp Zat'ni'katel, zat, tc 45
-- [UO-12] end

-- [UO-27] begin: lists 63804-63807
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
