--
-- TOC entry 223 (class 1259 OID 62968)
-- Name: items; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE items (
    item_id integer DEFAULT nextval('items_item_id_seq'::regclass) NOT NULL,
    applied_science_id integer,
    description character varying(255) NOT NULL,
    icon_location character varying(255),
    name character varying(255) NOT NULL,
    quality_id "EItemQuality" NOT NULL,
    tech_comp integer NOT NULL,
    tier integer NOT NULL,
    container_sets integer[] DEFAULT '{}'::integer[] NOT NULL,
    max_stack_size integer NOT NULL,
    moniker_ids bigint[] DEFAULT '{}'::bigint[] NOT NULL,
    max_ranged_range integer,
    min_ranged_range integer,
    visual_component character varying(255) DEFAULT NULL::character varying,
    max_melee_range integer,
    min_melee_range integer,
    discipline_ids integer[] DEFAULT '{}'::integer[] NOT NULL,
    flags integer DEFAULT 0 NOT NULL,
    ammo_types "EAmmoType"[] DEFAULT '{}'::"EAmmoType"[] NOT NULL,
    default_ammo_type "EAmmoType",
    clip_size integer DEFAULT 0 NOT NULL,
    charges integer DEFAULT 0 NOT NULL
);

