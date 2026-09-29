--
-- Widen every dart gun that takes Dart_Default (19 ids: CO2 Pistol, CO2
-- Semi Auto, CO2 ACR and Gauss Dartguns) to accept all ten dart special
-- types (ammo campaign AM-11a, issue #1026). The CO2 Rifle Dartguns list
-- Bullet_Default in the seed and are left alone.
--
-- Same shape as ammo_weapon_widening.sql: an UPDATE in its own file, so no
-- packet edits items.sql. The client's ammo picker reads the live container
-- cache the server fills from resources.items.ammo_types, so this needs no
-- client patch (docs/reverse-engineering/findings/ammo-system.md Q2). The
-- WHERE on unnest skips a type a row already lists, so no array holds a
-- duplicate.
--

SET search_path = resources, pg_catalog;

UPDATE items
   SET ammo_types = ammo_types || ARRAY(
           SELECT t
             FROM unnest(ARRAY['Dart_Poison', 'Dart_Disease', 'Dart_Tranquilizer',
                               'Dart_EMP', 'Dart_Radioactive', 'Dart_Stim',
                               'Dart_Coagulant', 'Dart_Nanites', 'Dart_Antidote',
                               'Dart_Adrenaline']::"EAmmoType"[]) AS t
            WHERE NOT (t = ANY (items.ammo_types))
       )
 WHERE 'Dart_Default' = ANY (ammo_types);
