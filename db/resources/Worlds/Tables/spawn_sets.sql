--
-- TOC entry 253 (class 1259 OID 63102)
-- Name: spawn_sets; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE spawn_sets (
    set_id integer NOT NULL,
    name character varying(100) NOT NULL,
    type character varying(100) NOT NULL,
    world_id integer NOT NULL,
    height real,
    radius real
);

--
-- TOC entry 2886 (class 2604 OID 63185)
-- Name: set_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY spawn_sets ALTER COLUMN set_id SET DEFAULT nextval('spawn_sets_set_id_seq'::regclass);

