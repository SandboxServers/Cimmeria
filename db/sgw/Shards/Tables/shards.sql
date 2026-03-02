--
-- TOC entry 279 (class 1259 OID 63844)
-- Name: shards; Type: TABLE; Schema: public; Owner: -; Tablespace: 
--

CREATE TABLE shards (
    shard_id integer NOT NULL,
    name character varying(100),
    key character varying(100) NOT NULL,
    protected boolean DEFAULT false NOT NULL
);

