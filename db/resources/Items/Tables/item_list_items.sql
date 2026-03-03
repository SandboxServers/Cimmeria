--
-- TOC entry 218 (class 1259 OID 62955)
-- Name: item_list_items; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE item_list_items (
    item_id integer NOT NULL,
    item_list_id integer NOT NULL,
    design_id integer NOT NULL,
    quantity integer NOT NULL,
    naquadah integer NOT NULL
);

--
-- TOC entry 2851 (class 2604 OID 63175)
-- Name: item_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY item_list_items ALTER COLUMN item_id SET DEFAULT nextval('item_list_items_item_id_seq'::regclass);

