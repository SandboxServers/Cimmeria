--
-- TOC entry 301 (class 1259 OID 64128)
-- Name: mission_rewards_reward_id_seq; Type: SEQUENCE; Schema: resources; Owner: -
--

CREATE SEQUENCE mission_rewards_reward_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 3290 (class 0 OID 0)
-- Dependencies: 301
-- Name: mission_rewards_reward_id_seq; Type: SEQUENCE OWNED BY; Schema: resources; Owner: -
--

ALTER SEQUENCE mission_rewards_reward_id_seq OWNED BY mission_rewards.reward_id;

