--
-- TOC entry 234 (class 1259 OID 63027)
-- Name: monikers; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE monikers (
    moniker_id bigint NOT NULL,
    name character varying(100) NOT NULL,
    description character varying(500)
);

