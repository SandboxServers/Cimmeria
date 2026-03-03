--
-- TOC entry 893 (class 1247 OID 61002)
-- Name: EEntityFlags; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EEntityFlags" AS ENUM (
    'ENTITYFLAG_None',
    'ENTITYFLAG_HideName',
    'ENTITYFLAG_NoTarget',
    'ENTITYFLAG_DoNotDrop',
    'ENTITYFLAG_NoPetLeveling',
    'ENTITYFLAG_NoPetTargeting',
    'ENTITYFLAG_DespawnOnOwnerLeash',
    'ENTITYFLAG_NoPassive',
    'ENTITYFLAG_NoDefensive',
    'ENTITYFLAG_NoAggressive',
    'ENTITYFLAG_DetectionPet',
    'ENTITYFLAG_Pet',
    'ENTITYFLAG_Craft_Craft',
    'ENTITYFLAG_Craft_Research',
    'ENTITYFLAG_Craft_RevEng',
    'ENTITYFLAG_Craft_Alloying',
    'ENTITYFLAG_DespawnOnLeashFromOwner',
    'ENTITYFLAG_PetUseOwnFaction',
    'ENTITYFLAG_PetWaitToDespawn',
    'ENTITYFLAG_DoNotLeaveDefaultState'
);

