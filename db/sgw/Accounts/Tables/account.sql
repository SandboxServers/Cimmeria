--
-- TOC entry 268 (class 1259 OID 63748)
-- Name: account; Type: TABLE; Schema: public; Owner: -; Tablespace: 
--

CREATE TABLE account (
    account_id integer NOT NULL,
    account_name character varying(32) DEFAULT NULL::character varying NOT NULL,
    password character varying(64) DEFAULT NULL::character varying NOT NULL,
    accesslevel integer DEFAULT 0 NOT NULL,
    enabled boolean DEFAULT true NOT NULL
);

--
-- TOC entry 2608 (class 2604 OID 63848)
-- Name: account_id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY account ALTER COLUMN account_id SET DEFAULT nextval('accounts_account_id_seq'::regclass);

