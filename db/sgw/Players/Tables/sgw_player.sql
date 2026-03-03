--
-- TOC entry 270 (class 1259 OID 63757)
-- Name: sgw_player; Type: TABLE; Schema: public; Owner: -; Tablespace: 
--

CREATE TABLE sgw_player (
    account_id integer NOT NULL,
    player_id integer NOT NULL,
    level integer DEFAULT 1 NOT NULL,
    alignment integer NOT NULL,
    archetype integer NOT NULL,
    gender integer NOT NULL,
    player_name character varying(64) DEFAULT NULL::character varying NOT NULL,
    extra_name character varying(64) DEFAULT NULL::character varying NOT NULL,
    world_location character varying(64) DEFAULT NULL::character varying NOT NULL,
    bodyset character varying(64) DEFAULT NULL::character varying NOT NULL,
    title integer DEFAULT 0 NOT NULL,
    pos_x real NOT NULL,
    pos_y real NOT NULL,
    pos_z real NOT NULL,
    heading smallint DEFAULT 0 NOT NULL,
    naquadah integer DEFAULT 0 NOT NULL,
    exp integer DEFAULT 0 NOT NULL,
    first_login integer DEFAULT 1 NOT NULL,
    world_id integer,
    known_stargates integer[] DEFAULT '{}'::integer[] NOT NULL,
    components character varying(200)[] DEFAULT '{}'::character varying[] NOT NULL,
    abilities integer[] DEFAULT '{}'::integer[] NOT NULL,
    access_level integer DEFAULT 0 NOT NULL,
    skin_color_id integer NOT NULL,
    bandolier_slot integer DEFAULT 0 NOT NULL,
    interaction_maps player_interaction_map[],
    training_points integer DEFAULT 0 NOT NULL,
    discipline_ids integer[] DEFAULT '{}'::integer[] NOT NULL,
    racial_paradigm_levels integer[] DEFAULT '{}'::integer[] NOT NULL,
    applied_science_points integer DEFAULT 0 NOT NULL,
    blueprint_ids integer[] DEFAULT '{}'::integer[] NOT NULL,
    known_respawners integer[] DEFAULT '{}'::integer[] NOT NULL,
    CONSTRAINT alignment_sanity CHECK (((alignment >= 0) AND (alignment <= 5))),
    CONSTRAINT archetype_sanity CHECK (((archetype >= 0) AND (archetype <= 8))),
    CONSTRAINT bandolier_slot_sanity CHECK (((bandolier_slot >= 0) AND (bandolier_slot <= 3))),
    CONSTRAINT gender_sanity CHECK (((gender >= 1) AND (gender <= 3))),
    CONSTRAINT level_sanity CHECK (((level >= 0) AND (level <= 20))),
    CONSTRAINT skin_color_sanity CHECK (((skin_color_id >= 0) AND (skin_color_id <= 15)))
);

--
-- TOC entry 2629 (class 2604 OID 63854)
-- Name: player_id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_player ALTER COLUMN player_id SET DEFAULT nextval('sgw_characters_character_id_seq'::regclass);

