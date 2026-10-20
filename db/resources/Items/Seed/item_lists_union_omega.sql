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
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Human Weapons tier 1, 200 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63800, 'Omega Human Weapons Tier 1');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638000, 63800, 3145, 1, 1000); -- AR 21 Assault Rifle, assault rifle, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638001, 63800, 3126, 1, 600); -- SGHC 6 SMG, SMG, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638002, 63800, 3235, 1, 600); -- SI 3 9mm Pistol, pistol, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638003, 63800, 3313, 1, 1000); -- Pump Shotgun, shotgun, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638004, 63800, 3260, 1, 600); -- SK37 LMG, LMG, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638005, 63800, 3299, 1, 600); -- Nenz 24 Rifle, rifle or sniper, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638006, 63800, 3325, 1, 600); -- Combat Knife, melee, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638007, 63800, 3584, 1, 1000); -- CO2 Pistol Dartgun, dart gun, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638008, 63800, 3173, 1, 1000); -- DMR 12 Assault Rifle, assault rifle, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638009, 63800, 3127, 1, 1000); -- SGHC 6 SMG, SMG, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638010, 63800, 3236, 1, 1000); -- SI 3 9mm Pistol, pistol, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638011, 63800, 3314, 1, 1600); -- Pump Shotgun, shotgun, tc 8
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638012, 63800, 3261, 1, 1000); -- SK37 LMG, LMG, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638013, 63800, 3287, 1, 1000); -- SR1 .50-Cal Rifle, rifle or sniper, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638014, 63800, 3326, 1, 1000); -- Combat Knife, melee, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638015, 63800, 3585, 1, 1600); -- CO2 Pistol Dartgun, dart gun, tc 8
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638016, 63800, 3188, 1, 1000); -- Razor 12 Assault Rifle, assault rifle, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638017, 63800, 3147, 1, 1000); -- MPX 77 SMG, SMG, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638018, 63800, 3249, 1, 1000); -- Mule Pistol, pistol, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638019, 63800, 3315, 1, 2000); -- Pump Shotgun, shotgun, tc 10

-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Human Weapons tier 2, 200 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63801, 'Omega Human Weapons Tier 2');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638050, 63801, 4772, 1, 4000); -- AR 21A Assault Rifle, assault rifle, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638051, 63801, 4739, 1, 1000); -- Kotchner V17 RF SMG, SMG, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638052, 63801, 4883, 1, 4000); -- SI 4 9mm Pistol, pistol, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638053, 63801, 4926, 1, 4000); -- Semi Auto Shotgun, shotgun, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638054, 63801, 4836, 1, 4000); -- SK37 B LMG, LMG, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638055, 63801, 4946, 1, 4000); -- Nenz 25 Rifle, rifle or sniper, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638056, 63801, 5070, 1, 4000); -- Heavy Combat Knife, melee, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638057, 63801, 3591, 1, 4600); -- CO2 Rifle Dartgun, dart gun, tc 23
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638058, 63801, 4997, 1, 4000); -- GL 22 Grenade Launcher, heavy, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638059, 63801, 4787, 1, 4000); -- DMR 21 Assault Rifle, assault rifle, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638060, 63801, 4693, 1, 4000); -- SGHC 6 AP SMG, SMG, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638061, 63801, 4900, 1, 4000); -- Mule MKII Pistol, pistol, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638062, 63801, 3320, 1, 4600); -- Semi Auto Shotgun, shotgun, tc 23
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638063, 63801, 4858, 1, 4000); -- ZEV HE LMG, LMG, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638064, 63801, 4960, 1, 4000); -- SR2 .50-Cal Rifle, rifle or sniper, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638065, 63801, 3334, 1, 5000); -- Heavy Combat Knife, melee, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638066, 63801, 3592, 1, 5000); -- CO2 Rifle Dartgun, dart gun, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638067, 63801, 4998, 1, 4600); -- GL 22 Grenade Launcher, heavy, tc 23
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638068, 63801, 4801, 1, 4000); -- Razor 12 SP Assault Rifle, assault rifle, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638069, 63801, 4708, 1, 4000); -- MPX 77a SMG, SMG, tc 20

-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Human Weapons tier 3, 200 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63802, 'Omega Human Weapons Tier 3');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638100, 63802, 4774, 1, 6600); -- AR 21A TACMOD Assault Rifle, assault rifle, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638101, 63802, 4711, 1, 6600); -- MPX 78 SMG, SMG, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638102, 63802, 4885, 1, 6600); -- SI 5 9mm Pistol, pistol, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638103, 63802, 4928, 1, 6600); -- Combat Shotgun, shotgun, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638104, 63802, 4841, 1, 6600); -- SK38 LMG, LMG, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638105, 63802, 4948, 1, 6600); -- Nenz 25S Rifle, rifle or sniper, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638106, 63802, 5077, 1, 6600); -- Composite Combat Knife, melee, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638107, 63802, 5025, 1, 6600); -- CO2 Semi Auto Dartgun, dart gun, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638108, 63802, 5003, 1, 6600); -- GL 23 Grenade Launcher, heavy, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638109, 63802, 4789, 1, 6600); -- DMR 21 X Assault Rifle, assault rifle, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638110, 63802, 4726, 1, 6600); -- PVR SMG, SMG, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638111, 63802, 4902, 1, 6600); -- Mule MKIII Pistol, pistol, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638112, 63802, 4929, 1, 7000); -- Combat Shotgun, shotgun, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638113, 63802, 4861, 1, 6600); -- ZEV 2 LMG, LMG, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638114, 63802, 4949, 1, 7000); -- Nenz 25S Rifle, rifle or sniper, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638115, 63802, 5078, 1, 7000); -- Composite Combat Knife, melee, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638116, 63802, 5026, 1, 7000); -- CO2 Semi Auto Dartgun, dart gun, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638117, 63802, 5004, 1, 7000); -- GL 23 Grenade Launcher, heavy, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638118, 63802, 4803, 1, 6600); -- Razor Elite Assault Rifle, assault rifle, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638119, 63802, 4744, 1, 6600); -- Kotchner V17 VRF SMG, SMG, tc 33

-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Human Weapons tier 4, 200 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63803, 'Omega Human Weapons Tier 4');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638150, 63803, 4780, 1, 9000); -- AR 200 Assault Rifle, assault rifle, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638151, 63803, 4717, 1, 9000); -- MPX 78 EX SMG, SMG, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638152, 63803, 4891, 1, 9000); -- SI-P 5 9mm Pistol, pistol, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638153, 63803, 4933, 1, 9000); -- Assault Shotgun, shotgun, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638154, 63803, 4849, 1, 9000); -- SK38 X LMG, LMG, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638155, 63803, 4954, 1, 9000); -- Nenz 25S-II Rifle, rifle or sniper, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638156, 63803, 5083, 1, 9000); -- Microblade Combat Knife, melee, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638157, 63803, 5034, 1, 9000); -- CO2 ACR Dartgun, dart gun, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638158, 63803, 5009, 1, 9000); -- GL 24 Grenade Launcher, heavy, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638159, 63803, 4795, 1, 9000); -- DMR 210 Assault Rifle, assault rifle, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638160, 63803, 4732, 1, 9000); -- PVK New SMG, SMG, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638161, 63803, 4909, 1, 9000); -- Big Mule Pistol, pistol, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638162, 63803, 4934, 1, 9600); -- Assault Shotgun, shotgun, tc 48
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638163, 63803, 4869, 1, 9000); -- ZEV 3 LMG, LMG, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638164, 63803, 4955, 1, 9600); -- Nenz 25S-II Rifle, rifle or sniper, tc 48
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638165, 63803, 5085, 1, 10000); -- Microblade Combat Knife, melee, tc 50
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638166, 63803, 5035, 1, 9600); -- CO2 ACR Dartgun, dart gun, tc 48
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638167, 63803, 5010, 1, 9600); -- GL 24 Grenade Launcher, heavy, tc 48
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638168, 63803, 4809, 1, 9000); -- Razor X Assault Rifle, assault rifle, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638169, 63803, 4754, 1, 9000); -- Kotchner V17 X SMG, SMG, tc 45
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
