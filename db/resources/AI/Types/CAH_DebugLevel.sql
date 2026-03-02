--
-- TOC entry 767 (class 1247 OID 59904)
-- Name: CAH_DebugLevel; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "CAH_DebugLevel" AS ENUM (
    'NoDebug',
    'Clients',
    'Server',
    'ClientsAndServer'
);

