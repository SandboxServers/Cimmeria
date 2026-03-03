--
-- TOC entry 878 (class 1247 OID 60906)
-- Name: EDuelDefeatReason; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EDuelDefeatReason" AS ENUM (
    'EDUEL_DEFEAT_Health',
    'EDUEL_DEFEAT_LeftSquad',
    'EDUEL_DEFEAT_Connection',
    'EDUEL_DEFEAT_Range',
    'EDUEL_DEFEAT_Teleport',
    'EDUEL_DEFEAT_InDuel',
    'EDUEL_DEFEAT_Forfeit'
);

