--
-- TOC entry 833 (class 1247 OID 60212)
-- Name: ECombatantState; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "ECombatantState" AS ENUM (
    'PLAYER_STATE_Blind',
    'PLAYER_STATE_Confuse',
    'PLAYER_STATE_DoT',
    'PLAYER_STATE_Disease',
    'PLAYER_STATE_Disorient',
    'PLAYER_STATE_Fear',
    'PLAYER_STATE_KnockBack',
    'PLAYER_STATE_KnockDown',
    'PLAYER_STATE_Slow',
    'PLAYER_STATE_Snare',
    'PLAYER_STATE_Stun',
    'PLAYER_STATE_Suppression',
    'PLAYER_STATE_Alive',
    'PLAYER_STATE_Dead'
);

