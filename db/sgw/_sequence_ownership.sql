--
-- TOC entry 2704 (class 0 OID 0)
-- Dependencies: 269
-- Name: accounts_account_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE accounts_account_id_seq OWNED BY account.account_id;

--
-- TOC entry 2705 (class 0 OID 0)
-- Dependencies: 271
-- Name: sgw_characters_character_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE sgw_characters_character_id_seq OWNED BY sgw_player.player_id;

--
-- TOC entry 2706 (class 0 OID 0)
-- Dependencies: 273
-- Name: sgw_gate_mail_mail_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE sgw_gate_mail_mail_id_seq OWNED BY sgw_gate_mail.mail_id;

--
-- TOC entry 2707 (class 0 OID 0)
-- Dependencies: 276
-- Name: sgw_inventory_base_item_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE sgw_inventory_base_item_id_seq OWNED BY sgw_inventory_base.item_id;

--
-- TOC entry 2708 (class 0 OID 0)
-- Dependencies: 277
-- Name: sgw_inventory_item_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE sgw_inventory_item_id_seq OWNED BY sgw_inventory.item_id;

