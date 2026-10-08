-- sgw_player_tutorials — the one-time tutorials a character has been shown
-- (Class Start v6, CS-03).
--
-- A content trigger's `once` flag resets on relog and on every world change,
-- and content counters live in memory, so neither can say "this character
-- has already seen the Equipping a Weapon tutorial". One row per tutorial
-- shown does.
--
-- `tutorial_id` is the dialog id (`resources.dialogs.dialog_id`, a
-- `DUIST_DefaultTutorial` row such as 5882 or 5883). The content action
-- `show_tutorial` inserts the row with ON CONFLICT DO NOTHING and displays
-- the dialog only when the insert added a row, so a replayed chain, a relog
-- or a world change never shows it twice. The condition `tutorial_shown`
-- reads the same set, hydrated onto the cell at world entry.
--
-- The foreign key to sgw_player (ON DELETE CASCADE) lives in
-- db/sgw/_foreign_keys.sql, like sgw_player_ability_grants': sgw_player has
-- no primary key until _primary_keys.sql runs.
CREATE TABLE sgw_player_tutorials (
    player_id   INTEGER NOT NULL,
    tutorial_id INTEGER NOT NULL,
    shown_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (player_id, tutorial_id)
);
