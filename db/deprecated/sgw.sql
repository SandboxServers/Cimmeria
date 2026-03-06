SET statement_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SET check_function_bodies = false;
SET client_min_messages = warning;

SET search_path = public, pg_catalog;

SET default_tablespace = '';

--
-- TOC entry 268 (class 1259 OID 63748)
-- Name: account; Type: TABLE; Schema: public; Owner: -; Tablespace: 
--

CREATE TABLE account (
    account_id integer NOT NULL,
    account_name character varying(32) DEFAULT NULL::character varying NOT NULL,
    password character varying(64) DEFAULT NULL::character varying NOT NULL,
    accesslevel integer DEFAULT 0 NOT NULL,
    enabled boolean DEFAULT true NOT NULL
);


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

CREATE TYPE player_interaction_map AS ("template_id" int4, "interaction_set_map_id" int4, "mission_id" int4);

--
-- TOC entry 270 (class 1259 OID 63757)
-- Name: sgw_player; Type: TABLE; Schema: public; Owner: -; Tablespace: 
--

CREATE TABLE sgw_player (
    account_id integer NOT NULL,
    player_id integer NOT NULL,
    level integer DEFAULT 1 NOT NULL,
    alignment integer NOT NULL,
    archetype integer NOT NULL,
    gender integer NOT NULL,
    player_name character varying(64) DEFAULT NULL::character varying NOT NULL,
    extra_name character varying(64) DEFAULT NULL::character varying NOT NULL,
    world_location character varying(64) DEFAULT NULL::character varying NOT NULL,
    bodyset character varying(64) DEFAULT NULL::character varying NOT NULL,
    title integer DEFAULT 0 NOT NULL,
    pos_x real NOT NULL,
    pos_y real NOT NULL,
    pos_z real NOT NULL,
    heading smallint DEFAULT 0 NOT NULL,
    naquadah integer DEFAULT 0 NOT NULL,
    exp integer DEFAULT 0 NOT NULL,
    first_login integer DEFAULT 1 NOT NULL,
    world_id integer,
    known_stargates integer[] DEFAULT '{}'::integer[] NOT NULL,
    components character varying(200)[] DEFAULT '{}'::character varying[] NOT NULL,
    abilities integer[] DEFAULT '{}'::integer[] NOT NULL,
    access_level integer DEFAULT 0 NOT NULL,
    skin_color_id integer NOT NULL,
    bandolier_slot integer DEFAULT 0 NOT NULL,
    interaction_maps player_interaction_map[],
    training_points integer DEFAULT 0 NOT NULL,
    discipline_ids integer[] DEFAULT '{}'::integer[] NOT NULL,
    racial_paradigm_levels integer[] DEFAULT '{}'::integer[] NOT NULL,
    applied_science_points integer DEFAULT 0 NOT NULL,
    blueprint_ids integer[] DEFAULT '{}'::integer[] NOT NULL,
    known_respawners integer[] DEFAULT '{}'::integer[] NOT NULL,
    CONSTRAINT alignment_sanity CHECK (((alignment >= 0) AND (alignment <= 5))),
    CONSTRAINT archetype_sanity CHECK (((archetype >= 0) AND (archetype <= 8))),
    CONSTRAINT bandolier_slot_sanity CHECK (((bandolier_slot >= 0) AND (bandolier_slot <= 3))),
    CONSTRAINT gender_sanity CHECK (((gender >= 1) AND (gender <= 3))),
    CONSTRAINT level_sanity CHECK (((level >= 0) AND (level <= 20))),
    CONSTRAINT skin_color_sanity CHECK (((skin_color_id >= 0) AND (skin_color_id <= 15)))
);


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


--
-- TOC entry 272 (class 1259 OID 63791)
-- Name: sgw_gate_mail; Type: TABLE; Schema: public; Owner: -; Tablespace: 
--

CREATE TABLE sgw_gate_mail (
    mail_id integer NOT NULL,
    character_id integer NOT NULL,
    sender_id integer,
    subject character varying(128) DEFAULT NULL::character varying NOT NULL,
    message text NOT NULL,
    cash bigint DEFAULT 0 NOT NULL,
    sent_time integer NOT NULL,
    read_time integer NOT NULL,
    flags integer DEFAULT 0 NOT NULL,
    item_id integer,
    sender_name character varying(128) DEFAULT ''::character varying NOT NULL
);


--
-- TOC entry 273 (class 1259 OID 63801)
-- Name: sgw_gate_mail_mail_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE sgw_gate_mail_mail_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- TOC entry 2706 (class 0 OID 0)
-- Dependencies: 273
-- Name: sgw_gate_mail_mail_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE sgw_gate_mail_mail_id_seq OWNED BY sgw_gate_mail.mail_id;


--
-- TOC entry 274 (class 1259 OID 63803)
-- Name: sgw_inventory_base; Type: TABLE; Schema: public; Owner: -; Tablespace: 
--

CREATE TABLE sgw_inventory_base (
    item_id integer NOT NULL,
    stack_size integer DEFAULT 1 NOT NULL,
    charges integer DEFAULT 0 NOT NULL,
    container_id integer NOT NULL,
    slot_id integer NOT NULL,
    durability integer DEFAULT (-1) NOT NULL,
    type_id integer NOT NULL,
    flags integer DEFAULT 0 NOT NULL,
    ammo_type resources."EAmmoType" DEFAULT 'AMMO_NONE'::resources."EAmmoType" NOT NULL,
    ammo_types resources."EAmmoType"[] DEFAULT '{}'::resources."EAmmoType"[] NOT NULL
);


--
-- TOC entry 275 (class 1259 OID 63815)
-- Name: sgw_inventory; Type: TABLE; Schema: public; Owner: -; Tablespace: 
--

CREATE TABLE sgw_inventory (
    item_id integer,
    stack_size integer DEFAULT 1,
    charges integer DEFAULT 0,
    container_id integer,
    slot_id integer,
    durability integer DEFAULT (-1),
    type_id integer,
    flags integer DEFAULT 0,
    character_id integer NOT NULL,
    bound boolean DEFAULT false NOT NULL,
    ammo integer DEFAULT 0 NOT NULL,
    CONSTRAINT local_id_check CHECK ((item_id >= 10000))
)
INHERITS (sgw_inventory_base);


--
-- TOC entry 276 (class 1259 OID 63830)
-- Name: sgw_inventory_base_item_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE sgw_inventory_base_item_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- TOC entry 2707 (class 0 OID 0)
-- Dependencies: 276
-- Name: sgw_inventory_base_item_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE sgw_inventory_base_item_id_seq OWNED BY sgw_inventory_base.item_id;


--
-- TOC entry 277 (class 1259 OID 63832)
-- Name: sgw_inventory_item_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE sgw_inventory_item_id_seq
    START WITH 10221
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- TOC entry 2708 (class 0 OID 0)
-- Dependencies: 277
-- Name: sgw_inventory_item_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE sgw_inventory_item_id_seq OWNED BY sgw_inventory.item_id;


--
-- TOC entry 278 (class 1259 OID 63834)
-- Name: sgw_mission; Type: TABLE; Schema: public; Owner: -; Tablespace: 
--

CREATE TABLE sgw_mission (
    player_id integer NOT NULL,
    mission_id integer NOT NULL,
    current_step_id integer,
    status integer NOT NULL,
    completed_step_ids integer[] NOT NULL,
    completed_objective_ids integer[] NOT NULL,
    active_objective_ids integer[] NOT NULL,
    failed_objective_ids integer[] NOT NULL,
    repeats integer DEFAULT 0 NOT NULL
);


--
-- TOC entry 279 (class 1259 OID 63844)
-- Name: shards; Type: TABLE; Schema: public; Owner: -; Tablespace: 
--

CREATE TABLE shards (
    shard_id integer NOT NULL,
    name character varying(100),
    key character varying(100) NOT NULL,
    protected boolean DEFAULT false NOT NULL
);


--
-- TOC entry 2608 (class 2604 OID 63848)
-- Name: account_id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY account ALTER COLUMN account_id SET DEFAULT nextval('accounts_account_id_seq'::regclass);


--
-- TOC entry 2641 (class 2604 OID 63849)
-- Name: mail_id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_gate_mail ALTER COLUMN mail_id SET DEFAULT nextval('sgw_gate_mail_mail_id_seq'::regclass);


--
-- TOC entry 2655 (class 2604 OID 63850)
-- Name: item_id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_inventory ALTER COLUMN item_id SET DEFAULT nextval('sgw_inventory_item_id_seq'::regclass);


--
-- TOC entry 2656 (class 2604 OID 63851)
-- Name: ammo_type; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_inventory ALTER COLUMN ammo_type SET DEFAULT 'AMMO_NONE'::resources."EAmmoType";


--
-- TOC entry 2657 (class 2604 OID 63852)
-- Name: ammo_types; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_inventory ALTER COLUMN ammo_types SET DEFAULT '{}'::resources."EAmmoType"[];


--
-- TOC entry 2648 (class 2604 OID 63853)
-- Name: item_id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_inventory_base ALTER COLUMN item_id SET DEFAULT nextval('sgw_inventory_base_item_id_seq'::regclass);


--
-- TOC entry 2629 (class 2604 OID 63854)
-- Name: player_id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_player ALTER COLUMN player_id SET DEFAULT nextval('sgw_characters_character_id_seq'::regclass);


--
-- TOC entry 2688 (class 0 OID 63748)
-- Dependencies: 268
-- Data for Name: account; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO account (account_id, account_name, password, accesslevel, enabled) VALUES (2, 'test', 'a94a8fe5ccb19ba61c4c0873d391e987982fbbd3', 1, true);


--
-- TOC entry 2709 (class 0 OID 0)
-- Dependencies: 269
-- Name: accounts_account_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('accounts_account_id_seq', 2, true);


--
-- TOC entry 2710 (class 0 OID 0)
-- Dependencies: 271
-- Name: sgw_characters_character_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('sgw_characters_character_id_seq', 61, true);


--
-- TOC entry 2692 (class 0 OID 63791)
-- Dependencies: 272
-- Data for Name: sgw_gate_mail; Type: TABLE DATA; Schema: public; Owner: -
--



--
-- TOC entry 2711 (class 0 OID 0)
-- Dependencies: 273
-- Name: sgw_gate_mail_mail_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('sgw_gate_mail_mail_id_seq', 1, false);



--
-- TOC entry 2694 (class 0 OID 63803)
-- Dependencies: 274
-- Data for Name: sgw_inventory_base; Type: TABLE DATA; Schema: public; Owner: -
--



--
-- TOC entry 2712 (class 0 OID 0)
-- Dependencies: 276
-- Name: sgw_inventory_base_item_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('sgw_inventory_base_item_id_seq', 1, false);


--
-- TOC entry 2713 (class 0 OID 0)
-- Dependencies: 277
-- Name: sgw_inventory_item_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('sgw_inventory_item_id_seq', 10234, true);

--
-- TOC entry 2690 (class 0 OID 63757)
-- Dependencies: 270
-- Data for Name: sgw_player; Type: TABLE DATA; Schema: public; Owner: -
--

--
-- TOC entry 2699 (class 0 OID 63844)
-- Dependencies: 279
-- Data for Name: shards; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO shards (shard_id, name, key, protected) VALUES (1, 'Test', '', false);


--
-- TOC entry 2662 (class 2606 OID 63856)
-- Name: account_pkey; Type: CONSTRAINT; Schema: public; Owner: -; Tablespace: 
--

ALTER TABLE ONLY account
    ADD CONSTRAINT account_pkey PRIMARY KEY (account_id);


--
-- TOC entry 2677 (class 2606 OID 63858)
-- Name: missions_pkey; Type: CONSTRAINT; Schema: public; Owner: -; Tablespace: 
--

ALTER TABLE ONLY sgw_mission
    ADD CONSTRAINT missions_pkey PRIMARY KEY (player_id, mission_id);


--
-- TOC entry 2670 (class 2606 OID 63860)
-- Name: sgw_gate_mail_pkey; Type: CONSTRAINT; Schema: public; Owner: -; Tablespace: 
--

ALTER TABLE ONLY sgw_gate_mail
    ADD CONSTRAINT sgw_gate_mail_pkey PRIMARY KEY (mail_id);


--
-- TOC entry 2672 (class 2606 OID 63862)
-- Name: sgw_inventory_base_pkey; Type: CONSTRAINT; Schema: public; Owner: -; Tablespace: 
--

ALTER TABLE ONLY sgw_inventory_base
    ADD CONSTRAINT sgw_inventory_base_pkey PRIMARY KEY (item_id);


--
-- TOC entry 2675 (class 2606 OID 63864)
-- Name: sgw_inventory_pkey; Type: CONSTRAINT; Schema: public; Owner: -; Tablespace: 
--

ALTER TABLE ONLY sgw_inventory
    ADD CONSTRAINT sgw_inventory_pkey PRIMARY KEY (item_id);


--
-- TOC entry 2664 (class 2606 OID 63866)
-- Name: sgw_player_pkey; Type: CONSTRAINT; Schema: public; Owner: -; Tablespace: 
--

ALTER TABLE ONLY sgw_player
    ADD CONSTRAINT sgw_player_pkey PRIMARY KEY (player_id);


--
-- TOC entry 2666 (class 2606 OID 63868)
-- Name: sgw_player_player_name_key; Type: CONSTRAINT; Schema: public; Owner: -; Tablespace: 
--

ALTER TABLE ONLY sgw_player
    ADD CONSTRAINT sgw_player_player_name_key UNIQUE (player_name);


--
-- TOC entry 2679 (class 2606 OID 63870)
-- Name: shards_pkey; Type: CONSTRAINT; Schema: public; Owner: -; Tablespace: 
--

ALTER TABLE ONLY shards
    ADD CONSTRAINT shards_pkey PRIMARY KEY (shard_id);


--
-- TOC entry 2667 (class 1259 OID 63871)
-- Name: mail_lookup_index; Type: INDEX; Schema: public; Owner: -; Tablespace: 
--

CREATE INDEX mail_lookup_index ON sgw_gate_mail USING btree (character_id);


--
-- TOC entry 2668 (class 1259 OID 63872)
-- Name: mail_reverse_lookup_index; Type: INDEX; Schema: public; Owner: -; Tablespace: 
--

CREATE INDEX mail_reverse_lookup_index ON sgw_gate_mail USING btree (sender_id);


--
-- TOC entry 2673 (class 1259 OID 63875)
-- Name: sgw_inventory_Index01; Type: INDEX; Schema: public; Owner: -; Tablespace: 
--

CREATE INDEX "sgw_inventory_Index01" ON sgw_inventory USING btree (character_id);


--
-- TOC entry 2687 (class 2606 OID 63876)
-- Name: missions_player_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_mission
    ADD CONSTRAINT missions_player_id_fkey FOREIGN KEY (player_id) REFERENCES sgw_player(player_id) ON UPDATE RESTRICT ON DELETE CASCADE;


--
-- TOC entry 2683 (class 2606 OID 63881)
-- Name: sgw_gate_mail_character_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_gate_mail
    ADD CONSTRAINT sgw_gate_mail_character_id_fkey FOREIGN KEY (character_id) REFERENCES sgw_player(player_id) ON UPDATE RESTRICT ON DELETE CASCADE;


--
-- TOC entry 2684 (class 2606 OID 63886)
-- Name: sgw_gate_mail_sender_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_gate_mail
    ADD CONSTRAINT sgw_gate_mail_sender_id_fkey FOREIGN KEY (sender_id) REFERENCES sgw_player(player_id) ON UPDATE RESTRICT ON DELETE SET NULL;


--
-- TOC entry 2685 (class 2606 OID 63891)
-- Name: sgw_inventory_character_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_inventory
    ADD CONSTRAINT sgw_inventory_character_id_fkey FOREIGN KEY (character_id) REFERENCES sgw_player(player_id) ON UPDATE RESTRICT ON DELETE CASCADE;


--
-- TOC entry 2686 (class 2606 OID 63896)
-- Name: sgw_inventory_type_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_inventory
    ADD CONSTRAINT sgw_inventory_type_id_fkey FOREIGN KEY (type_id) REFERENCES resources.items(item_id) ON UPDATE CASCADE ON DELETE RESTRICT;


--
-- TOC entry 2680 (class 2606 OID 63901)
-- Name: sgw_player_account_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_player
    ADD CONSTRAINT sgw_player_account_id_fkey FOREIGN KEY (account_id) REFERENCES account(account_id) ON UPDATE RESTRICT ON DELETE CASCADE;


--
-- TOC entry 2681 (class 2606 OID 63916)
-- Name: sgw_player_world_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_player
    ADD CONSTRAINT sgw_player_world_id_fkey FOREIGN KEY (world_id) REFERENCES resources.worlds(world_id) ON UPDATE RESTRICT ON DELETE RESTRICT;


--
-- TOC entry 2682 (class 2606 OID 63921)
-- Name: sgw_player_world_location_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_player
    ADD CONSTRAINT sgw_player_world_location_fkey FOREIGN KEY (world_location) REFERENCES resources.worlds(world) ON UPDATE RESTRICT ON DELETE RESTRICT;


-- Completed on 2014-10-18 11:54:09

--
-- PostgreSQL database dump complete
--

