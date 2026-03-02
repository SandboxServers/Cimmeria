--
-- TOC entry 273 (class 1259 OID 63801)
-- Name: sgw_gate_mail_mail_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE sgw_gate_mail_mail_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 2706 (class 0 OID 0)
-- Dependencies: 273
-- Name: sgw_gate_mail_mail_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE sgw_gate_mail_mail_id_seq OWNED BY sgw_gate_mail.mail_id;

