--
-- TOC entry 306 (class 1259 OID 64199)
-- Name: effect_nvps; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE effect_nvps (
    nvp_id integer NOT NULL,
    effect_id integer NOT NULL,
    name character varying(100) NOT NULL,
    value character varying(100) NOT NULL
);

--
-- TOC entry 2911 (class 2604 OID 64202)
-- Name: nvp_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY effect_nvps ALTER COLUMN nvp_id SET DEFAULT nextval('effect_nvps_2_nvp_id_seq'::regclass);

