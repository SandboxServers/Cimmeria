-- sgw_player_ability_grants — where a character's free (non-trained)
-- abilities came from (Class Start v6, CS-01a, lock L6).
--
-- sgw_player.abilities is the known set and sgw_player.trained_abilities the
-- trainer purchases. An ability granted by content (a tutorial, a racial
-- core, a class signature, a mission reward) or by a GM is in `abilities`
-- only, and without this table nothing tells it apart from a starter or a
-- legacy grant. One row per granted ability says which:
--
-- | source_kind  | written by                                   | GM / Debug NPC reset |
-- |--------------|----------------------------------------------|----------------------|
-- | tutorial     | the `grant_ability` content action           | kept                 |
-- | racial_core  | a start profile or the content action        | kept                 |
-- | signature    | the content action (class identity reward)   | kept                 |
-- | mission      | the content action                           | kept                 |
-- | gm           | .giveability, GM grant-all, Debug NPC grant  | removed with the row |
--
-- Every kind survives a trainer respec, which strips trained_abilities only.
-- A non-gm row also counts as branch credit: the trainer's spend gate adds
-- the granted tree node's skill_point_cost to tree_points_spent (OD-CS06).
-- A trained ability never has a row. Characters created before this table
-- start with none (D-AT11).
--
-- `source_id` is the mission or chain id the grant came from, or NULL.
--
-- The foreign key to sgw_player (ON DELETE CASCADE) lives in
-- db/sgw/_foreign_keys.sql, like sgw_player_content_cooldown's: sgw_player
-- has no primary key until _primary_keys.sql runs.
CREATE TABLE sgw_player_ability_grants (
    player_id   INTEGER NOT NULL,
    ability_id  INTEGER NOT NULL,
    source_kind VARCHAR(16) NOT NULL
        CONSTRAINT sgw_player_ability_grants_source_kind_check
        CHECK (source_kind IN ('tutorial', 'racial_core', 'signature', 'mission', 'gm')),
    source_id   INTEGER,
    granted_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (player_id, ability_id)
);
