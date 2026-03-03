--
-- TOC entry 866 (class 1247 OID 60836)
-- Name: EDamageType; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EDamageType" AS ENUM (
    'DT_Untyped',
    'DT_Energy',
    'DT_Hazmat',
    'DT_Physical',
    'DT_Psionic'
);

