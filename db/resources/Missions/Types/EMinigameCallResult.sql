--
-- TOC entry 983 (class 1247 OID 61640)
-- Name: EMinigameCallResult; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EMinigameCallResult" AS ENUM (
    'MCR_Accepted',
    'MCR_Declined',
    'MCR_NoCash',
    'MCR_ZeroCash',
    'MCR_TimedOut',
    'MCR_NotAvailable',
    'MCR_WrongAlignment',
    'MCR_TimeLimit',
    'MCR_HelpSelf',
    'MCR_NoInstance'
);

