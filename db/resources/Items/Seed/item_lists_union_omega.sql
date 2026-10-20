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
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Crafting Bio-Medical, 25 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63817, 'Omega Crafting Biomedical');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638850, 63817, 5369, 1, 125); -- BMAS-5 Field Crafting Tool, tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638851, 63817, 5447, 1, 125); -- Beta Biomedical Research Kit, tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638852, 63817, 8407, 1, 125); -- BMAS-10 Field Crafting Tool, tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638853, 63817, 8408, 1, 375); -- BMAS-15 Field Crafting Tool, tier 1, tc 15
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638854, 63817, 8409, 1, 500); -- BMAS-20 Field Crafting Tool, tier 2, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638855, 63817, 8410, 1, 625); -- BMAS-25 Field Crafting Tool, tier 2, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638856, 63817, 8411, 1, 750); -- BMAS-30 Field Crafting Tool, tier 2, tc 30
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638857, 63817, 8412, 1, 875); -- BMAS-35 Field Crafting Tool, tier 3, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638858, 63817, 8413, 1, 1000); -- BMAS-40 Field Crafting Tool, tier 3, tc 40
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638859, 63817, 8414, 1, 1125); -- BMAS-45 Field Crafting Tool, tier 4, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638860, 63817, 8415, 1, 1250); -- BMAS-50 Field Crafting Tool, tier 5, tc 50
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638861, 63817, 8416, 1, 1375); -- BMAS-55 Field Crafting Tool, tier 5, tc 55
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638862, 63817, 5188, 1, 75); -- Protein Complex (Bio-Medical), tier 1, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638863, 63817, 5189, 1, 125); -- T1 Cell (Bio-Medical), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638864, 63817, 5360, 1, 125); -- Rich Substrate (Bio-Medical Subcombine A), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638865, 63817, 5362, 1, 125); -- Strombine (Bio-Medical Subcombine A), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638866, 63817, 5363, 1, 200); -- Xeno-bacterium (Bio-Medical Subcombine B), tier 1, tc 8
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638867, 63817, 5364, 1, 200); -- Chiral Lysine (Bio-Medical Subcombine B), tier 1, tc 8
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638868, 63817, 5366, 1, 275); -- X-Cosmid (Bio-Medical Subcombine C), tier 1, tc 11
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638869, 63817, 5367, 1, 275); -- Mimic Rhinovirus (Bio-Medical Subcombine C), tier 1, tc 11

-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Crafting Electronics, 25 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63818, 'Omega Crafting Electronics');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638900, 63818, 8402, 1, 125); -- EAS-5 Field Crafting Tool, tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638901, 63818, 8887, 1, 125); -- Beta Electronics Research Kit, tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638902, 63818, 8433, 1, 250); -- EAS-10 Field Crafting Tool, tier 1, tc 10
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638903, 63818, 8434, 1, 375); -- EAS-15 Field Crafting Tool, tier 1, tc 15
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638904, 63818, 8435, 1, 500); -- EAS-20 Field Crafting Tool, tier 2, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638905, 63818, 8436, 1, 625); -- EAS-25 Field Crafting Tool, tier 2, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638906, 63818, 8437, 1, 750); -- EAS-30 Field Crafting Tool, tier 2, tc 30
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638907, 63818, 8438, 1, 875); -- EAS-35 Field Crafting Tool, tier 3, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638908, 63818, 8439, 1, 1000); -- EAS-40 Field Crafting Tool, tier 4, tc 40
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638909, 63818, 8440, 1, 1125); -- EAS-45 Field Crafting Tool, tier 4, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638910, 63818, 8441, 1, 1250); -- EAS-50 Field Crafting Tool, tier 5, tc 50
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638911, 63818, 8442, 1, 1375); -- EAS-55 Field Crafting Tool, tier 5, tc 55
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638912, 63818, 5224, 1, 75); -- Integrated Circuit (Electronics), tier 1, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638913, 63818, 5225, 1, 125); -- Integrated Psuedocore (Electronics), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638914, 63818, 5373, 1, 125); -- IC-K-layer (Electronics Subcombine A), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638915, 63818, 5374, 1, 125); -- Output Processor (Electronics Subcombine A), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638916, 63818, 5375, 1, 125); -- Minimatrix (Electronics Subcombine A), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638917, 63818, 5376, 1, 200); -- Signal Damp (Electronics Subcombine B), tier 1, tc 8
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638918, 63818, 5377, 1, 200); -- Optical Rectifier (Electronics Subcombine B), tier 1, tc 8
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638919, 63818, 5378, 1, 200); -- Cell Blade (Electronics Subcombine B), tier 1, tc 8

-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Crafting Power Systems, 25 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63819, 'Omega Crafting Power Systems');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638950, 63819, 8405, 1, 125); -- PSAS-5 Field Crafting Tool, tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638951, 63819, 8888, 1, 125); -- Beta Power Systems Research Kit, tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638952, 63819, 8453, 1, 250); -- PSAS-10 Field Crafting Tool, tier 1, tc 10
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638953, 63819, 8454, 1, 375); -- PSAS-15 Field Crafting Tool, tier 1, tc 15
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638954, 63819, 8455, 1, 500); -- PSAS-20 Field Crafting Tool, tier 2, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638955, 63819, 8456, 1, 625); -- PSAS-25 Field Crafting Tool, tier 2, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638956, 63819, 8457, 1, 750); -- PSAS-30 Field Crafting Tool, tier 2, tc 30
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638957, 63819, 8458, 1, 875); -- PSAS-35 Field Crafting Tool, tier 3, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638958, 63819, 8459, 1, 1000); -- PSAS-40 Field Crafting Tool, tier 3, tc 40
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638959, 63819, 8460, 1, 1125); -- PSAS-45 Field Crafting Tool, tier 4, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638960, 63819, 8461, 1, 1250); -- PSAS-50 Field Crafting Tool, tier 5, tc 50
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638961, 63819, 8462, 1, 1375); -- PSAS-55 Field Crafting Tool, tier 5, tc 55
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638962, 63819, 5257, 1, 75); -- Wave Guide (Power Systems), tier 1, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638963, 63819, 5259, 1, 125); -- Particle Guide (Power Systems), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638964, 63819, 5385, 1, 125); -- Optical Fiber (Power Systems Subcombine A), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638965, 63819, 5386, 1, 125); -- Fehu Particle Accelerator (Power Systems Subcombine A), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638966, 63819, 5387, 1, 125); -- Ring Turbine (Power Systems Subcombine A), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638967, 63819, 5388, 1, 200); -- Particle Dynamo (Power Systems Subcombine B), tier 1, tc 8
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638968, 63819, 5389, 1, 200); -- Counterspin Net (Power Systems Subcombine B), tier 1, tc 8
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638969, 63819, 5390, 1, 200); -- Epicycle Multiplier (Power Systems Subcombine B), tier 1, tc 8

-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Crafting Materials, 25 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63820, 'Omega Crafting Materials');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639000, 63820, 8406, 1, 125); -- MAS-5 Field Crafting Tool, tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639001, 63820, 8889, 1, 125); -- Beta Materials Research Kit, tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639002, 63820, 8443, 1, 250); -- MAS-10 Field Crafting Tool, tier 1, tc 10
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639003, 63820, 8444, 1, 375); -- MAS-15 Field Crafting Tool, tier 1, tc 15
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639004, 63820, 8445, 1, 500); -- MAS-20 Field Crafting Tool, tier 2, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639005, 63820, 8446, 1, 625); -- MAS-25 Field Crafting Tool, tier 2, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639006, 63820, 8447, 1, 750); -- MAS-30 Field Crafting Tool, tier 2, tc 30
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639007, 63820, 8448, 1, 875); -- MAS-35 Field Crafting Tool, tier 3, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639008, 63820, 8449, 1, 1000); -- MAS-40 Field Crafting Tool, tier 3, tc 40
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639009, 63820, 8450, 1, 1125); -- MAS-45 Field Crafting Tool, tier 4, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639010, 63820, 8451, 1, 1250); -- MAS-50 Field Crafting Tool, tier 5, tc 50
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639011, 63820, 8452, 1, 1375); -- MAS-55 Field Crafting Tool, tier 5, tc 55
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639012, 63820, 5254, 1, 75); -- Steel Core (Materials), tier 1, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639013, 63820, 5256, 1, 125); -- Titanium Core (Materials), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639014, 63820, 5398, 1, 125); -- Steel Plating (Materials Subcombine A), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639015, 63820, 5399, 1, 125); -- Steel Brushing (Materials Subcombine A), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639016, 63820, 5400, 1, 125); -- Steel Shunt (Materials Subcombine A), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639017, 63820, 5401, 1, 200); -- Titanium Plating (Materials Subcombine B), tier 1, tc 8
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639018, 63820, 5403, 1, 200); -- Titanium Scoring (Materials Subcombine B), tier 1, tc 8
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639019, 63820, 5404, 1, 200); -- Titanium Reinforcement (Materials Subcombine B), tier 1, tc 8
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
