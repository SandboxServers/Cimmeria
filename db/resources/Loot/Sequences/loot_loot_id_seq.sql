--
-- TOC entry 225 (class 1259 OID 62990)
-- Name: loot_loot_id_seq; Type: SEQUENCE; Schema: resources; Owner: -
--

CREATE SEQUENCE loot_loot_id_seq
    START WITH 10
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 3287 (class 0 OID 0)
-- Dependencies: 225
-- Name: loot_loot_id_seq; Type: SEQUENCE OWNED BY; Schema: resources; Owner: -
--

ALTER SEQUENCE loot_loot_id_seq OWNED BY loot.loot_id;

