-- Migration: bump Health Slappack TC1 (items 2893 and 4735) to
-- max_stack_size = 10 and swap the placeholder
-- 'set:CoreWidgets image:IconMissing' icon for the Medkit sprite
-- (`set:ItemIcon001 image:Medkit`) already used by other healing
-- consumables (e.g. Plasma Burn Treatment Kit, Evac-U-Pac).
--
-- Item 2893 is the canonical drop / consumable referenced by the
-- loot table (see db/scripts/add_cellblock_guard_slappack_loot.sql)
-- and the use chain (see db/scripts/add_health_slappack_use_chain.sql).
-- Item 4735 is a higher-tech-comp craftable duplicate with the same
-- display name; both rows are kept in lockstep so the in-inventory
-- display doesn't fork by source.
--
-- Mirrors the post-change seed in
-- `db/resources/Items/Seed/items.sql` so a fresh load and an
-- in-place upgrade converge to the same row shape.
--
-- Safe to re-run — the WHERE clause filters out rows already at the
-- target values, so a repeat invocation is a no-op (`rows_affected = 0`
-- is expected on the second run).

\set ON_ERROR_STOP on

BEGIN;

UPDATE resources.items
   SET max_stack_size = 10,
       icon_location  = 'set:ItemIcon001 image:Medkit'
 WHERE item_id IN (2893, 4735)
   AND (max_stack_size <> 10 OR icon_location <> 'set:ItemIcon001 image:Medkit');

COMMIT;
