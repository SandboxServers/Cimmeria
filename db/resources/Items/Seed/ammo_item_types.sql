--
-- resources.ammo_item_types seed (ammo campaign AM-F, issue #1026): each
-- bullet and dart special type to its reserve item in ammo_items.sql.
-- Dagger_* types have no rows: daggers are a later wave (D-AM08).
--

SET search_path = resources, pg_catalog;

INSERT INTO ammo_item_types (ammo_type, item_id) VALUES
    ('Bullet_Armor_Piercing', 9000),
    ('Bullet_Hollow_Point', 9001),
    ('Bullet_Incendiary', 9002),
    ('Bullet_EMP', 9003),
    ('Bullet_Explosive', 9004),
    ('Dart_Poison', 9005),
    ('Dart_Disease', 9006),
    ('Dart_Tranquilizer', 9007),
    ('Dart_EMP', 9008),
    ('Dart_Radioactive', 9009),
    ('Dart_Stim', 9010),
    ('Dart_Coagulant', 9011),
    ('Dart_Nanites', 9012),
    ('Dart_Antidote', 9013),
    ('Dart_Adrenaline', 9014);
