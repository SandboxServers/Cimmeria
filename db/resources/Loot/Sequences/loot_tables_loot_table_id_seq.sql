--
-- TOC entry 227 (class 1259 OID 62998)
-- Name: loot_tables_loot_table_id_seq; Type: SEQUENCE; Schema: resources; Owner: -
--

CREATE SEQUENCE loot_tables_loot_table_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 3288 (class 0 OID 0)
-- Dependencies: 227
-- Name: loot_tables_loot_table_id_seq; Type: SEQUENCE OWNED BY; Schema: resources; Owner: -
--

ALTER SEQUENCE loot_tables_loot_table_id_seq OWNED BY loot_tables.loot_table_id;

