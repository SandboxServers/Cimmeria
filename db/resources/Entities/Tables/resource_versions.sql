--
-- TOC entry 243 (class 1259 OID 63058)
-- Name: resource_versions; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE resource_versions (
    type "EResourceType" NOT NULL,
    version integer NOT NULL,
    invalidated_keys integer[] NOT NULL,
    new_keys integer[] NOT NULL,
    pending boolean DEFAULT false NOT NULL,
    invalidate_all boolean DEFAULT false NOT NULL,
    snapshot character varying(100) DEFAULT (txid_current_snapshot())::character varying NOT NULL
);

