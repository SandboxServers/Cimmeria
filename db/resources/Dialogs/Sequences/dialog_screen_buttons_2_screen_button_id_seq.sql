--
-- TOC entry 303 (class 1259 OID 64176)
-- Name: dialog_screen_buttons_2_screen_button_id_seq; Type: SEQUENCE; Schema: resources; Owner: -
--

CREATE SEQUENCE dialog_screen_buttons_2_screen_button_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 3272 (class 0 OID 0)
-- Dependencies: 303
-- Name: dialog_screen_buttons_2_screen_button_id_seq; Type: SEQUENCE OWNED BY; Schema: resources; Owner: -
--

ALTER SEQUENCE dialog_screen_buttons_2_screen_button_id_seq OWNED BY dialog_screen_buttons.screen_button_id;

