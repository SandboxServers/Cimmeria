--
-- TOC entry 229 (class 1259 OID 63004)
-- Name: mission_reward_groups; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE mission_reward_groups (
    mission_id integer NOT NULL,
    group_id integer NOT NULL,
    choices integer DEFAULT 1 NOT NULL
);

--
-- TOC entry 2868 (class 2604 OID 63179)
-- Name: group_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY mission_reward_groups ALTER COLUMN group_id SET DEFAULT nextval('mission_reward_groups_group_id_seq'::regclass);

