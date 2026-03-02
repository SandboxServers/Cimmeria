--
-- TOC entry 204 (class 1259 OID 62874)
-- Name: dialogs; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE dialogs (
    dialog_id integer NOT NULL,
    dialog_flags integer NOT NULL,
    event_set_id integer,
    ui_screen_type "EDialogUIScreenType" NOT NULL,
    tags character varying[],
    accepts_mission_id integer,
    name character varying(300)
);

--
-- TOC entry 2828 (class 2604 OID 70176)
-- Name: dialog_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY dialogs ALTER COLUMN dialog_id SET DEFAULT nextval('dialogs_dialog_id_seq'::regclass);

