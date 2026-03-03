--
-- TOC entry 211 (class 1259 OID 62924)
-- Name: error_texts; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE error_texts (
    error_id integer NOT NULL,
    flags integer NOT NULL,
    language integer NOT NULL,
    moniker_id integer NOT NULL,
    moniker_name character varying(255) NOT NULL,
    text character varying(255) NOT NULL
);

