--
-- Name: char_creation_debug_kit_items; Type: TABLE; Schema: resources; Owner: -
--
-- The debug kit's items (Class Start v6, lock L2): placed like
-- char_creation_items, after them. A gun starts empty, like every gun
-- (OD-CS13 amendment, 2026-10-05).
--

CREATE TABLE char_creation_debug_kit_items (
    item_id integer NOT NULL,
    stack_size integer DEFAULT 1 NOT NULL,
    CONSTRAINT char_creation_debug_kit_items_stack_size_positive CHECK ((stack_size > 0))
);
