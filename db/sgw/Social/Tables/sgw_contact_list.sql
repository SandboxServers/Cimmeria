--
-- Named contact lists owned by a player character.
--
-- Each list has a server-assigned id, a player_id owner (FK → sgw_player with
-- cascade delete so lists are destroyed with the character), a display name,
-- and a flags bitmask whose exact semantics are determined client-side.
--
-- Two system lists ("Friends" / "Ignore") are created server-side on first
-- login — not seeded here — so any new character gets them automatically.
--
-- UNIQUE(player_id, name) prevents duplicate list names per player without
-- blocking the same name across different players.
--

CREATE TABLE sgw_contact_list (
    list_id integer NOT NULL DEFAULT nextval('sgw_contact_list_list_id_seq'),
    player_id integer NOT NULL,
    name character varying(128) NOT NULL,
    flags integer DEFAULT 0 NOT NULL
);
