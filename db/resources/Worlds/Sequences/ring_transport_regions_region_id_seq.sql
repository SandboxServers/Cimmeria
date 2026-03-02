--
-- TOC entry 246 (class 1259 OID 63077)
-- Name: ring_transport_regions_region_id_seq; Type: SEQUENCE; Schema: resources; Owner: -
--

CREATE SEQUENCE ring_transport_regions_region_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 3299 (class 0 OID 0)
-- Dependencies: 246
-- Name: ring_transport_regions_region_id_seq; Type: SEQUENCE OWNED BY; Schema: resources; Owner: -
--

ALTER SEQUENCE ring_transport_regions_region_id_seq OWNED BY ring_transport_regions.region_id;

