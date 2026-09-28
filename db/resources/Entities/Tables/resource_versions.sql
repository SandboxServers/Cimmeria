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
    -- Unbounded: pg_current_snapshot() lists every running transaction id on the
    -- server, so it outgrew varchar(100) with a dozen concurrent writers.
    snapshot text DEFAULT (pg_current_snapshot())::text NOT NULL
);

