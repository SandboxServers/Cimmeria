-- Migration: add Health Slappack TC1 as a guaranteed drop on the shared
-- Cellblock guard loot table (loot_table_id = 2).
--
-- Castle CellBlock guards (template_id 15 "Cellblock Guard" and 24 "NID
-- Guard") both reference loot_table_id = 2, which previously held only
-- one entry — naquadah/credits at 80% chance. Without a healing
-- consumable in the drop pool, players running back-to-back guard
-- encounters can't recover fast enough between fights even with
-- out-of-combat regen. Item 2893 ("Health Slappack TC1, +500 Health",
-- resources.items.item_id = 2893) is the canonical TC1 consumable, so
-- we attach it as a 100% drop.
--
-- Convergent upsert (DO UPDATE): if loot_id 13 already exists with
-- different values (e.g. a stale row from an aborted earlier run), this
-- migration corrects them rather than silently leaving the wrong shape
-- in place. The sequence bump still gates on the current last_value so
-- it's a no-op when already advanced.
--
-- Safe to re-run.

\set ON_ERROR_STOP on

INSERT INTO resources.loot
    (loot_id, loot_table_id, design_id, min_quantity, probability, max_quantity)
VALUES
    (13, 2, 2893, 1, 1.0, 1)
ON CONFLICT (loot_id) DO UPDATE SET
    loot_table_id = EXCLUDED.loot_table_id,
    design_id     = EXCLUDED.design_id,
    min_quantity  = EXCLUDED.min_quantity,
    probability   = EXCLUDED.probability,
    max_quantity  = EXCLUDED.max_quantity;

-- Bump the sequence past the row we just inserted so future seed reloads
-- don't reuse loot_id 13. setval(..., is_called=true) means the *next*
-- nextval() returns 14.
SELECT setval('resources.loot_loot_id_seq', 13, true)
WHERE (SELECT last_value FROM resources.loot_loot_id_seq) < 13;
