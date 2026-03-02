--
-- TOC entry 1097 (class 1247 OID 62438)
-- Name: ETargetCollectionMethod; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "ETargetCollectionMethod" AS ENUM (
    'TCM_None',
    'TCM_Single',
    'TCM_AERadius',
    'TCM_AECone',
    'TCM_Group',
    'TCM_Aura',
    'TCM_RandomSingle'
);

