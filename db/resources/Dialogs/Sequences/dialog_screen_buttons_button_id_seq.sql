--
-- TOC entry 311 (class 1259 OID 70181)
-- Name: dialog_screen_buttons_button_id_seq; Type: SEQUENCE; Schema: resources; Owner: -
--

CREATE SEQUENCE dialog_screen_buttons_button_id_seq
    START WITH 6428
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 3273 (class 0 OID 0)
-- Dependencies: 311
-- Name: dialog_screen_buttons_button_id_seq; Type: SEQUENCE OWNED BY; Schema: resources; Owner: -
--

ALTER SEQUENCE dialog_screen_buttons_button_id_seq OWNED BY dialog_screen_buttons.screen_button_id;

