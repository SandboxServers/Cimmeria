--
-- TOC entry 203 (class 1259 OID 62872)
-- Name: dialog_sets_dialog_set_id_seq; Type: SEQUENCE; Schema: resources; Owner: -
--

CREATE SEQUENCE dialog_sets_dialog_set_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 3276 (class 0 OID 0)
-- Dependencies: 203
-- Name: dialog_sets_dialog_set_id_seq; Type: SEQUENCE OWNED BY; Schema: resources; Owner: -
--

ALTER SEQUENCE dialog_sets_dialog_set_id_seq OWNED BY dialog_sets.dialog_set_id;

