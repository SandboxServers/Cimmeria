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
    -- Minimum 3 seconds — see `entity_templates.respawn_secs`
    -- comment for the floor rationale.
    respawn_secs integer,
    -- Per-spawn patrol override. When set, takes precedence over the
    -- template's `entity_templates.patrol_path_id` / `patrol_point_delay` for
    -- THIS spawn only. `patrol_path_id` references a `point_sets.set_id` whose
    -- `point_set_points` are the ordered waypoints. NULL → fall back to the
    -- template default (and NULL there → no patrol). Authored in-game via the
    -- `.path_assign` console command.
    patrol_path_id integer,
    patrol_point_delay real,
    CONSTRAINT spawnlist_respawn_secs_min_3
        CHECK (respawn_secs IS NULL OR respawn_secs >= 3),
    -- `patrol_point_delay` is a per-waypoint dwell in seconds; a negative
    -- value is meaningless and would poison the patrol scheduler. Guard it
    -- at the schema boundary so a bad seed edit can't persist.
    CONSTRAINT spawnlist_patrol_point_delay_nonneg
        CHECK (patrol_point_delay IS NULL OR patrol_point_delay >= 0)
);

--
-- TOC entry 2887 (class 2604 OID 63186)
-- Name: spawn_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY spawnlist ALTER COLUMN spawn_id SET DEFAULT nextval('spawnlist_spawn_id_seq'::regclass);

