--
-- TOC entry 236 (class 1259 OID 63036)
-- Name: paths_path_id_seq; Type: SEQUENCE; Schema: resources; Owner: -
--

CREATE SEQUENCE paths_path_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 3296 (class 0 OID 0)
-- Dependencies: 236
-- Name: paths_path_id_seq; Type: SEQUENCE OWNED BY; Schema: resources; Owner: -
--

ALTER SEQUENCE paths_path_id_seq OWNED BY paths.path_id;

