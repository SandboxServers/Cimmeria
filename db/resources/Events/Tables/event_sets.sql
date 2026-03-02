--
-- TOC entry 213 (class 1259 OID 62932)
-- Name: event_sets; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE event_sets (
    event_set_id integer DEFAULT nextval('event_sets_event_set_id_seq'::regclass) NOT NULL,
    name character varying(200)
);

