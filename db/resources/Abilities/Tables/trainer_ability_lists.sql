--
-- TOC entry 264 (class 1259 OID 63142)
-- Name: trainer_ability_lists; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE trainer_ability_lists (
    list_id integer NOT NULL,
    description character varying(200)
);

--
-- TOC entry 2889 (class 2604 OID 63187)
-- Name: list_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY trainer_ability_lists ALTER COLUMN list_id SET DEFAULT nextval('trainer_ability_lists_list_id_seq'::regclass);

