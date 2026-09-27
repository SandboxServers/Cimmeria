--
-- TOC entry 272 (class 1259 OID 63791)
-- Name: sgw_gate_mail; Type: TABLE; Schema: public; Owner: -; Tablespace: 
--

CREATE TABLE sgw_gate_mail (
    mail_id integer NOT NULL,
    character_id integer NOT NULL,
    sender_id integer,
    subject character varying(128) DEFAULT NULL::character varying NOT NULL,
    message text NOT NULL,
    cash bigint DEFAULT 0 NOT NULL,
    sent_time integer NOT NULL,
    read_time integer NOT NULL,
    flags integer DEFAULT 0 NOT NULL,
    item_id integer,
    sender_name character varying(128) DEFAULT ''::character varying NOT NULL,
    -- Social-systems SS-M3 (D-SS10): set when the mail was returned to its
    -- sender. A returned mail cannot be returned again, so a return never
    -- loops; SS-M4's expiry quarantines instead of re-returning it.
    returned boolean DEFAULT false NOT NULL,
    -- Social-systems SS-M3: set when the recipient paid the COD. The item
    -- is then the recipient's: a paid COD cannot be returned (the seller
    -- would get the item and the price), and SS-M4's expiry must never
    -- return it (it takes the quarantine path instead).
    cod_paid boolean DEFAULT false NOT NULL,
    -- Social-systems SS-M1: a mail never carries negative cash; the send path
    -- refuses it first, and this is the backstop.
    CONSTRAINT sgw_gate_mail_cash_nonnegative_chk CHECK ((cash >= 0))
);

--
-- TOC entry 2641 (class 2604 OID 63849)
-- Name: mail_id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_gate_mail ALTER COLUMN mail_id SET DEFAULT nextval('sgw_gate_mail_mail_id_seq'::regclass);

