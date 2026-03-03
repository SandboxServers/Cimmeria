--
-- TOC entry 196 (class 1259 OID 62844)
-- Name: char_creation_choices; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE char_creation_choices (
    choice_id integer NOT NULL,
    vis_group_id integer NOT NULL,
    component character varying(255) NOT NULL,
    item_id integer,
    item_bound boolean DEFAULT false NOT NULL,
    item_durability integer DEFAULT (-1)
);

