--
-- TOC entry 255 (class 1259 OID 63107)
-- Name: spawnlist; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE spawnlist (
    spawn_id integer NOT NULL,
    x real NOT NULL,
    y real NOT NULL,
    z real NOT NULL,
    heading real NOT NULL,
    world_id integer NOT NULL,
    template_id integer NOT NULL,
    tag character varying(100),
    set_name character varying(100)
);

--
-- TOC entry 2887 (class 2604 OID 63186)
-- Name: spawn_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY spawnlist ALTER COLUMN spawn_id SET DEFAULT nextval('spawnlist_spawn_id_seq'::regclass);

