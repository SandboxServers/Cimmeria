--
-- TOC entry 1025 (class 1247 OID 61896)
-- Name: EPatrolType; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EPatrolType" AS ENUM (
    'PATROL_TYPE_OneWay',
    'PATROL_TYPE_BackAndForth',
    'PATROL_TYPE_Loop'
);

