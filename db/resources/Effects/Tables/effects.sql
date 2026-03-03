--
-- TOC entry 206 (class 1259 OID 62890)
-- Name: effects; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE effects (
    effect_id integer NOT NULL,
    ability_id integer NOT NULL,
    delay integer NOT NULL,
    effect_desc character varying(255) NOT NULL,
    effect_sequence integer NOT NULL,
    flags integer NOT NULL,
    icon_location character varying(255),
    pulse_count integer NOT NULL,
    pulse_duration real NOT NULL,
    tcm_param1 character varying(255),
    tcm_param2 character varying(255),
    target_collection_method "ETargetCollectionMethod" NOT NULL,
    use_ability_velocity boolean NOT NULL,
    is_channeled boolean NOT NULL,
    name character varying(255) NOT NULL,
    target_collection_id integer NOT NULL,
    event_set_id integer,
    script_name character varying(100)
);

