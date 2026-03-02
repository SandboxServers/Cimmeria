--
-- TOC entry 305 (class 1259 OID 64197)
-- Name: effect_nvps_2_nvp_id_seq; Type: SEQUENCE; Schema: resources; Owner: -
--

CREATE SEQUENCE effect_nvps_2_nvp_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 3278 (class 0 OID 0)
-- Dependencies: 305
-- Name: effect_nvps_2_nvp_id_seq; Type: SEQUENCE OWNED BY; Schema: resources; Owner: -
--

ALTER SEQUENCE effect_nvps_2_nvp_id_seq OWNED BY effect_nvps.nvp_id;

