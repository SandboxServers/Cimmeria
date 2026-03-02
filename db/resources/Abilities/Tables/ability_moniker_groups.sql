--
-- TOC entry 181 (class 1259 OID 62783)
-- Name: ability_moniker_groups; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE ability_moniker_groups (
    ability_id integer NOT NULL,
    moniker_ids bigint[] NOT NULL,
    group_id integer NOT NULL
);

--
-- TOC entry 2813 (class 2604 OID 63168)
-- Name: group_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY ability_moniker_groups ALTER COLUMN group_id SET DEFAULT nextval('ability_moniker_groups_group_id_seq'::regclass);

