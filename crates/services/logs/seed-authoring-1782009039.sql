-- [savespawn] -> db/resources/X.sql
DELETE FROM x;
-- ===== CONFIRMED by entity 7 -> commit into db/resources/X.sql =====
DELETE FROM x;
-- [savespawn] -> db/resources/X.sql
DELETE FROM x;
-- [delspawn] -> db/resources/Worlds/Seed/spawnlist.sql
DELETE FROM resources.spawnlist WHERE spawn_id = 42;
