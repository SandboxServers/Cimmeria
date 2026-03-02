--
-- TOC entry 265 (class 1259 OID 63145)
-- Name: trainer_ability_lists_list_id_seq; Type: SEQUENCE; Schema: resources; Owner: -
--

CREATE SEQUENCE trainer_ability_lists_list_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 3303 (class 0 OID 0)
-- Dependencies: 265
-- Name: trainer_ability_lists_list_id_seq; Type: SEQUENCE OWNED BY; Schema: resources; Owner: -
--

ALTER SEQUENCE trainer_ability_lists_list_id_seq OWNED BY trainer_ability_lists.list_id;

