--
-- TOC entry 258 (class 1259 OID 63115)
-- Name: special_words; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE special_words (
    flags integer NOT NULL,
    profanity integer NOT NULL,
    special_word character varying(255) NOT NULL,
    "substring" integer NOT NULL
);

