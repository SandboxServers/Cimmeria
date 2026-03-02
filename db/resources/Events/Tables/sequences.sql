--
-- TOC entry 248 (class 1259 OID 63081)
-- Name: sequences; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE sequences (
    sequence_id integer DEFAULT nextval('sequences_kismet_event_set_seq_id_seq'::regclass) NOT NULL,
    event_id integer NOT NULL,
    kismet_script_name character varying(255) NOT NULL
);

