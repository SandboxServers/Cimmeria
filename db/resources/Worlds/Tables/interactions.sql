--
-- TOC entry 217 (class 1259 OID 62944)
-- Name: interactions; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE interactions (
    interaction_id integer NOT NULL,
    body_set_asset character varying(255) DEFAULT NULL::character varying,
    body_set_priority character varying(255) DEFAULT NULL::character varying NOT NULL,
    cursor_asset character varying(255) DEFAULT NULL::character varying,
    cursor_priority character varying(255) DEFAULT NULL::character varying NOT NULL,
    is_static boolean NOT NULL,
    minimap_asset character varying(255) DEFAULT NULL::character varying,
    minimap_priority integer NOT NULL
);

