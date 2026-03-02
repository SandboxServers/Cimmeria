--
-- TOC entry 839 (class 1247 OID 60742)
-- Name: EContactListEvent; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EContactListEvent" AS ENUM (
    'ECONTACT_LIST_EVENT_LoggedInStatus',
    'ECONTACT_LIST_EVENT_GainLevel',
    'ECONTACT_LIST_EVENT_Death',
    'ECONTACT_LIST_EVENT_GateTravel'
);

