--
-- TOC entry 244 (class 1259 OID 63067)
-- Name: respawners; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE respawners (
    respawner_id integer NOT NULL,
    world_id integer NOT NULL,
    name character varying(100) NOT NULL,
    pos_x real NOT NULL,
    pos_y real NOT NULL,
    pos_z real NOT NULL
);

