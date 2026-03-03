--
-- TOC entry 242 (class 1259 OID 63055)
-- Name: resource_types; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE resource_types (
    type "EResourceType" NOT NULL,
    "table" character varying(100) NOT NULL,
    key_column character varying(100) NOT NULL
);

