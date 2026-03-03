--
-- TOC entry 266 (class 1259 OID 63147)
-- Name: worlds; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE worlds (
    world_id integer NOT NULL,
    flags integer NOT NULL,
    min_per_day integer NOT NULL,
    min_to_real_min integer NOT NULL,
    world character varying(255) NOT NULL,
    cell_id integer,
    gravity real DEFAULT 7.5 NOT NULL,
    run_speed real DEFAULT 8.125 NOT NULL,
    sideways_run_speed real DEFAULT 8.125 NOT NULL,
    backwards_run_speed real DEFAULT 6.09375 NOT NULL,
    walk_speed real DEFAULT 2.069 NOT NULL,
    sideways_walk_speed real DEFAULT 2.069 NOT NULL,
    backwards_walk_speed real DEFAULT 3.796875 NOT NULL,
    crouch_run_speed real DEFAULT 5.0625 NOT NULL,
    sideways_crouch_run_speed real DEFAULT 5.0625 NOT NULL,
    backwards_crouch_run_speed real DEFAULT 3.796875 NOT NULL,
    crouch_walk_speed real DEFAULT 0.935 NOT NULL,
    sideways_crouch_walk_speed real DEFAULT 0.70125 NOT NULL,
    backwards_crouch_walk_speed real DEFAULT 0.935 NOT NULL,
    swim_speed real DEFAULT 4 NOT NULL,
    sideways_swim_speed real DEFAULT 4 NOT NULL,
    backwards_swim_speed real DEFAULT 1 NOT NULL,
    jump_speed real DEFAULT 6 NOT NULL,
    has_script boolean DEFAULT false NOT NULL,
    client_map character varying(100) NOT NULL
);

