--
-- TOC entry 1001 (class 1247 OID 61746)
-- Name: EMobMovementType; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EMobMovementType" AS ENUM (
    'MOB_MOVEMENT_Cover',
    'MOB_MOVEMENT_CombatAdvance',
    'MOB_MOVEMENT_Patrol',
    'MOB_MOVEMENT_Follow',
    'MOB_MOVEMENT_Wander',
    'MOB_MOVEMENT_Leash',
    'MOB_MOVEMENT_Avoid'
);

