--
-- TOC entry 779 (class 1247 OID 59960)
-- Name: EAbilityFlags; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EAbilityFlags" AS ENUM (
    'AbilityNone',
    'WeaponBar',
    'DeploymentBar',
    'UseWeaponRange',
    'Toggled',
    'Response',
    'PetToggled',
    'PetTrained',
    'UsableWithDisguise',
    'ForceStanding',
    'DoNotActivate_AutoCycle',
    'Deactivate_AutoCycle',
    'SpeedGrenade',
    'SpeedDeploy',
    'SpeedAttack',
    'SpeedPet',
    'ForceTargetGround',
    'PetCommand'
);

