--
-- TOC entry 238 (class 1259 OID 63044)
-- Name: point_set_points_point_id_seq; Type: SEQUENCE; Schema: resources; Owner: -
--

CREATE SEQUENCE point_set_points_point_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 3297 (class 0 OID 0)
-- Dependencies: 238
-- Name: point_set_points_point_id_seq; Type: SEQUENCE OWNED BY; Schema: resources; Owner: -
--

ALTER SEQUENCE point_set_points_point_id_seq OWNED BY point_set_points.point_id;

