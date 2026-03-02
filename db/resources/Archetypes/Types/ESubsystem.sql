--
-- TOC entry 1094 (class 1247 OID 62416)
-- Name: ESubsystem; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "ESubsystem" AS ENUM (
    'SUBSYSTEM_Unknown',
    'SUBSYSTEM_Minigame',
    'SUBSYSTEM_Mission',
    'SUBSYSTEM_Inventory',
    'SUBSYSTEM_PlayerAbility',
    'SUBSYSTEM_MobAbility',
    'SUBSYSTEM_Dialog',
    'SUBSYSTEM_PlayerPassiveAbility',
    'SUBSYSTEM_GateTravel',
    'SUBSYSTEM_PendingAbility'
);

