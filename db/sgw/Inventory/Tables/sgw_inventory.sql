--
-- TOC entry 275 (class 1259 OID 63815)
-- Name: sgw_inventory; Type: TABLE; Schema: public; Owner: -; Tablespace: 
--

CREATE TABLE sgw_inventory (
    item_id integer,
    stack_size integer DEFAULT 1,
    charges integer DEFAULT 0,
    container_id integer,
    slot_id integer,
    durability integer DEFAULT (-1),
    type_id integer,
    flags integer DEFAULT 0,
    character_id integer NOT NULL,
    bound boolean DEFAULT false NOT NULL,
    ammo integer DEFAULT 0 NOT NULL,
    CONSTRAINT local_id_check CHECK ((item_id >= 10000))
)
INHERITS (sgw_inventory_base);

--
-- TOC entry 2655 (class 2604 OID 63850)
-- Name: item_id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_inventory ALTER COLUMN item_id SET DEFAULT nextval('sgw_inventory_item_id_seq'::regclass);

--
-- TOC entry 2656 (class 2604 OID 63851)
-- Name: ammo_type; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_inventory ALTER COLUMN ammo_type SET DEFAULT 'AMMO_NONE'::resources."EAmmoType";

--
-- TOC entry 2657 (class 2604 OID 63852)
-- Name: ammo_types; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_inventory ALTER COLUMN ammo_types SET DEFAULT '{}'::resources."EAmmoType"[];

