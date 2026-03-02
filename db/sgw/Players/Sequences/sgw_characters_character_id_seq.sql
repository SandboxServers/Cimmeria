--
-- TOC entry 271 (class 1259 OID 63789)
-- Name: sgw_characters_character_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE sgw_characters_character_id_seq
    START WITH 61
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--
-- TOC entry 2705 (class 0 OID 0)
-- Dependencies: 271
-- Name: sgw_characters_character_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE sgw_characters_character_id_seq OWNED BY sgw_player.player_id;

