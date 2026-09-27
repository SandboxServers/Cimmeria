--
-- Cover nodes: individual cover positions within a cover set, in world
-- space. Position is in BigWorld meters (BW = (ue.y, ue.z, ue.x) / 100);
-- `orient` is the node's defensive facing in radians, measured from BW +X
-- toward +Z (facing = (cos, sin) in (x, z)) -- the convention
-- `cell::cover::scoring` consumes, NOT the entity-yaw convention.
-- `height` and `quality` use the ECoverHeight / ECoverQuality enums
-- (byte ordinals of the client's CoverHeight / CoverQuality properties).
-- `width` is the marker's CoverWidth in meters. `tail` is a legacy column
-- from the retired covernodes_*.pak record format (4 unexplained bytes);
-- extracted rows carry zeros.
--
-- Per set, node_id is 0-based and stable per extraction of one client
-- build. The (chunk_id, node_id) composite is the natural key.
--
-- Name: cover_nodes; Type: TABLE; Schema: resources; Owner: -
--

CREATE TABLE cover_nodes (
    chunk_id integer NOT NULL,
    node_id integer NOT NULL,
    pos_x real NOT NULL,
    pos_y real NOT NULL,
    pos_z real NOT NULL,
    orient real NOT NULL,
    height "ECoverHeight" NOT NULL,
    quality "ECoverQuality" NOT NULL,
    width real NOT NULL,
    tail bytea NOT NULL
);
