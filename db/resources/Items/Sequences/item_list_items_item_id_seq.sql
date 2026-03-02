--
-- TOC entry 219 (class 1259 OID 62958)
-- Name: item_list_items_item_id_seq; Type: SEQUENCE; Schema: resources; Owner: -
--

CREATE SEQUENCE item_list_items_item_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 3283 (class 0 OID 0)
-- Dependencies: 219
-- Name: item_list_items_item_id_seq; Type: SEQUENCE OWNED BY; Schema: resources; Owner: -
--

ALTER SEQUENCE item_list_items_item_id_seq OWNED BY item_list_items.item_id;

