--
-- TOC entry 215 (class 1259 OID 62939)
-- Name: generic_regions; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE generic_regions (
    region_id integer NOT NULL,
    height real NOT NULL,
    radius real NOT NULL,
    flags integer NOT NULL,
    handler character varying(200) NOT NULL,
    x1 real NOT NULL,
    y1 real NOT NULL,
    z1 real NOT NULL,
    x2 real,
    y2 real,
    z2 real,
    x3 real,
    y3 real,
    z3 real,
    x4 real,
    y4 real,
    z4 real,
    world_id integer NOT NULL,
    tag character varying(100)
);

--
-- TOC entry 2845 (class 2604 OID 63174)
-- Name: region_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY generic_regions ALTER COLUMN region_id SET DEFAULT nextval('generic_regions_region_id_seq'::regclass);

