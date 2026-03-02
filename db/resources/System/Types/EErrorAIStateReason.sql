--
-- TOC entry 899 (class 1247 OID 61064)
-- Name: EErrorAIStateReason; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EErrorAIStateReason" AS ENUM (
    'EAI_ERROR_STATE_None',
    'EAI_ERROR_STATE_StateChanges',
    'EAI_ERROR_STATE_NavFailures',
    'EAI_ERROR_STATE_PatrolFailure',
    'EAI_ERROR_STATE_BehaviorEvent',
    'EAI_ERROR_STATE_BehaviorNACS',
    'EAI_ERROR_STATE_AbilitySet',
    'EAI_ERROR_STATE_GMSlashCommand',
    'EAI_ERROR_STATE_PetWithoutOwner'
);

