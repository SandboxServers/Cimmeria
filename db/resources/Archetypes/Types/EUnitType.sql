--
-- TOC entry 1124 (class 1247 OID 62624)
-- Name: EUnitType; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EUnitType" AS ENUM (
    'Undefined',
    'AbilityUser',
    'AbilityTarget',
    'MissionGiver',
    'MissionAccepter',
    'EventTriggerer',
    'BehaviorMob',
    'DialogGiver',
    'DialogReceiver',
    'AbilitySecondaryTarget',
    'BehaviorEventInvoker',
    'PetOwner',
    'MinigamePlayer',
    'MinigameGiver',
    'SpawnRegion',
    'PetOwnerTarget',
    'BehaviorEventMobTarget'
);

