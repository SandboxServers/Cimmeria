--
-- TOC entry 260 (class 1259 OID 63120)
-- Name: stargates; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE stargates (
    address1 integer NOT NULL,
    address2 integer NOT NULL,
    address3 integer NOT NULL,
    address4 integer NOT NULL,
    address5 integer NOT NULL,
    address6 integer NOT NULL,
    address_origin integer NOT NULL,
    stargate_id integer DEFAULT nextval('stargates_id_seq'::regclass) NOT NULL,
    name character varying(255) NOT NULL,
    pitch double precision NOT NULL,
    prefab_sequence character varying(255) NOT NULL,
    roll double precision NOT NULL,
    world_id integer NOT NULL,
    x_pos double precision NOT NULL,
    y_pos double precision NOT NULL,
    yaw double precision NOT NULL,
    z_pos double precision NOT NULL,
    event_set_id integer,
    arrival_x double precision,
    arrival_y double precision,
    arrival_z double precision,
    arrival_yaw double precision,
    CONSTRAINT stargates_arrival_all_or_nothing CHECK (
        (arrival_x IS NULL AND arrival_y IS NULL AND arrival_z IS NULL AND arrival_yaw IS NULL)
        OR
        (arrival_x IS NOT NULL AND arrival_y IS NOT NULL AND arrival_z IS NOT NULL AND arrival_yaw IS NOT NULL)
    )
);

--
-- Arrival columns (Harset H01).
--
-- `x_pos/y_pos/z_pos/yaw` are the STARGATE PROP's own transform — the
-- `GLB-Stargate_Prefab_Seq` origin lifted straight out of the cooked map.
-- The 2009 server arrived travellers on exactly that point
-- (`deprecated/python/cell/SGWPlayer.py:2129` — `moveTo(addr.xPos, addr.yPos,
-- addr.zPos, addr.yaw, ...)`), which was harmless because it never validated
-- containment. Harset is the first navmesh-backed destination: the prefab
-- origin there sits ~1.5 units above the floor with the nearest walkable
-- vertex ~5 units away in XZ, inside the prefab's own footprint carve-out, so
-- a traveller arrives off-mesh and every subsequent client position is
-- silently suppressed.
--
-- `arrival_*` is therefore NEW authoring, not a restoration: an absolute
-- "stand here on arrival" point, pinned in-game. Absolute rather than a
-- yaw-relative offset because the error has a vertical component a planar
-- offset cannot express, and because the pin session (milestone M0) produces
-- absolute coordinates from the map debug HUD.
--
-- All four are NULL or all four are set — a half-pinned row would read as a
-- valid partial arrival. `arrival_yaw` is a facing, not a delta; when the
-- group is NULL the loader falls back to `yaw`.

