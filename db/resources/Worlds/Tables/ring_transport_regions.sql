--
-- TOC entry 245 (class 1259 OID 63070)
-- Name: ring_transport_regions; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE ring_transport_regions (
    region_id integer NOT NULL,
    world_id integer NOT NULL,
    x real NOT NULL,
    y real NOT NULL,
    z real NOT NULL,
    tag character varying(100) NOT NULL,
    height real NOT NULL,
    radius real NOT NULL,
    event_set_id integer NOT NULL,
    display_name_id integer NOT NULL,
    destination_region_ids integer[] DEFAULT '{}'::integer[] NOT NULL,
    point_set_id integer NOT NULL
);

--
-- TOC entry 2883 (class 2604 OID 63183)
-- Name: region_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY ring_transport_regions ALTER COLUMN region_id SET DEFAULT nextval('ring_transport_regions_region_id_seq'::regclass);

