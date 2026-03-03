--
-- TOC entry 194 (class 1259 OID 62838)
-- Name: char_creation; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE char_creation (
    char_def_id integer NOT NULL,
    alignment "EAlignment" NOT NULL,
    archetype "EArchetype" NOT NULL,
    body_set character varying(255) NOT NULL,
    gender "EGender" NOT NULL,
    starting_world character varying(100) NOT NULL,
    starting_x real NOT NULL,
    starting_y real NOT NULL,
    starting_z real NOT NULL
);

