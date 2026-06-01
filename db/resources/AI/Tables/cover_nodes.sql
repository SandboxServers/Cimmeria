--
-- Cover nodes: individual cover positions within a cover set. Each
-- record is the on-disk 22-byte `CoverNodePrefabData` structure decoded
-- from the SGW client cache (covernodes_*.pak). Position is in BigWorld
-- meters (BW coord system); `orient` is the cover's defensive facing
-- in radians; `height` and `quality` use the existing ECoverHeight /
-- ECoverQuality enums; `tail` preserves the trailing 4 bytes whose
-- semantics are TBD (median-f32 value across the corpus suggests a
-- secondary lean angle near -π/2, but unconfirmed at v1).
--
-- Per chunk, node_id is 0-based and stable per extraction. The
-- (chunk_id, node_id) composite is the natural key.
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
    tail bytea NOT NULL
);
