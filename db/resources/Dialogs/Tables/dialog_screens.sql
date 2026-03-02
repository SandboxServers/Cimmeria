--
-- TOC entry 199 (class 1259 OID 62858)
-- Name: dialog_screens; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE dialog_screens (
    dialog_id integer NOT NULL,
    screen_id integer NOT NULL,
    text text NOT NULL,
    speaker_id integer,
    index integer NOT NULL
);

--
-- TOC entry 2820 (class 2604 OID 70180)
-- Name: screen_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY dialog_screens ALTER COLUMN screen_id SET DEFAULT nextval('dialog_screens_screen_id_seq'::regclass);

