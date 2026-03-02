--
-- TOC entry 226 (class 1259 OID 62992)
-- Name: loot_tables; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE loot_tables (
    loot_table_id integer NOT NULL,
    description character varying NOT NULL
);

--
-- TOC entry 2865 (class 2604 OID 63178)
-- Name: loot_table_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY loot_tables ALTER COLUMN loot_table_id SET DEFAULT nextval('loot_tables_loot_table_id_seq'::regclass);

