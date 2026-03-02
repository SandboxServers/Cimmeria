--
-- TOC entry 277 (class 1259 OID 63832)
-- Name: sgw_inventory_item_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE sgw_inventory_item_id_seq
    START WITH 10221
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 2708 (class 0 OID 0)
-- Dependencies: 277
-- Name: sgw_inventory_item_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE sgw_inventory_item_id_seq OWNED BY sgw_inventory.item_id;

