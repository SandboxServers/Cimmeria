--
-- TOC entry 1043 (class 1247 OID 61936)
-- Name: ERegionEvents; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "ERegionEvents" AS ENUM (
    'REGION_EVENT_OnEnter',
    'REGION_EVENT_OnExit',
    'REGION_EVENT_TeleportOut',
    'REGION_EVENT_TeleportIn'
);

