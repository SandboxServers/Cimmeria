--
-- TOC entry 184 (class 1259 OID 62794)
-- Name: ability_sets; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE ability_sets (
    ability_set_id integer NOT NULL,
    description character varying(300)
);

--
-- TOC entry 2814 (class 2604 OID 63169)
-- Name: ability_set_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY ability_sets ALTER COLUMN ability_set_id SET DEFAULT nextval('ability_sets2_ability_set_id_seq'::regclass);

