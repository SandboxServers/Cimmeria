--
-- TOC entry 233 (class 1259 OID 63019)
-- Name: missions; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE missions (
    mission_id integer DEFAULT nextval('missions_mission_id_seq'::regclass) NOT NULL,
    history_text character varying(255) NOT NULL,
    award_xp boolean NOT NULL,
    can_abandon boolean NOT NULL,
    can_fail boolean NOT NULL,
    can_repeat_on_fail boolean NOT NULL,
    difficulty integer NOT NULL,
    is_a_story boolean NOT NULL,
    is_enabled boolean NOT NULL,
    is_hidden boolean NOT NULL,
    is_override_mission boolean NOT NULL,
    is_shareable boolean NOT NULL,
    level integer NOT NULL,
    mission_defn character varying(255) NOT NULL,
    mission_label character varying(255) NOT NULL,
    num_repeats integer NOT NULL,
    show_faction_change_icon boolean NOT NULL,
    show_instance_icon boolean NOT NULL,
    show_pvp_icon boolean NOT NULL,
    script_name character varying(100),
    reward_naq integer DEFAULT 0 NOT NULL,
    reward_xp integer DEFAULT 0 NOT NULL,
    script_spaces character varying(100)
);

