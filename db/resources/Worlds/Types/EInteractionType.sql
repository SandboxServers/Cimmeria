--
-- TOC entry 935 (class 1247 OID 61316)
-- Name: EInteractionType; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EInteractionType" AS ENUM (
    'ItemVendor',
    'AbilityTrainer',
    'Conversation',
    'InitialInteraction',
    'Loot',
    'MissionRewardList',
    'RingTransporter',
    'Banker',
    'OrganizationCreation',
    'Trade',
    'TeamBanker',
    'CommandBanker'
);

