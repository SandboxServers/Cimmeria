--
-- TOC entry 262 (class 1259 OID 63133)
-- Name: texts; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE texts (
    moniker_id integer NOT NULL,
    flags integer NOT NULL,
    language integer NOT NULL,
    moniker_name character varying(255) NOT NULL,
    text character varying(255) NOT NULL
);

