--
-- TOC entry 205 (class 1259 OID 62880)
-- Name: disciplines; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE disciplines (
    discipline_id integer NOT NULL,
    applied_science_id integer NOT NULL,
    "column" integer NOT NULL,
    icon character varying(255) NOT NULL,
    name character varying(255) NOT NULL,
    racial_paradigm_id integer NOT NULL,
    racial_paradigm_level integer NOT NULL,
    "row" integer NOT NULL,
    tech_competency integer NOT NULL,
    required_discipline_ids integer[] DEFAULT '{}'::integer[] NOT NULL
);

