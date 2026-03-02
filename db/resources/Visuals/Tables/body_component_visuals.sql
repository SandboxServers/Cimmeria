--
-- TOC entry 191 (class 1259 OID 62820)
-- Name: body_component_visuals; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE body_component_visuals (
    component_name character varying(1000) NOT NULL,
    index integer NOT NULL,
    body_sets character varying(500)[] NOT NULL,
    slots character varying(100)[] NOT NULL,
    skeletal_meshes character varying(500)[] NOT NULL
);

