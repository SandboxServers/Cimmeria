--
-- TOC entry 256 (class 1259 OID 63110)
-- Name: spawnlist_spawn_id_seq; Type: SEQUENCE; Schema: resources; Owner: -
--

CREATE SEQUENCE spawnlist_spawn_id_seq
    START WITH 36
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 3302 (class 0 OID 0)
-- Dependencies: 256
-- Name: spawnlist_spawn_id_seq; Type: SEQUENCE OWNED BY; Schema: resources; Owner: -
--

ALTER SEQUENCE spawnlist_spawn_id_seq OWNED BY spawnlist.spawn_id;

