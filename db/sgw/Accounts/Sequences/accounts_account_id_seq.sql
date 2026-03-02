--
-- TOC entry 269 (class 1259 OID 63755)
-- Name: accounts_account_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE accounts_account_id_seq
    START WITH 2
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 2704 (class 0 OID 0)
-- Dependencies: 269
-- Name: accounts_account_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE accounts_account_id_seq OWNED BY account.account_id;

