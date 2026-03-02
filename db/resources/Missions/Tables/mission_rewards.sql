--
-- TOC entry 302 (class 1259 OID 64130)
-- Name: mission_rewards; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE mission_rewards (
    reward_id integer NOT NULL,
    group_id integer NOT NULL,
    item_id integer NOT NULL
);

--
-- TOC entry 2908 (class 2604 OID 64133)
-- Name: reward_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY mission_rewards ALTER COLUMN reward_id SET DEFAULT nextval('mission_rewards_reward_id_seq'::regclass);

