--
-- TOC entry 251 (class 1259 OID 63097)
-- Name: spawn_points; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE spawn_points (
    point_id integer NOT NULL,
    x real NOT NULL,
    y real NOT NULL,
    z real NOT NULL,
    orientation real NOT NULL,
    spawn_set_name character varying(100) NOT NULL,
    spawn_table_name character varying(100) NOT NULL
);

--
-- TOC entry 2885 (class 2604 OID 63184)
-- Name: point_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY spawn_points ALTER COLUMN point_id SET DEFAULT nextval('spawn_points_point_id_seq'::regclass);

