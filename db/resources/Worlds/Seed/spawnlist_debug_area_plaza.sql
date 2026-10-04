--
-- NEW CONTENT (Debug Area, DA-02): the services plaza (Z2) and the dummies
-- range (Z3) of world 1300 `DebugArea` (Ihpet_Crater_Light).
-- docs/content/debug-area.md has the table and what each NPC tests.
--
-- Id block 13000-13199 (DA-02): plaza 13001-13022, dummies 13100-13104.
-- Tags `DebugArea_*` (D-DA6), never `DebugHub_*`: the stasis hub's tests
-- count that prefix.
--
-- Plaza: centre (252.0, 7.0, -923.0), kept clear for 12 m. Twenty-one NPCs on
-- a 10.5 m ring, each facing the centre, about 2.9 m apart. The ring has a 30
-- degree gap to the south, towards the Z1 arrival, and one to the north,
-- towards the dummies, so a tester walks in from the arrival and out to the
-- range. The east arc (x > 252), south to north: ability granter, ability
-- reset, trainer, pet trainer, vendor, munitions vendor, loot crate, Livewire
-- terminal, dialog NPC, mail clerk, Black Market auctioneer. The west arc,
-- south to north: Storage Officer, Team and Command Bankers, Team and Command
-- registrars, crafting supplies vendor, then the four crafting stations, which
-- stand within 5 m of one another so one spot reaches them all.
--
-- Dummies: one line at z -872, x 240 to 264, 6 m apart, facing south towards
-- the plaza: hostile levels 1, 10, 25 and 50, then the friendly heal target.
--
-- Heights are navmesh floor heights (nav_inspect against
-- data/spaces/ihpet_crater_light.nav): the plaza floor is one open walkable
-- region (polys 3455, 3456, 3460, 3464, 3639) at 6.99 to 7.10, the dummies'
-- strip 6.59 to 7.08. The plaza interior agrees with the occluder terrain to
-- about 1 m (the campaign survey); the live client has not seen them yet
-- (DA-06).
--
-- Every spawn is stationary: none of them walks, and the flag keeps them out
-- of the spawner's off-mesh check. The plaza NPCs mostly reuse the stasis
-- hub's templates, so a service behaves the same in both places; the
-- chain-driven ones answer through their own `DebugArea_*` tags
-- (debug_area_plaza_chains.sql).
--

SET search_path = resources, pg_catalog;

-- Plaza, east arc.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13020, 254.72, 7.1, -933.14, -0.2618, 1300, 1300, 'DebugArea_AbilityGranter', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13021, 257.25, 6.99, -932.09, -0.5236, 1300, 1301, 'DebugArea_AbilityReset', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13002, 259.42, 6.99, -930.42, -0.7854, 1300, 301, 'DebugArea_Trainer', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13007, 261.09, 6.99, -928.25, -1.0472, 1300, 360, 'DebugArea_PetTrainer', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13001, 262.14, 6.99, -925.72, -1.309, 1300, 300, 'DebugArea_Vendor', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13006, 262.5, 6.99, -923.0, -1.5708, 1300, 1302, 'DebugArea_MunitionsVendor', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13005, 262.14, 6.99, -920.28, -1.8326, 1300, 304, 'DebugArea_LootCrate', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13004, 261.09, 6.99, -917.75, -2.0944, 1300, 303, 'DebugArea_LivewireTerminal', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13003, 259.42, 6.99, -915.58, -2.3562, 1300, 302, 'DebugArea_DialogNpc', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13011, 257.25, 6.99, -913.91, -2.618, 1300, 390, 'DebugArea_MailClerk', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13012, 254.72, 6.99, -912.86, -2.8798, 1300, 305, 'DebugArea_Auctioneer', NULL, true);

-- Plaza, west arc.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13008, 249.28, 7.06, -933.14, 0.2618, 1300, 370, 'DebugArea_Banker', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13009, 246.49, 6.99, -931.94, 0.5527, 1300, 371, 'DebugArea_TeamBanker', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13010, 244.16, 6.99, -929.98, 0.8436, 1300, 372, 'DebugArea_CommandBanker', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13013, 242.48, 6.99, -927.44, 1.1345, 1300, 330, 'DebugArea_TeamRegistrar', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13014, 241.61, 6.99, -924.52, 1.4254, 1300, 331, 'DebugArea_CommandRegistrar', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13019, 241.61, 6.99, -921.48, 1.7162, 1300, 314, 'DebugArea_CraftSupplies', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13015, 242.48, 6.99, -918.56, 2.0071, 1300, 310, 'DebugArea_Station_BioMedical', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13016, 244.16, 6.99, -916.02, 2.298, 1300, 311, 'DebugArea_Station_Electronics', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13017, 246.49, 6.99, -914.06, 2.5889, 1300, 312, 'DebugArea_Station_PowerSystems', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13018, 249.28, 6.99, -912.86, 2.8798, 1300, 313, 'DebugArea_Station_Materials', NULL, true);

-- Dummies range (D-DA7). Templates 1310-1314 carry `training_dummy`.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13100, 240.0, 7.08, -872.0, 3.1416, 1300, 1310, 'DebugArea_Dummy_L1', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13101, 246.0, 6.59, -872.0, 3.1416, 1300, 1311, 'DebugArea_Dummy_L10', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13102, 252.0, 6.59, -872.0, 3.1416, 1300, 1312, 'DebugArea_Dummy_L25', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13103, 258.0, 6.7, -872.0, 3.1416, 1300, 1313, 'DebugArea_Dummy_L50', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13104, 264.0, 6.86, -872.0, 3.1416, 1300, 1314, 'DebugArea_Dummy_Friendly', NULL, true);

-- These rows load after spawnlist.sql's own sequence footer, so they raise
-- the sequence themselves. Floor 13799 is the top of the whole Debug Area
-- reservation (DA-02..04, spawns 13000-13799), so a `.savespawn` row never
-- takes a reserved id.
SELECT pg_catalog.setval('spawnlist_spawn_id_seq', GREATEST((SELECT MAX(spawn_id) FROM spawnlist), (SELECT last_value FROM spawnlist_spawn_id_seq), 13799), true);
