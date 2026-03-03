--
-- TOC entry 221 (class 1259 OID 62963)
-- Name: item_lists; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE item_lists (
    item_list_id integer NOT NULL,
    name character varying(100)
);

--
-- TOC entry 2852 (class 2604 OID 63176)
-- Name: item_list_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY item_lists ALTER COLUMN item_list_id SET DEFAULT nextval('item_lists_item_list_id_seq'::regclass);

