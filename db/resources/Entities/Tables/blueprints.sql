--
-- TOC entry 189 (class 1259 OID 62814)
-- Name: blueprints; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE blueprints (
    blueprint_id integer NOT NULL,
    discipline_id integer,
    is_alloy boolean,
    product_id integer,
    quantity integer NOT NULL,
    requires_elementary_components boolean
);

