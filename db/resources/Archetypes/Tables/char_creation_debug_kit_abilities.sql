--
-- Name: char_creation_debug_kit_abilities; Type: TABLE; Schema: resources; Owner: -
--
-- The debug kit's abilities (Class Start v6, lock L2). createCharacter adds
-- them, with no provenance row, to a character whose start profile has
-- char_creation.debug_kit = true, and records sgw_player.debug_kit so the GM /
-- Debug NPC reset gives them back. Never chosen from access level. No
-- canonical profile sets the flag; the seeded playtest characters
-- (db/sgw/Players/Seed/sgw_player.sql) are debug-kit characters.
--

CREATE TABLE char_creation_debug_kit_abilities (
    ability_id integer NOT NULL
);
