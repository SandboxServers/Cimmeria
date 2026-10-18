-- Step 4a of the rebuild. Runs in sgw_next right after the fresh seed data load:
-- account holds exactly the seed rows. Move them aside so the carried-over
-- accounts can land in an empty table, then top-up (seed-accounts.sql) decides
-- which seed rows are missing. Plain SQL, no transaction control. No PK needed.
CREATE SCHEMA IF NOT EXISTS cimmeria_seed;
DROP TABLE IF EXISTS cimmeria_seed.account;
CREATE TABLE cimmeria_seed.account AS SELECT * FROM account;
-- CASCADE: sgw_player and the other player tables reference account, and a plain
-- TRUNCATE refuses while a referencing table exists, even an empty one. In the
-- container rebuild those tables are carried and truncated again right after.
TRUNCATE account CASCADE;
