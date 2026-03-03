--
-- TOC entry 304 (class 1259 OID 64178)
-- Name: dialog_screen_buttons; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE dialog_screen_buttons (
    screen_button_id integer NOT NULL,
    button_id integer NOT NULL,
    screen_id integer NOT NULL,
    button_type integer NOT NULL,
    text character varying(255) NOT NULL
);

--
-- TOC entry 2909 (class 2604 OID 64181)
-- Name: screen_button_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY dialog_screen_buttons ALTER COLUMN screen_button_id SET DEFAULT nextval('dialog_screen_buttons_2_screen_button_id_seq'::regclass);

--
-- TOC entry 2910 (class 2604 OID 70183)
-- Name: button_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY dialog_screen_buttons ALTER COLUMN button_id SET DEFAULT nextval('dialog_screen_buttons_button_id_seq'::regclass);

