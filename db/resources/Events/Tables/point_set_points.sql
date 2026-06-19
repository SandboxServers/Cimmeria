--
-- TOC entry 237 (class 1259 OID 63038)
-- Name: point_set_points; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE point_set_points (
    set_id integer NOT NULL,
    point_id integer NOT NULL,
    x real NOT NULL,
    y real NOT NULL,
    z real NOT NULL,
    yaw real DEFAULT 0 NOT NULL,
    pitch real DEFAULT 0 NOT NULL,
    roll real DEFAULT 0 NOT NULL,
    -- Patrol-waypoint extensions. When a point set is used as a patrol
    -- path, these per-waypoint columns drive on-arrival behavior. All NULL for
    -- ordinary point sets (area triggers, ring pads, etc.).
    -- `sequence_id`: Kismet sequence to play when an NPC reaches this waypoint.
    -- `teleport_*`: optional teleport destination; on arrival the NPC plays the
    -- departure sequence, waits `teleport_delay` seconds, teleports, then plays
    -- `teleport_sequence_id`. Authored via the `.path_set_*` console commands.
    sequence_id integer,
    teleport_x real,
    teleport_y real,
    teleport_z real,
    teleport_sequence_id integer,
    teleport_delay real,
    -- The teleport destination is all-or-none: either all three coords are
    -- set (a real destination) or all NULL (no teleport on this waypoint).
    -- The `.path_set_teleport` console flow treats them as one destination,
    -- so a partial coordinate set is an invalid state — reject it here.
    CONSTRAINT point_set_points_teleport_xyz_all_or_none
        CHECK (
            (teleport_x IS NULL AND teleport_y IS NULL AND teleport_z IS NULL)
            OR (teleport_x IS NOT NULL AND teleport_y IS NOT NULL AND teleport_z IS NOT NULL)
        ),
    -- `teleport_delay` is a dwell in seconds before the teleport fires; a
    -- negative value is meaningless. Guard it at the schema boundary.
    CONSTRAINT point_set_points_teleport_delay_nonneg
        CHECK (teleport_delay IS NULL OR teleport_delay >= 0)
);

--
-- TOC entry 2876 (class 2604 OID 63181)
-- Name: point_id; Type: DEFAULT; Schema: resources; Owner: -
--

ALTER TABLE ONLY point_set_points ALTER COLUMN point_id SET DEFAULT nextval('point_set_points_point_id_seq'::regclass);

