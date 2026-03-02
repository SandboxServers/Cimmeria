--
-- TOC entry 208 (class 1259 OID 62907)
-- Name: entity_interactions_entity_interaction_id_seq; Type: SEQUENCE; Schema: resources; Owner: -
--

CREATE SEQUENCE entity_interactions_entity_interaction_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 3280 (class 0 OID 0)
-- Dependencies: 208
-- Name: entity_interactions_entity_interaction_id_seq; Type: SEQUENCE OWNED BY; Schema: resources; Owner: -
--

ALTER SEQUENCE entity_interactions_entity_interaction_id_seq OWNED BY entity_interactions.entity_interaction_id;

