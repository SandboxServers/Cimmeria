--
-- TOC entry 187 (class 1259 OID 62802)
-- Name: archetype_ability_tree; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE archetype_ability_tree (
    archetype "EArchetype" NOT NULL,
    ability_index integer NOT NULL,
    ability_id integer NOT NULL,
    tree_index integer NOT NULL,
    level integer DEFAULT 1 NOT NULL,
    prerequisite_abilities integer[] DEFAULT '{}'::integer[] NOT NULL,
    CONSTRAINT tree_index_sanity CHECK (((tree_index >= 0) AND (tree_index <= 2)))
);

