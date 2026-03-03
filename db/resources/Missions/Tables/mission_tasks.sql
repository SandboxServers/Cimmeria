--
-- TOC entry 232 (class 1259 OID 63016)
-- Name: mission_tasks; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE mission_tasks (
    task_id integer NOT NULL,
    objective_id integer NOT NULL,
    award_xp boolean NOT NULL,
    difficulty integer NOT NULL,
    is_enabled boolean NOT NULL,
    task_type integer NOT NULL
);

