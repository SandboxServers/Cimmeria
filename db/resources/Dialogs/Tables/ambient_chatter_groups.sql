--
-- Ambient chatter groups: NPCs who talk among themselves in say chat on a
-- schedule, for any player standing close enough to hear (the Debug Area's
-- System Lords' summit is the first). One row per group; its lines are in
-- `ambient_chatter_lines`.
--
-- No original data drives this. The 2009 client has no NPC-to-NPC chatter
-- route, and the dialog screens are all addressed to the player, so the
-- groups and their text are Cimmeria seed data. They are read into the cell's
-- startup caches by `load_ambient_chatter`
-- (crates/cell-catalog/src/cell/spawner/ambient_chatter.rs) and spoken by the
-- `cimmeria-cell-chatter` plugin as `onPlayerCommunication` say lines, the
-- route the `npc_bark` content action already uses. No client patch.
--
-- `hear_radius` is how far from a speaking NPC (metres) a player hears its
-- line; an exchange starts only while a player is that close to one of the
-- group's speakers. `exchange_gap_secs` is the quiet time between one
-- exchange's last line and the next exchange's first.
--
-- Name: ambient_chatter_groups; Type: TABLE; Schema: resources; Owner: -
--

CREATE TABLE ambient_chatter_groups (
    group_id integer NOT NULL,
    world_id integer NOT NULL,
    name text NOT NULL,
    hear_radius real DEFAULT 20 NOT NULL,
    exchange_gap_secs integer DEFAULT 30 NOT NULL,
    CONSTRAINT ambient_chatter_groups_hear_radius_positive CHECK (hear_radius > 0 AND hear_radius <= 100),
    CONSTRAINT ambient_chatter_groups_exchange_gap_min CHECK (exchange_gap_secs >= 5)
);
