--
-- TOC entry 224 (class 1259 OID 62985)
-- Name: loot; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE loot (
    loot_id integer NOT NULL,
    loot_table_id integer NOT NULL,
    design_id integer,
    min_quantity integer NOT NULL,
    probability real NOT NULL,
    max_quantity integer NOT NULL,
    CONSTRAINT loot_check CHECK (((min_quantity > 0) AND (max_quantity >= min_quantity))),
    CONSTRAINT loot_probability_check CHECK (((probability > (0)::double precision) AND (probability <= (1)::double precision)))
);

--
-- TOC entry 2862 (class 2604 OID 63177)
-- Name: loot_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY loot ALTER COLUMN loot_id SET DEFAULT nextval('loot_loot_id_seq'::regclass);

