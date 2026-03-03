--
-- TOC entry 785 (class 1247 OID 60012)
-- Name: EActionHandlerEvent; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EActionHandlerEvent" AS ENUM (
    'EACTIONHANDLEREVENT_PreExecute',
    'EACTIONHANDLEREVENT_PostExecute',
    'EACTIONHANDLEREVENT_PostFailExecute'
);

