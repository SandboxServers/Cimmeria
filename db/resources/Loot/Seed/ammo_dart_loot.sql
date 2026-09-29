--
-- NEW CONTENT (debug hub): dart rows for the test-rig crate, loot table 3
-- (ammo campaign AM-11a, issue #1026, the dart extension of D-AM06). Debug
-- hub only, never live content.
--
-- One CO2 Pistol Dartgun (3584, clip 15, widened by ammo_dart_widening.sql
-- to take every dart special type) and one stack of each of the ten dart
-- special ammo items (9005-9014, ammo_items.sql). Every row is probability
-- 1, like the rest of table 3. Quantities are exactly max_stack_size (500),
-- never above it: a grant over the cap writes an over-cap stack (#1045).
--
-- loot_id 9140-9150 is the AM-11a block, clear of the seeded ids (up to 65)
-- and of AM-05's ammo_loot.sql rows.
--

SET search_path = resources, pg_catalog;

INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (9140, 3, 3584, 1, 1, 1);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (9141, 3, 9005, 500, 1, 500);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (9142, 3, 9006, 500, 1, 500);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (9143, 3, 9007, 500, 1, 500);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (9144, 3, 9008, 500, 1, 500);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (9145, 3, 9009, 500, 1, 500);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (9146, 3, 9010, 500, 1, 500);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (9147, 3, 9011, 500, 1, 500);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (9148, 3, 9012, 500, 1, 500);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (9149, 3, 9013, 500, 1, 500);
INSERT INTO loot (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity) VALUES (9150, 3, 9014, 500, 1, 500);
