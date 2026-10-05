--
-- TOC entry 194 (class 1259 OID 62838)
-- Name: char_creation; Type: TABLE; Schema: resources; Owner: -; Tablespace:
--
-- One row per client CharDefId: the character's identity (alignment,
-- archetype, body_set, gender; these must match
-- crates/resources/src/base/chardef.rs) and its start profile (Class Start
-- v6, CS-02). createCharacter reads the profile, and nothing else, for:
--
--   profile_id      the ledger's ProfileID (PRA_OPCORE_SOLDIER, SGU_FREE_JAFFA, ...)
--   starting_world  the start world; creation refuses a world that is not in
--                   resources.worlds or has no loaded cell space (lock L3)
--   starting_x/y/z  the spawn point (world entry sends no facing)
--   start_level     the level the character is created at. Never derived from
--                   a mission's seeded level. Training and Applied Science
--                   points follow it (one of each per level).
--   debug_kit       lock L2: add the debug kit (char_creation_debug_kit_*).
--                   Never derived from access level. False on every
--                   canonical row.
--   start_state     CANONICAL, or NON_CANONICAL_BLOCKED_LEGACY for a holding
--                   state kept literally while its real start is blocked
--                   (Goa'uld OD-CS08, Asgard OD-CS09)
--
-- The profile's abilities are char_creation_abilities (each with a
-- provenance source_kind) and its items char_creation_items.
-- Ledger: docs/analysis/class-start-v6/README.md.
--

CREATE TABLE char_creation (
    char_def_id integer NOT NULL,
    alignment "EAlignment" NOT NULL,
    archetype "EArchetype" NOT NULL,
    body_set character varying(255) NOT NULL,
    gender "EGender" NOT NULL,
    starting_world character varying(100) NOT NULL,
    starting_x real NOT NULL,
    starting_y real NOT NULL,
    starting_z real NOT NULL,
    profile_id character varying(64) NOT NULL,
    start_level integer DEFAULT 1 NOT NULL,
    debug_kit boolean DEFAULT false NOT NULL,
    start_state character varying(32) DEFAULT 'CANONICAL' NOT NULL,
    CONSTRAINT char_creation_start_level_check CHECK (((start_level >= 1) AND (start_level <= 50))),
    CONSTRAINT char_creation_start_state_check CHECK (((start_state)::text = ANY ((ARRAY['CANONICAL'::character varying, 'NON_CANONICAL_BLOCKED_LEGACY'::character varying])::text[])))
);

