--
-- TOC entry 881 (class 1247 OID 60922)
-- Name: EDuelState; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EDuelState" AS ENUM (
    'EDUEL_STATE_None',
    'EDUEL_STATE_ResponsePending',
    'EDUEL_STATE_Challenged',
    'EDUEL_STATE_StartPending',
    'EDUEL_STATE_Engaged'
);

