--
-- TOC entry 193 (class 1259 OID 62832)
-- Name: body_sets; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

CREATE TABLE body_sets (
    body_set character varying(500) NOT NULL,
    ref_skeletal_mesh character varying(500) NOT NULL,
    -- Eye height above the entity's position (its feet), BigWorld metres
    -- (NA31). Line of sight is cast between eyes: NPC aggro, assist and
    -- attack checks and the player fire-time check. Measured from the
    -- reference skeletal mesh's bounds in the cooked client package
    -- (FBoxSphereBounds, 100 UE3 units per metre): the top of the bounds
    -- less 0.12 m (the humanoid head-mesh centres sit 0.11-0.13 m below
    -- the top), or the bounds' centre if that is higher (tiny creatures).
    -- NULL -> the runtime default `DEFAULT_EYE_HEIGHT` (1.5). See
    -- docs/reverse-engineering/findings/being-eye-heights.md.
    eye_height real,
    CONSTRAINT body_sets_eye_height_positive
        CHECK (eye_height IS NULL OR eye_height > 0.0)
);

