--
-- Members of a contact list — stored by player name string, NOT by player_id,
-- because: (1) the wire protocol sends names not ids; (2) members can refer to
-- deleted or not-yet-created characters without orphaning rows; (3) the original
-- SGW ContactListManager operated on names.
--
-- (list_id, player_name) is the composite PK — a player may appear in the same
-- list only once. Multiple lists owned by the same character can each contain
-- the same member name; that is intentional (you can have the same person in
-- Friends AND a custom list).
--

CREATE TABLE sgw_contact_list_member (
    list_id integer NOT NULL,
    player_name character varying(128) NOT NULL,
    CONSTRAINT sgw_contact_list_member_player_name_nonempty CHECK (char_length(player_name) > 0)
);
