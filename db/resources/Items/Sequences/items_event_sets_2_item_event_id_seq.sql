--
-- TOC entry 307 (class 1259 OID 64289)
-- Name: items_event_sets_2_item_event_id_seq; Type: SEQUENCE; Schema: resources; Owner: -
--

CREATE SEQUENCE items_event_sets_2_item_event_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 3285 (class 0 OID 0)
-- Dependencies: 307
-- Name: items_event_sets_2_item_event_id_seq; Type: SEQUENCE OWNED BY; Schema: resources; Owner: -
--

ALTER SEQUENCE items_event_sets_2_item_event_id_seq OWNED BY items_event_sets.item_event_id;

