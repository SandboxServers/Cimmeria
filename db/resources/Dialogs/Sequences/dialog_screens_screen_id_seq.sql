--
-- TOC entry 310 (class 1259 OID 70178)
-- Name: dialog_screens_screen_id_seq; Type: SEQUENCE; Schema: resources; Owner: -
--

CREATE SEQUENCE dialog_screens_screen_id_seq
    START WITH 6428
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 3274 (class 0 OID 0)
-- Dependencies: 310
-- Name: dialog_screens_screen_id_seq; Type: SEQUENCE OWNED BY; Schema: resources; Owner: -
--

ALTER SEQUENCE dialog_screens_screen_id_seq OWNED BY dialog_screens.dialog_id;

