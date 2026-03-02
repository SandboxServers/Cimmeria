--
-- TOC entry 228 (class 1259 OID 63000)
-- Name: mission_objectives; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE mission_objectives (
    objective_id integer NOT NULL,
    step_id integer NOT NULL,
    award_xp boolean NOT NULL,
    difficulty integer NOT NULL,
    is_enabled boolean NOT NULL,
    is_hidden boolean NOT NULL,
    is_optional boolean NOT NULL,
    display_log_text character varying(255) DEFAULT NULL::character varying
);

