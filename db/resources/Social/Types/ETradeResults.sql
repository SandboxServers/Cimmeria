--
-- TOC entry 1121 (class 1247 OID 62610)
-- Name: ETradeResults; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "ETradeResults" AS ENUM (
    'Completed',
    'Cancelled',
    'NoLocalSpace',
    'NoRemoteSpace',
    'NoLocalCash',
    'NoRemoteCash'
);

