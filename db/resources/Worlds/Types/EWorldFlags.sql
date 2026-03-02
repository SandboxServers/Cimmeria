--
-- TOC entry 1142 (class 1247 OID 62710)
-- Name: EWorldFlags; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EWorldFlags" AS ENUM (
    'WF_None',
    'WF_Instance',
    'WF_ForceTravelWhenReady',
    'WF_TeardownWhenIdle'
);

