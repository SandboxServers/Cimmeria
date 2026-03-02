--
-- TOC entry 252 (class 1259 OID 63100)
-- Name: spawn_points_point_id_seq; Type: SEQUENCE; Schema: resources; Owner: -
--

CREATE SEQUENCE spawn_points_point_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 3300 (class 0 OID 0)
-- Dependencies: 252
-- Name: spawn_points_point_id_seq; Type: SEQUENCE OWNED BY; Schema: resources; Owner: -
--

ALTER SEQUENCE spawn_points_point_id_seq OWNED BY spawn_points.point_id;

