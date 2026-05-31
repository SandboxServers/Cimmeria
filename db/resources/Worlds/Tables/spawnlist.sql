--
-- TOC entry 255 (class 1259 OID 63107)
-- Name: spawnlist; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE spawnlist (
    spawn_id integer NOT NULL,
    x real NOT NULL,
    y real NOT NULL,
    z real NOT NULL,
    heading real NOT NULL,
    world_id integer NOT NULL,
    template_id integer NOT NULL,
    tag character varying(100),
    set_name character varying(100),
    is_stationary boolean DEFAULT false NOT NULL,
    -- Per-spawn respawn-delay override in seconds. Takes precedence
    -- over `entity_templates.respawn_secs` when set; NULL falls back
    -- to the template default. When both are NULL the mob is
    -- one-shot. Lets level designers tune a boss encounter to a
    -- different cadence than the same template's trash spawns.
    -- Zero / negative values are rejected at the DB boundary so a
    -- typo in a spawn row fails fast instead of silently becoming
    -- a one-shot spawn (the runtime loader also downgrades
    -- non-positive values to NULL as a belt-and-suspenders fallback).
    respawn_secs integer,
    CONSTRAINT spawnlist_respawn_secs_positive
        CHECK (respawn_secs IS NULL OR respawn_secs > 0)
);

--
-- TOC entry 2887 (class 2604 OID 63186)
-- Name: spawn_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY spawnlist ALTER COLUMN spawn_id SET DEFAULT nextval('spawnlist_spawn_id_seq'::regclass);

