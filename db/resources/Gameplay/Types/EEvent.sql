--
-- TOC entry 905 (class 1247 OID 61088)
-- Name: EEvent; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EEvent" AS ENUM (
    'EVENT_ItemPickup',
    'EVENT_ItemEquip',
    'EVENT_ItemUnequip',
    'EVENT_ItemDurabilityLoss',
    'EVENT_ItemUse',
    'EVENT_ItemMelee',
    'EVENT_ItemRanged'
);

