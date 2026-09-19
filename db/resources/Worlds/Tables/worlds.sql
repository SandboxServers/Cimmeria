--
-- TOC entry 266 (class 1259 OID 63147)
-- Name: worlds; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE worlds (
    world_id integer NOT NULL,
    flags integer NOT NULL,
    min_per_day integer NOT NULL,
    min_to_real_min integer NOT NULL,
    world character varying(255) NOT NULL,
    cell_id integer,
    gravity real DEFAULT 7.5 NOT NULL,
    run_speed real DEFAULT 8.125 NOT NULL,
    sideways_run_speed real DEFAULT 8.125 NOT NULL,
    backwards_run_speed real DEFAULT 6.09375 NOT NULL,
    walk_speed real DEFAULT 2.069 NOT NULL,
    sideways_walk_speed real DEFAULT 2.069 NOT NULL,
    backwards_walk_speed real DEFAULT 3.796875 NOT NULL,
    crouch_run_speed real DEFAULT 5.0625 NOT NULL,
    sideways_crouch_run_speed real DEFAULT 5.0625 NOT NULL,
    backwards_crouch_run_speed real DEFAULT 3.796875 NOT NULL,
    crouch_walk_speed real DEFAULT 0.935 NOT NULL,
    sideways_crouch_walk_speed real DEFAULT 0.70125 NOT NULL,
    backwards_crouch_walk_speed real DEFAULT 0.935 NOT NULL,
    swim_speed real DEFAULT 4 NOT NULL,
    sideways_swim_speed real DEFAULT 4 NOT NULL,
    backwards_swim_speed real DEFAULT 1 NOT NULL,
    jump_speed real DEFAULT 6 NOT NULL,
    has_script boolean DEFAULT false NOT NULL,
    client_map character varying(100) NOT NULL,
    -- How the cell treats this world's `data/spaces/<world>.nav` mesh.
    --
    --   'enforce'  (default) the mesh is a containment gate: a non-GM client
    --              position that is off the walkable mesh is rejected and the
    --              player is snapped back, and an authored arrival that is off
    --              the mesh is refused or redirected.
    --   'advisory' the mesh is information only. Pathing, line of sight and
    --              surface-height sampling still use it; nothing gates on it.
    --              A world whose mesh has holes a player can legitimately walk
    --              through belongs here, because a partial mesh used as a gate
    --              fails CLOSED — an ordinary player cannot cross the hole,
    --              while a GM (warn-only) never notices.
    --
    -- Explicit per world on purpose: a security-relevant movement gate must
    -- never turn itself off because some heuristic decided the mesh looked
    -- bad. See docs/architecture/navmesh-containment-modes.md.
    navmesh_mode character varying(16) DEFAULT 'enforce' NOT NULL
        CHECK (navmesh_mode IN ('enforce', 'advisory'))
);

