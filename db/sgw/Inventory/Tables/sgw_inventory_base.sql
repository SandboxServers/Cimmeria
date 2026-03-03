--
-- TOC entry 274 (class 1259 OID 63803)
-- Name: sgw_inventory_base; Type: TABLE; Schema: public; Owner: -; Tablespace: 
--

CREATE TABLE sgw_inventory_base (
    item_id integer NOT NULL,
    stack_size integer DEFAULT 1 NOT NULL,
    charges integer DEFAULT 0 NOT NULL,
    container_id integer NOT NULL,
    slot_id integer NOT NULL,
    durability integer DEFAULT (-1) NOT NULL,
    type_id integer NOT NULL,
    flags integer DEFAULT 0 NOT NULL,
    ammo_type resources."EAmmoType" DEFAULT 'AMMO_NONE'::resources."EAmmoType" NOT NULL,
    ammo_types resources."EAmmoType"[] DEFAULT '{}'::resources."EAmmoType"[] NOT NULL
);

--
-- TOC entry 2648 (class 2604 OID 63853)
-- Name: item_id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_inventory_base ALTER COLUMN item_id SET DEFAULT nextval('sgw_inventory_base_item_id_seq'::regclass);

