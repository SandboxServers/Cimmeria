--
-- TOC entry 278 (class 1259 OID 63834)
-- Name: sgw_mission; Type: TABLE; Schema: public; Owner: -; Tablespace: 
--

CREATE TABLE sgw_mission (
    player_id integer NOT NULL,
    mission_id integer NOT NULL,
    current_step_id integer,
    status integer NOT NULL,
    completed_step_ids integer[] NOT NULL,
    completed_objective_ids integer[] NOT NULL,
    active_objective_ids integer[] NOT NULL,
    failed_objective_ids integer[] NOT NULL,
    repeats integer DEFAULT 0 NOT NULL
);

