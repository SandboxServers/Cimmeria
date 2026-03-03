--
-- TOC entry 200 (class 1259 OID 62864)
-- Name: dialog_set_maps; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE dialog_set_maps (
    dialog_set_map_id integer NOT NULL,
    dialog_set_id integer NOT NULL,
    dialog_id integer,
    topic_text character varying(200) NOT NULL,
    interaction_flags bigint NOT NULL,
    min_level integer DEFAULT 1 NOT NULL,
    missions_completed integer[] DEFAULT '{}'::integer[] NOT NULL,
    missions_not_accepted integer[] DEFAULT '{}'::integer[] NOT NULL,
    alignments integer[] DEFAULT '{}'::integer[] NOT NULL,
    factions integer[] DEFAULT '{}'::integer[] NOT NULL
);

--
-- TOC entry 2821 (class 2604 OID 63170)
-- Name: dialog_set_map_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY dialog_set_maps ALTER COLUMN dialog_set_map_id SET DEFAULT nextval('dialog_set_maps_dialog_set_map_id_seq'::regclass);

