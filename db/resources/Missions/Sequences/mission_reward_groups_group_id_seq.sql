--
-- TOC entry 230 (class 1259 OID 63008)
-- Name: mission_reward_groups_group_id_seq; Type: SEQUENCE; Schema: resources; Owner: -
--

CREATE SEQUENCE mission_reward_groups_group_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 3289 (class 0 OID 0)
-- Dependencies: 230
-- Name: mission_reward_groups_group_id_seq; Type: SEQUENCE OWNED BY; Schema: resources; Owner: -
--

ALTER SEQUENCE mission_reward_groups_group_id_seq OWNED BY mission_reward_groups.group_id;

