--
-- TOC entry 1106 (class 1247 OID 62498)
-- Name: ETextType; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "ETextType" AS ENUM (
    'TEXT_TYPE_AbilityDisplayName',
    'TEXT_TYPE_Tooltip',
    'TEXT_TYPE_ItemDisplayName',
    'TEXT_TYPE_ItemDescription',
    'TEXT_TYPE_ItemTooltip',
    'TEXT_TYPE_MissionDisplayName',
    'TEXT_TYPE_MissionObjectiveLogText',
    'TEXT_TYPE_MissionStepLogText',
    'TEXT_TYPE_MissionLabel',
    'TEXT_TYPE_DiscoveryText',
    'TEXT_TYPE_LevelUp',
    'TEXT_TYPE_AbilityDescription',
    'TEXT_TYPE_GateMailSender',
    'TEXT_TYPE_GateMailRecipient',
    'TEXT_TYPE_GateMailSubject',
    'TEXT_TYPE_GateMailBody',
    'TEXT_TYPE_RegionDisplayName',
    'TEXT_TYPE_MobDisplayName',
    'TEXT_TYPE_MinigameContactName',
    'TEXT_TYPE_MinigameContactTitle',
    'TEXT_TYPE_MinigameRequestNPCName',
    'TEXT_TYPE_MinigameRequestNPCTitle',
    'TEXT_TYPE_MinigameExternalName',
    'TEXT_TYPE_MissionHistoryText',
    'TEXT_TYPE_SystemText',
    'TEXT_TYPE_MAX'
);

