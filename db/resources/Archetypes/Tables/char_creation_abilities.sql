--
-- TOC entry 195 (class 1259 OID 62841)
-- Name: char_creation_abilities; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--
-- The abilities a char_def's start profile (char_creation) grants at
-- creation, each with the provenance kind createCharacter records in
-- sgw_player_ability_grants (Class Start v6, CS-02):
--
--   racial_core, signature, tutorial, mission
--       a provenance row of that kind: survives respec and the GM / Debug
--       NPC reset, counts as branch credit (OD-CS06, lock L6)
--   legacy_kit
--       no provenance row and no branch credit, the way every starter was
--       before CS-02. Only the NON_CANONICAL_BLOCKED_LEGACY holding states
--       carry it (OD-CS08, OD-CS09); a canonical profile with a legacy_kit
--       row is refused at load.
--

CREATE TABLE char_creation_abilities (
    char_def_id integer NOT NULL,
    ability_id integer NOT NULL,
    source_kind character varying(16) NOT NULL,
    CONSTRAINT char_creation_abilities_source_kind_check CHECK (((source_kind)::text = ANY ((ARRAY['racial_core'::character varying, 'signature'::character varying, 'tutorial'::character varying, 'mission'::character varying, 'legacy_kit'::character varying])::text[])))
);
