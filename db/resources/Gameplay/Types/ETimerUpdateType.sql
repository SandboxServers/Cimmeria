--
-- TOC entry 1115 (class 1247 OID 62572)
-- Name: ETimerUpdateType; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "ETimerUpdateType" AS ENUM (
    'AbilityWarmup',
    'AbilityCooldown',
    'ItemCooldown',
    'DurationEffect',
    'DialogTimer',
    'CategoryCooldown',
    'MissionTimer',
    'StepTimer',
    'ObjectiveTimer',
    'ReloadTimer',
    'ReloadDeploymentTimer',
    'DuelTimer',
    'PvPTimer',
    'CraftInductionTimer'
);

