--
-- TOC entry 201 (class 1259 OID 62867)
-- Name: dialog_set_maps_dialog_set_map_id_seq; Type: SEQUENCE; Schema: resources; Owner: -
--

CREATE SEQUENCE dialog_set_maps_dialog_set_map_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 3275 (class 0 OID 0)
-- Dependencies: 201
-- Name: dialog_set_maps_dialog_set_map_id_seq; Type: SEQUENCE OWNED BY; Schema: resources; Owner: -
--

ALTER SEQUENCE dialog_set_maps_dialog_set_map_id_seq OWNED BY dialog_set_maps.dialog_set_map_id;

