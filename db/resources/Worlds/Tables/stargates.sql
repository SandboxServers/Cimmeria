--
-- TOC entry 260 (class 1259 OID 63120)
-- Name: stargates; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE stargates (
    address1 integer NOT NULL,
    address2 integer NOT NULL,
    address3 integer NOT NULL,
    address4 integer NOT NULL,
    address5 integer NOT NULL,
    address6 integer NOT NULL,
    address_origin integer NOT NULL,
    stargate_id integer DEFAULT nextval('stargates_id_seq'::regclass) NOT NULL,
    name character varying(255) NOT NULL,
    pitch double precision NOT NULL,
    prefab_sequence character varying(255) NOT NULL,
    roll double precision NOT NULL,
    world_id integer NOT NULL,
    x_pos double precision NOT NULL,
    y_pos double precision NOT NULL,
    yaw double precision NOT NULL,
    z_pos double precision NOT NULL,
    event_set_id integer
);

