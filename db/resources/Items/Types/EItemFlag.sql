--
-- TOC entry 941 (class 1247 OID 61386)
-- Name: EItemFlag; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EItemFlag" AS ENUM (
    'ITEM_FLAG_MinigameInstrument',
    'ITEM_FLAG_MinigameConsumable',
    'ITEM_FLAG_BindOnAcquire',
    'ITEM_FLAG_BindOnEquip',
    'ITEM_FLAG_NotResearchable',
    'ITEM_FLAG_Kicker',
    'ITEM_FLAG_Craft_Craft',
    'ITEM_FLAG_Craft_Research',
    'ITEM_FLAG_Craft_RevEng',
    'ITEM_FLAG_Craft_Alloying',
    'ITEM_FLAG_CanBeSold',
    'ITEM_FLAG_CanBeDeleted',
    'ITEM_FLAG_Unique',
    'ITEM_FLAG_MustEquipToUse',
    'ITEM_FLAG_DestroyOnClear',
    'ITEM_FLAG_ElementaryComponent'
);

