--
-- TOC entry 821 (class 1247 OID 60160)
-- Name: ECASPosition; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "ECASPosition" AS ENUM (
    'CAS_POSITION_FRONT',
    'CAS_POSITION_FLANK',
    'CAS_POSITION_REAR',
    'CAS_POSITION_ABOVE',
    'CAS_POSITION_BELOW'
);

