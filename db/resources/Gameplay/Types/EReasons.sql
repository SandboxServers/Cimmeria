--
-- TOC entry 1040 (class 1247 OID 61926)
-- Name: EReasons; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EReasons" AS ENUM (
    'REAS_requested',
    'REAS_kicked',
    'REAS_disbanded',
    'REAS_logout'
);

