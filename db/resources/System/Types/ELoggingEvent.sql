--
-- TOC entry 956 (class 1247 OID 61462)
-- Name: ELoggingEvent; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "ELoggingEvent" AS ENUM (
    'LE_UNKNOWN',
    'LE_Death',
    'LE_XP',
    'LE_Level',
    'LE_Mission',
    'LE_Item',
    'LE_Location',
    'LE_Privledge',
    'LE_Pet',
    'LE_StatChange',
    'LE_Cheater',
    'LE_Ability',
    'LE_Interact',
    'LE_Money',
    'LE_MobError',
    'LE_JoinChannel',
    'LE_LeaveChannel',
    'LE_PerfStat',
    'LE_Loginout',
    'LE_Error'
);

