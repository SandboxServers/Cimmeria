--
-- TOC entry 276 (class 1259 OID 63830)
-- Name: sgw_inventory_base_item_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE sgw_inventory_base_item_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 2707 (class 0 OID 0)
-- Dependencies: 276
-- Name: sgw_inventory_base_item_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE sgw_inventory_base_item_id_seq OWNED BY sgw_inventory_base.item_id;

