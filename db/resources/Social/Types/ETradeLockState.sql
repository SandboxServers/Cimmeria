--
-- TOC entry 1118 (class 1247 OID 62602)
-- Name: ETradeLockState; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "ETradeLockState" AS ENUM (
    'ETRADELOCKSTATE_None',
    'ETRADELOCKSTATE_Locked',
    'ETRADELOCKSTATE_LockedAndConfirmed'
);

