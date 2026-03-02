--
-- TOC entry 239 (class 1259 OID 63046)
-- Name: point_sets; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE point_sets (
    set_id integer NOT NULL,
    name character varying(100) NOT NULL,
    type character varying(100) NOT NULL,
    world_id integer NOT NULL,
    radius real,
    height real,
    shape character varying(100) NOT NULL,
    flags integer DEFAULT 1 NOT NULL
);

--
-- TOC entry 2878 (class 2604 OID 63182)
-- Name: set_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY point_sets ALTER COLUMN set_id SET DEFAULT nextval('point_sets_set_id_seq'::regclass);

