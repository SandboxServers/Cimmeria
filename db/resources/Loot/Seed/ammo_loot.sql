--
-- Special-ammo loot rows (ammo campaign AM-05, issue #1026).
--
-- Additive: every row here joins a loot table that loot.sql / loot_tables.sql
-- already seed, and none of those rows changes. Loaded after loot.sql, so
-- loot_id comes from loot_loot_id_seq (loot.sql leaves it at its last id)
-- and a new loot.sql row that bumps the setval never collides with these.
--
-- Ammo items are always named through resources.ammo_item_types, never by
-- id (AM-F: a re-seed of the 9000-9014 block touches only the two AM-F seed
-- files), and quantities are rounds (D-AM05), capped by the item's own
-- max_stack_size.
--
-- The loot guards are in cell-catalog spawner/tests/live_db_ammo_loot.rs.
--

SET search_path = resources, pg_catalog;

-- D-AM06, the debug hub crate (table 3, chain 7020, re-rolled on every
-- open): a full stack of every bullet special at probability 1, so one open
-- gives a tester every type. The dart gun and the ten dart stacks are in
-- ammo_dart_loot.sql (AM-11a). Debug hub only.
INSERT INTO loot (loot_table_id, design_id, min_quantity, probability, max_quantity)
SELECT 3, a.item_id, i.max_stack_size, 1, i.max_stack_size
  FROM ammo_item_types a
  JOIN items i ON i.item_id = a.item_id
 WHERE a.ammo_type IN ('Bullet_Hollow_Point', 'Bullet_Armor_Piercing',
                       'Bullet_Incendiary', 'Bullet_EMP', 'Bullet_Explosive')
 ORDER BY a.ammo_type;

-- D-AM06: a pistol and an SMG that accept all five (both are in AM-F's
-- widened families, ammo_weapon_widening.sql). 3235 is an SI 3 9mm Pistol,
-- the tier-1 Standard Pistol with the widest discipline list; 3147 is an
-- MPX 77 SMG, the lowest-tech Standard SMG. One each, probability 1.
INSERT INTO loot (loot_table_id, design_id, min_quantity, probability, max_quantity)
VALUES (3, 3235, 1, 1, 1),
       (3, 3147, 1, 1, 1);

-- D-AM03, the Castle pre-Romney chest (tables 8 non-Jaffa and 9 Jaffa, once
-- per character, mission 703 active): 50-75 Hollow Point rounds, certain,
-- beside the #1031 rows. The non-Jaffa chest's SGHC 6 SMG (3127) takes them:
-- the High Capacity SMG family was widened by the D-AM10 amendment
-- (ammo_weapon_widening.sql, #1052). The Jaffa chest's Serpent Staff (3472)
-- does not, so a Jaffa uses them in a Standard Pistol or SMG.
INSERT INTO loot (loot_table_id, design_id, min_quantity, probability, max_quantity)
SELECT t.loot_table_id, a.item_id, 50, 1, 75
  FROM ammo_item_types a
 CROSS JOIN (VALUES (8), (9)) AS t (loot_table_id)
 WHERE a.ammo_type = 'Bullet_Hollow_Point'
 ORDER BY t.loot_table_id;

-- NPC drops (#1026 "ammo items drop from NPC loot tables"), Castle NID only:
-- they are the Castle's gun-carrying humans and its tables are where a
-- level 3-4 player meets Standard Pistols and SMGs. Hollow Point and Armor
-- Piercing only (D-AM04's first pair); Incendiary, EMP and Explosive come
-- only from the debug crate and .giveammo, and no NPC table drops darts.
-- Not on table 2 (the
-- Cellblock tutorial guard: nothing there fires bullets) nor 6 (PRU drones:
-- salvage only).
--
--   table 4  Castle NID guard       Hollow Point    5 %  10-25 rounds
--   table 5  Castle NID veteran     Hollow Point    6 %  15-30 rounds
--                                   Armor Piercing  3 %  10-20 rounds
--   table 7  Castle hall NID guard  Hollow Point   10 %  10-25 rounds
--
-- Chance a corpse drops nothing, after these rows (loot.sql's figures times
-- the new misses; each stays inside live_db_castle_loot.rs's band):
--   table 4  30.5 % * 0.95        = 29.0 %  (band 25-35 %)
--   table 5  17.5 % * 0.94 * 0.97 = 16.0 %  (band 15-20 %)
--   table 7  35 %   * 0.90        = 31.5 %  (band 30-40 %)
INSERT INTO loot (loot_table_id, design_id, min_quantity, probability, max_quantity)
SELECT d.loot_table_id, a.item_id, d.min_quantity, d.probability, d.max_quantity
  FROM (VALUES (4, 'Bullet_Hollow_Point'::"EAmmoType", 10, 0.05::real, 25, 1),
               (5, 'Bullet_Hollow_Point'::"EAmmoType", 15, 0.06::real, 30, 2),
               (5, 'Bullet_Armor_Piercing'::"EAmmoType", 10, 0.03::real, 20, 3),
               (7, 'Bullet_Hollow_Point'::"EAmmoType", 10, 0.10::real, 25, 4))
       AS d (loot_table_id, ammo_type, min_quantity, probability, max_quantity, ord)
  JOIN ammo_item_types a ON a.ammo_type = d.ammo_type
 ORDER BY d.ord;
