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
    -- Ability-tree v2 columns (docs/analysis/ability-trees, AT-01). The
    -- defaults make a row without them behave as before: no spend gate,
    -- one training point per node.
    required_branch_points integer DEFAULT 0 NOT NULL,
    skill_point_cost integer DEFAULT 1 NOT NULL,
    is_branch_root boolean DEFAULT false NOT NULL,
    is_capstone boolean DEFAULT false NOT NULL,
    branch_name text,
    project_status text,
    CONSTRAINT tree_index_sanity CHECK (((tree_index >= 0) AND (tree_index <= 2))),
    CONSTRAINT skill_point_cost_sanity CHECK ((skill_point_cost >= 0))
);

