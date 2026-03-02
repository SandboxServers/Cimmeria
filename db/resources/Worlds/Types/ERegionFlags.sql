--
-- TOC entry 1046 (class 1247 OID 61946)
-- Name: ERegionFlags; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "ERegionFlags" AS ENUM (
    'REGION_FLAG_ClientHinted',
    'REGION_FLAG_Stargate',
    'REGION_FLAG_PvPZone',
    'REGION_FLAG_NonPvPZone'
);

