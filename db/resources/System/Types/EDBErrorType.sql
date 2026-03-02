--
-- TOC entry 863 (class 1247 OID 60820)
-- Name: EDBErrorType; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EDBErrorType" AS ENUM (
    'EDB_ERROR_Insufficient_info',
    'EDB_ERROR_No_such_player',
    'EDB_ERROR_Player_in_org_type',
    'EDB_ERROR_Player_rank_too_low',
    'EDB_ERROR_Invoker_not_org_member',
    'EDB_ERROR_Target_not_org_member',
    'EDB_ERROR_Invalid_org_rank'
);

