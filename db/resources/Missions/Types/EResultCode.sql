--
-- TOC entry 1058 (class 1247 OID 62016)
-- Name: EResultCode; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EResultCode" AS ENUM (
    'RC_None',
    'RC_Hit',
    'RC_Miss',
    'RC_Critical',
    'RC_DoubleCritical',
    'RC_Glancing'
);

