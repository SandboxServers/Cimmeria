--
-- TOC entry 231 (class 1259 OID 63013)
-- Name: mission_steps; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE mission_steps (
    step_id integer NOT NULL,
    mission_id integer NOT NULL,
    award_xp boolean NOT NULL,
    difficulty integer NOT NULL,
    step_enabled boolean NOT NULL,
    step_display_log_text character varying(255) NOT NULL,
    index integer
);

