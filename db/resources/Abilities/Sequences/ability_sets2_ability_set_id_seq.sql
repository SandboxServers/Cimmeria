--
-- TOC entry 185 (class 1259 OID 62797)
-- Name: ability_sets2_ability_set_id_seq; Type: SEQUENCE; Schema: resources; Owner: -
--

CREATE SEQUENCE ability_sets2_ability_set_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 3271 (class 0 OID 0)
-- Dependencies: 185
-- Name: ability_sets2_ability_set_id_seq; Type: SEQUENCE OWNED BY; Schema: resources; Owner: -
--

ALTER SEQUENCE ability_sets2_ability_set_id_seq OWNED BY ability_sets.ability_set_id;

