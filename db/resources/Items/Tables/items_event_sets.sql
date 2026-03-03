--
-- TOC entry 308 (class 1259 OID 64291)
-- Name: items_event_sets; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE items_event_sets (
    item_event_id integer NOT NULL,
    item_id integer NOT NULL,
    ability_id integer NOT NULL,
    event_id integer NOT NULL
);

--
-- TOC entry 2912 (class 2604 OID 64294)
-- Name: item_event_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY items_event_sets ALTER COLUMN item_event_id SET DEFAULT nextval('items_event_sets_2_item_event_id_seq'::regclass);

