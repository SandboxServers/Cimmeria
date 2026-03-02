--
-- TOC entry 1103 (class 1247 OID 62488)
-- Name: ETargetType; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "ETargetType" AS ENUM (
    'TargetNONE',
    'TargetSelf',
    'TargetTarget',
    'TargetGround'
);

