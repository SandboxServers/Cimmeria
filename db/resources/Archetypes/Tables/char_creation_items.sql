--
-- Name: char_creation_items; Type: TABLE; Schema: resources; Owner: -
--
-- Items every character of a char_def starts with, on top of the items its
-- visual choices carry (char_creation_choices.item_id). createCharacter
-- places each row with the same bag fill order as the choice items
-- (crates/base/src/base/character_create/starter_kit.rs), so a weapon lands
-- in the bandolier, and loads a weapon's magazine (sgw_inventory.ammo =
-- items.clip_size) so it fires on the first press.
--

CREATE TABLE char_creation_items (
    char_def_id integer NOT NULL,
    item_id integer NOT NULL,
    stack_size integer DEFAULT 1 NOT NULL,
    CONSTRAINT char_creation_items_stack_size_positive CHECK ((stack_size > 0))
);
