-- sgw_player_discipline_expertise — per-(player, discipline) expertise level.
--
-- The crafting `sgw_player.discipline_ids integer[]` column captures *which*
-- crafting disciplines a character has unlocked, but not the expertise
-- percentage in each. Python's `Crafter.disciplines` was a `{id -> expertise}`
-- map; storing it as a parallel array on `sgw_player` would require N
-- coordinated updates per gainExpertise call, so we normalise it instead.
--
-- One row per (player, discipline) the player has spent an ASP on.
-- Composite PK `(player_id, discipline_id)` is also the natural lookup key.
-- ON DELETE CASCADE keeps the table consistent when a character is wiped
-- without requiring the deletion path to touch this table explicitly.
--
-- Expertise is bounded to [0, 100] inclusive — matches Python's hard cap of
-- 100 in `Crafter.gainExpertise`. Initial value on learnDiscipline is 1
-- (Python `Crafter.spendAppliedSciencePoints` sets `self.disciplines[id] = 1`),
-- which is the DEFAULT. The CHECK prevents both negative (corruption) and
-- >100 (cap drift) writes from succeeding silently.
CREATE TABLE sgw_player_discipline_expertise (
    player_id     INTEGER NOT NULL,
    discipline_id INTEGER NOT NULL,
    expertise     INTEGER NOT NULL DEFAULT 1
        CHECK (expertise >= 0 AND expertise <= 100),
    PRIMARY KEY (player_id, discipline_id),
    FOREIGN KEY (player_id) REFERENCES sgw_player(player_id) ON DELETE CASCADE
);
