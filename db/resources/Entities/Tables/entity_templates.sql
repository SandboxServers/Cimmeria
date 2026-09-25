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
    -- Wander radius in world units.
    --
    -- `NULL` → NPC doesn't wander. The loader COALESCEs NULL → 0.0
    -- at runtime, which the AI tick treats as "no wander".
    -- A literal `0.0` is rejected by the
    -- `entity_templates_wander_radius_positive` CHECK constraint
    -- below — use NULL to disable.
    --
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
    -- Per-tick movement speed, in world units per 100ms tick (the same
    -- unit the runtime's `CellEntity::move_speed` field already uses).
    -- NULL → runtime falls back to the historical hardcoded default of
    -- 0.6 (6.0 units/sec), set in
    -- `crates/entity/src/cell_entity/construction.rs`.
    --
    -- 0.6 units/tick was measured against World 12's player run speed
    -- (8.125 units/sec) and found to be 26% too slow — a follower NPC
    -- pathing at the default would never close the follow-distance
    -- band against a moving player and would trail further every
    -- hallway (GC1b-0 feasibility pass, Castle Cellblock escort work).
    -- Set this column on templates that need to keep pace with (or
    -- catch up to) a player, e.g. escort/companion NPCs.
    move_speed real,
    -- Leash radius in world units, measured from the NPC's own position to
    -- its spawn point (horizontal distance; NA12 / D-NA03 / D-NA09).
    -- NULL → the runtime default `cell::combat::LEASH_DISTANCE` (50).
    -- An NPC more than this far from spawn (plus a 5-unit hysteresis
    -- band) gives up the fight and walks home. Set it on templates that
    -- should chase further (bosses) or give up sooner (sentries).
    leash_distance real,
    -- Proximity-aggro radius in world units (NA13 / D-NA01 / D-NA09),
    -- horizontal distance from the NPC. NULL → the runtime default
    -- `cell::combat::DEFAULT_AGGRO_RADIUS` (18). An Idle NPC that is hostile
    -- to players (its spawn's `aggression_override`, else the faction
    -- reaction) engages a player inside this radius, within 4 u of its
    -- height and in navmesh line of sight.
    aggro_radius real,
    -- Assist radius in world units (NA14 / D-NA04 / D-NA09), horizontal
    -- distance from this NPC to a same-faction NPC that has just engaged.
    -- NULL → the runtime default `cell::combat::DEFAULT_ASSIST_RADIUS` (10).
    -- A hostile Idle / patrolling / wandering NPC inside this radius of a
    -- neighbour that enters Fighting from damage or proximity, within 4 u of
    -- its height and in navmesh line of sight, joins on the same target.
    -- Assist does not chain. A deviation from legacy, which had no assist.
    assist_radius real,
    CONSTRAINT entity_templates_assist_radius_positive
        CHECK (assist_radius IS NULL OR assist_radius > 0.0),
    CONSTRAINT entity_templates_aggro_radius_positive
        CHECK (aggro_radius IS NULL OR aggro_radius > 0.0),
    CONSTRAINT entity_templates_leash_distance_positive
        CHECK (leash_distance IS NULL OR leash_distance > 0.0),
    CONSTRAINT entity_templates_move_speed_positive
        CHECK (move_speed IS NULL OR move_speed > 0.0),
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

