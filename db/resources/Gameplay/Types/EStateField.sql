--
-- TOC entry 1088 (class 1247 OID 62230)
-- Name: EStateField; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EStateField" AS ENUM (
    'BSF_Dead',
    'BSF_AutoCycling',
    'BSF_Crouching',
    'BSF_InCombat',
    'BSF_PlayingMinigame',
    'BSF_InStealth',
    'BSF_MovementLock',
    'BSF_Walking',
    'BSF_Holster'
);

