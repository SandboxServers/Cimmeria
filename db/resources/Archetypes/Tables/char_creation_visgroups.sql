--
-- TOC entry 197 (class 1259 OID 62849)
-- Name: char_creation_visgroups; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE char_creation_visgroups (
    vis_group_id integer NOT NULL,
    char_def_id integer NOT NULL,
    text character varying(255) NOT NULL,
    vis_type "EVisualGroupType" NOT NULL
);

