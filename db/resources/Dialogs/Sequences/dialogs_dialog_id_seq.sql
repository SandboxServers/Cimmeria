--
-- TOC entry 309 (class 1259 OID 70174)
-- Name: dialogs_dialog_id_seq; Type: SEQUENCE; Schema: resources; Owner: -
--

CREATE SEQUENCE dialogs_dialog_id_seq
    START WITH 6428
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 3277 (class 0 OID 0)
-- Dependencies: 309
-- Name: dialogs_dialog_id_seq; Type: SEQUENCE OWNED BY; Schema: resources; Owner: -
--

ALTER SEQUENCE dialogs_dialog_id_seq OWNED BY dialogs.dialog_id;

