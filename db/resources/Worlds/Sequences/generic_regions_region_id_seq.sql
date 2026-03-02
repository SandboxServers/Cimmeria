--
-- TOC entry 216 (class 1259 OID 62942)
-- Name: generic_regions_region_id_seq; Type: SEQUENCE; Schema: resources; Owner: -
--

CREATE SEQUENCE generic_regions_region_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 3282 (class 0 OID 0)
-- Dependencies: 216
-- Name: generic_regions_region_id_seq; Type: SEQUENCE OWNED BY; Schema: resources; Owner: -
--

ALTER SEQUENCE generic_regions_region_id_seq OWNED BY generic_regions.region_id;

