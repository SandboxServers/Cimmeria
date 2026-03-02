--
-- TOC entry 188 (class 1259 OID 62811)
-- Name: archetypes; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE archetypes (
    archetype "EArchetype" NOT NULL,
    coordination integer NOT NULL,
    engagement integer NOT NULL,
    fortitude integer NOT NULL,
    morale integer NOT NULL,
    perception integer NOT NULL,
    intelligence integer NOT NULL,
    health integer NOT NULL,
    focus integer NOT NULL,
    "healthPerLevel" integer NOT NULL,
    "focusPerLevel" integer NOT NULL,
    name character varying(200)
);

