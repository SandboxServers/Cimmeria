--
-- TOC entry 235 (class 1259 OID 63033)
-- Name: paths; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE paths (
    path_id integer NOT NULL,
    x real NOT NULL,
    y real NOT NULL,
    z real NOT NULL,
    index integer NOT NULL
);

--
-- TOC entry 2872 (class 2604 OID 63180)
-- Name: path_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY paths ALTER COLUMN path_id SET DEFAULT nextval('paths_path_id_seq'::regclass);

