--
-- TOC entry 209 (class 1259 OID 62909)
-- Name: entity_templates; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE entity_templates (
    template_id integer NOT NULL,
    static_mesh character varying(200),
    body_set character varying(200) NOT NULL,
    components character varying(200)[],
    flags bigint DEFAULT 0 NOT NULL,
    interaction_type bigint DEFAULT 0 NOT NULL,
    event_set_id integer,
    level integer,
    alignment integer,
    faction integer,
    name_id integer,
    name character varying(200),
    patrol_path_id integer,
    patrol_point_delay real,
    template_name character varying(200) NOT NULL,
    class character varying(50) NOT NULL,
    buy_item_list integer,
    sell_item_list integer,
    repair_item_list integer,
    recharge_item_list integer,
    ability_set_id integer,
    ammo_type "EAmmoType",
    loot_table_id integer,
    primary_color_id bigint DEFAULT 0 NOT NULL,
    secondary_color_id bigint DEFAULT 0 NOT NULL,
    skin_tint bigint DEFAULT 0 NOT NULL,
    weapon_item_id integer,
    static_interaction_sets integer[] DEFAULT ARRAY[]::integer[] NOT NULL,
    trainer_ability_list_id integer,
    speaker_id integer,
    has_dynamic_properties boolean DEFAULT true NOT NULL,
    interaction_set_id integer,
    -- Per-template respawn delay in seconds. NULL = the template
    -- doesn't carry a default; per-spawn `spawnlist.respawn_secs`
    -- (if set) is the only source. If both are NULL the mob is
    -- one-shot (corpse never repopulates).
    --
    -- Minimum: 3 seconds. The death animation needs time to play
    -- before the corpse comes back, and the 1 Hz respawn tick
    -- needs a window where the dead-state wire packets reach the
    -- client before the alive-state ones; 1- and 2-second
    -- respawns produce a visible "die / instantly alive" glitch
    -- in the worst case. Recommended floor for typical mobs is
    -- 30 seconds; bosses 300+. Values below 3 are rejected at the
    -- DB boundary so misconfigured seed data fails fast (the
    -- runtime loader also downgrades non-positive values to NULL
    -- as a belt-and-suspenders fallback). See
    -- `cell_entity::respawn_secs` for the resolved field on the
    -- runtime entity.
    respawn_secs integer,
    -- Wander radius in world units. NULL or 0 → NPC doesn't wander.
    -- Positive value → idle NPCs without a patrol path pick random
    -- points within `wander_radius` of `spawn_position`, walk there,
    -- pause for a random dwell drawn from
    -- `[wander_min_dwell_secs, wander_max_dwell_secs]`, repeat. The
    -- runtime falls back to `[3.0, 8.0]` seconds when either dwell
    -- bound is NULL.
    wander_radius real,
    wander_min_dwell_secs real,
    wander_max_dwell_secs real,
    -- Follow-state distance band, in world units. NPCs targeted by
    -- `SetFollowTarget` walk toward their target until within
    -- `follow_max_distance`, then hold until the target moves out
    -- of the band. `follow_min_distance` is a no-op zone — the NPC
    -- doesn't back away when the target gets close. NULL on both
    -- → runtime defaults `[2.0, 5.0]`.
    follow_min_distance real,
    follow_max_distance real,
    CONSTRAINT entity_templates_respawn_secs_min_3
        CHECK (respawn_secs IS NULL OR respawn_secs >= 3),
    CONSTRAINT entity_templates_wander_radius_positive
        CHECK (wander_radius IS NULL OR wander_radius > 0.0),
    CONSTRAINT entity_templates_wander_dwell_positive
        CHECK (
            (wander_min_dwell_secs IS NULL OR wander_min_dwell_secs > 0.0)
            AND (wander_max_dwell_secs IS NULL OR wander_max_dwell_secs > 0.0)
            AND (
                wander_min_dwell_secs IS NULL
                OR wander_max_dwell_secs IS NULL
                OR wander_min_dwell_secs <= wander_max_dwell_secs
            )
        ),
    CONSTRAINT entity_templates_follow_distance_positive
        CHECK (
            (follow_min_distance IS NULL OR follow_min_distance > 0.0)
            AND (follow_max_distance IS NULL OR follow_max_distance > 0.0)
            AND (
                follow_min_distance IS NULL
                OR follow_max_distance IS NULL
                OR follow_min_distance <= follow_max_distance
            )
        )
);

--
-- TOC entry 2843 (class 2604 OID 63173)
-- Name: template_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY entity_templates ALTER COLUMN template_id SET DEFAULT nextval('entity_templates_template_id_seq'::regclass);

