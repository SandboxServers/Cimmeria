--
-- TOC entry 210 (class 1259 OID 62922)
-- Name: entity_templates_template_id_seq; Type: SEQUENCE; Schema: resources; Owner: -
--

CREATE SEQUENCE entity_templates_template_id_seq
    START WITH 24
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 3281 (class 0 OID 0)
-- Dependencies: 210
-- Name: entity_templates_template_id_seq; Type: SEQUENCE OWNED BY; Schema: resources; Owner: -
--

ALTER SEQUENCE entity_templates_template_id_seq OWNED BY entity_templates.template_id;

