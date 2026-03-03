--
-- TOC entry 202 (class 1259 OID 62869)
-- Name: dialog_sets; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE dialog_sets (
    dialog_set_id integer NOT NULL,
    name character varying(200)
);

--
-- TOC entry 2827 (class 2604 OID 63171)
-- Name: dialog_set_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY dialog_sets ALTER COLUMN dialog_set_id SET DEFAULT nextval('dialog_sets_dialog_set_id_seq'::regclass);

