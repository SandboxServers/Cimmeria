--
-- TOC entry 254 (class 1259 OID 63105)
-- Name: spawn_sets_set_id_seq; Type: SEQUENCE; Schema: resources; Owner: -
--

CREATE SEQUENCE spawn_sets_set_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 3301 (class 0 OID 0)
-- Dependencies: 254
-- Name: spawn_sets_set_id_seq; Type: SEQUENCE OWNED BY; Schema: resources; Owner: -
--

ALTER SEQUENCE spawn_sets_set_id_seq OWNED BY spawn_sets.set_id;

