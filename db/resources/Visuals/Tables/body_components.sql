--
-- TOC entry 192 (class 1259 OID 62826)
-- Name: body_components; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE body_components (
    component_name character varying(1000) NOT NULL,
    body_sets character varying(500)[] NOT NULL,
    slots character varying(100)[] NOT NULL,
    skeletal_meshes character varying(500)[] NOT NULL
);

