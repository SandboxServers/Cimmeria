-- sgw_player_content_cooldown — per-(player, key) cooldowns for content
-- actions that give something away (social-systems SS-U3).
--
-- The first user is the stasis-room Gate Mail Clerk's `send_system_mail`
-- action: one test mail (a stack of Health Slappacks and 50 naquadah) per
-- player every 10 minutes. The cooldown cannot live on the mail itself: a
-- player who takes the attachments may delete the mail, and a check against
-- sgw_gate_mail would then let them ask again at once. A row here survives
-- both the mail's deletion and a server restart.
--
-- The key names the action (`send_system_mail/<chain_id>`), so two chains
-- never share a window. The base claims a cooldown with one conditional
-- upsert in the same transaction as the mail it guards, so a refused or
-- rolled-back mail leaves the previous claim in place.
--
-- The foreign key to sgw_player (ON DELETE CASCADE) lives in
-- db/sgw/_foreign_keys.sql, like sgw_player_discipline_expertise's: sgw_player
-- has no primary key until _primary_keys.sql runs.
CREATE TABLE sgw_player_content_cooldown (
    player_id    INTEGER NOT NULL,
    cooldown_key VARCHAR(64) NOT NULL,
    -- Epoch seconds, like sgw_gate_mail.sent_time.
    last_used_at INTEGER NOT NULL,
    PRIMARY KEY (player_id, cooldown_key)
);
