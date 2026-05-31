-- Migration: add sgw_player_discipline_expertise table for crafting expertise.
-- Safe to run on existing databases (uses IF NOT EXISTS).
-- See db/sgw/Players/Tables/sgw_player_discipline_expertise.sql for the
-- canonical definition and rationale.
--
-- The crafting Phase 1 port needs per-(player, discipline) expertise storage.
-- The existing `sgw_player.discipline_ids integer[]` column captures *which*
-- disciplines a character knows, but not the expertise percentage in each.

CREATE TABLE IF NOT EXISTS sgw_player_discipline_expertise (
    player_id     INTEGER NOT NULL,
    discipline_id INTEGER NOT NULL,
    expertise     INTEGER NOT NULL DEFAULT 1
        CHECK (expertise >= 0 AND expertise <= 100),
    PRIMARY KEY (player_id, discipline_id),
    FOREIGN KEY (player_id) REFERENCES sgw_player(player_id) ON DELETE CASCADE
);
