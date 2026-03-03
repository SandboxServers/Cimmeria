--
-- TOC entry 207 (class 1259 OID 62896)
-- Name: entity_interactions; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE entity_interactions (
    entity_interaction_id integer NOT NULL,
    template_id integer NOT NULL,
    interaction_set_map_id integer NOT NULL,
    min_level integer DEFAULT 1 NOT NULL,
    factions "EFaction"[] DEFAULT '{}'::"EFaction"[] NOT NULL,
    alignments "EAlignment"[] DEFAULT '{}'::"EAlignment"[] NOT NULL,
    missions_completed integer[] DEFAULT '{}'::integer[] NOT NULL,
    missions_not_accepted integer[] DEFAULT '{}'::integer[] NOT NULL
);

--
-- TOC entry 2835 (class 2604 OID 63172)
-- Name: entity_interaction_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY entity_interactions ALTER COLUMN entity_interaction_id SET DEFAULT nextval('entity_interactions_entity_interaction_id_seq'::regclass);

