--
-- TOC entry 190 (class 1259 OID 62817)
-- Name: blueprints_components; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE blueprints_components (
    blueprint_id integer NOT NULL,
    item_id integer NOT NULL,
    quantity integer NOT NULL,
    component_set_id integer NOT NULL
);

