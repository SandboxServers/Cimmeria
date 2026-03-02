--
-- TOC entry 209 (class 1259 OID 62909)
-- Name: entity_templates; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE entity_templates (
    template_id integer NOT NULL,
    static_mesh character varying(200),
    body_set character varying(200) NOT NULL,
    components character varying(200)[],
    flags bigint DEFAULT 0 NOT NULL,
    interaction_type bigint DEFAULT 0 NOT NULL,
    event_set_id integer,
    level integer,
    alignment integer,
    faction integer,
    name_id integer,
    name character varying(200),
    patrol_path_id integer,
    patrol_point_delay real,
    template_name character varying(200) NOT NULL,
    class character varying(50) NOT NULL,
    buy_item_list integer,
    sell_item_list integer,
    repair_item_list integer,
    recharge_item_list integer,
    ability_set_id integer,
    ammo_type "EAmmoType",
    loot_table_id integer,
    primary_color_id bigint DEFAULT 0 NOT NULL,
    secondary_color_id bigint DEFAULT 0 NOT NULL,
    skin_tint bigint DEFAULT 0 NOT NULL,
    weapon_item_id integer,
    static_interaction_sets integer[] DEFAULT ARRAY[]::integer[] NOT NULL,
    trainer_ability_list_id integer,
    speaker_id integer,
    has_dynamic_properties boolean DEFAULT true NOT NULL,
    interaction_set_id integer
);

--
-- TOC entry 2843 (class 2604 OID 63173)
-- Name: template_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY entity_templates ALTER COLUMN template_id SET DEFAULT nextval('entity_templates_template_id_seq'::regclass);

