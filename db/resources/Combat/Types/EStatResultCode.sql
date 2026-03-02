--
-- TOC entry 1079 (class 1247 OID 62198)
-- Name: EStatResultCode; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EStatResultCode" AS ENUM (
    'SRC_None',
    'SRC_Absorb',
    'SRC_Immune',
    'SRC_Mortal'
);

