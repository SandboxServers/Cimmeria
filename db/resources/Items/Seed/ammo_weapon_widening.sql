--
-- Widen the Standard Pistol (27 ids) and Standard SMG (25 ids) families to
-- accept all five bullet special types (ammo campaign AM-F, D-AM10; family
-- lists in docs/analysis/ammo/audit.md section 6).
--
-- D-AM10 amendment (@Cadacious, 2026-09-28): the High Capacity SMG family
-- (27 ids, the SGHC 6 / 6 AP / 7 / X / Gauss SMG line: starter item 21 and
-- the Castle pre-Romney chest weapon 3127 among them) accepts the same five
-- types, so the Hollow Point the Castle chest hands out fits the gun that
-- same chest hands out.
--
-- An UPDATE in its own file rather than an edit of items.sql, so no packet
-- ever conflicts in that file. The client's ammo picker reads the live
-- container cache the server fills from resources.items.ammo_types, so this
-- needs no client patch (ammo-system.md Q2).
--
-- The WHERE clause skips a row that already lists a type, so the arrays
-- never hold a duplicate.
--

SET search_path = resources, pg_catalog;

UPDATE items
   SET ammo_types = ammo_types || ARRAY(
           SELECT t
             FROM unnest(ARRAY['Bullet_Armor_Piercing', 'Bullet_Hollow_Point',
                               'Bullet_Incendiary', 'Bullet_EMP',
                               'Bullet_Explosive']::"EAmmoType"[]) AS t
            WHERE NOT (t = ANY (items.ammo_types))
       )
 WHERE description IN ('Standard Pistol', 'Standard SMG', 'High Capacity SMG');
