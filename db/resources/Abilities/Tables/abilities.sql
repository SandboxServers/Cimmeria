--
-- TOC entry 180 (class 1259 OID 62768)
-- Name: abilities; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE abilities (
    ability_id integer NOT NULL,
    name character varying(255) DEFAULT NULL::character varying NOT NULL,
    description character varying(255) DEFAULT NULL::character varying NOT NULL,
    type_id "EAbilityTypes" NOT NULL,
    cooldown real NOT NULL,
    flags integer NOT NULL,
    icon character varying(255) DEFAULT NULL::character varying,
    is_ranged boolean NOT NULL,
    min_range integer NOT NULL,
    max_range integer NOT NULL,
    passive_yn boolean NOT NULL,
    param1 character varying(255) DEFAULT NULL::character varying,
    param2 character varying(255) DEFAULT NULL::character varying,
    target_type_id integer NOT NULL,
    target_collection_method "ETargetCollectionMethod" NOT NULL,
    taunt_adjustment integer NOT NULL,
    threat_level_id "EThreatLevel" NOT NULL,
    training_cost integer NOT NULL,
    velocity real NOT NULL,
    warmup real NOT NULL,
    effect_ids integer[] DEFAULT '{}'::integer[] NOT NULL,
    moniker_ids bigint[] DEFAULT '{}'::bigint[] NOT NULL,
    event_set_id integer,
    required_ammo integer DEFAULT 0 NOT NULL,
    positions "ECASPosition"[],
    item_monikers bigint[] DEFAULT '{}'::bigint[] NOT NULL
);

