-- Migration: add cur_ammo_type column to sgw_inventory for per-slot ammo subtype.
--
-- Players can pick a non-default ammo subtype per bandolier weapon (e.g. AP
-- rounds vs. tracer for the same rifle). Persisting this requires a column
-- per inventory row — overloading the existing `flags` bitfield was rejected
-- as too opaque.
--
-- A value of 0 means "no explicit choice" and the load path falls back to the
-- item's `default_ammo_type` (resources.items.default_ammo_type). Safe to run
-- on existing databases (uses IF NOT EXISTS).

ALTER TABLE sgw_inventory
    ADD COLUMN IF NOT EXISTS cur_ammo_type integer NOT NULL DEFAULT 0;
