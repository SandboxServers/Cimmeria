--
-- TOC entry 222 (class 1259 OID 62966)
-- Name: item_lists_item_list_id_seq; Type: SEQUENCE; Schema: resources; Owner: -
--

CREATE SEQUENCE item_lists_item_list_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 3284 (class 0 OID 0)
-- Dependencies: 222
-- Name: item_lists_item_list_id_seq; Type: SEQUENCE OWNED BY; Schema: resources; Owner: -
--

ALTER SEQUENCE item_lists_item_list_id_seq OWNED BY item_lists.item_list_id;

