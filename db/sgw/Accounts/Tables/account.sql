--
-- TOC entry 268 (class 1259 OID 63748)
-- Name: account; Type: TABLE; Schema: public; Owner: -; Tablespace:
--

CREATE TABLE account (
    account_id integer NOT NULL,
    account_name character varying(32) DEFAULT NULL::character varying NOT NULL,
    -- Legacy unsalted SHA-1 hash (uppercase hex), as the original 2009 client
    -- sent it. NULLable now: it is set to NULL once an account migrates to
    -- argon2id on its first plaintext (TLS) login.
    password character varying(64) DEFAULT NULL::character varying,
    -- argon2id PHC string (e.g. "$argon2id$v=19$m=65536,t=3,p=1$...$...").
    -- NULL until the account migrates from the legacy SHA-1 scheme.
    password_hash_v2 text,
    -- Password hashing scheme selector:
    --   1 = sha1_legacy  (verify against `password`)
    --   2 = argon2id     (verify against `password_hash_v2`)
    -- Accounts start at 1 and flip to 2 on first plaintext login.
    password_algo smallint DEFAULT 1 NOT NULL,
    accesslevel integer DEFAULT 0 NOT NULL,
    enabled boolean DEFAULT true NOT NULL
);

--
-- TOC entry 2608 (class 2604 OID 63848)
-- Name: account_id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY account ALTER COLUMN account_id SET DEFAULT nextval('accounts_account_id_seq'::regclass);

