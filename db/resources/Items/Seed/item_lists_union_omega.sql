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
