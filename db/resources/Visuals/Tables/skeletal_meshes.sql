--
-- TOC entry 250 (class 1259 OID 63091)
-- Name: skeletal_meshes; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE skeletal_meshes (
    mesh_name character varying(1000) NOT NULL,
    socket_names character varying(200)[] NOT NULL,
    bone_names character varying(200)[] NOT NULL
);

