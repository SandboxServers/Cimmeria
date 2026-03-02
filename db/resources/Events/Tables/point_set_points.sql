--
-- TOC entry 237 (class 1259 OID 63038)
-- Name: point_set_points; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE point_set_points (
    set_id integer NOT NULL,
    point_id integer NOT NULL,
    x real NOT NULL,
    y real NOT NULL,
    z real NOT NULL,
    yaw real DEFAULT 0 NOT NULL,
    pitch real DEFAULT 0 NOT NULL,
    roll real DEFAULT 0 NOT NULL
);

--
-- TOC entry 2876 (class 2604 OID 63181)
-- Name: point_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY point_set_points ALTER COLUMN point_id SET DEFAULT nextval('point_set_points_point_id_seq'::regclass);

