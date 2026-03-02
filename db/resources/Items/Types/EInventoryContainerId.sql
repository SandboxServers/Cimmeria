--
-- TOC entry 938 (class 1247 OID 61342)
-- Name: EInventoryContainerId; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EInventoryContainerId" AS ENUM (
    'INV_Main',
    'INV_Mission',
    'INV_Bandolier',
    'INV_Head',
    'INV_Face',
    'INV_Neck',
    'INV_Chest',
    'INV_Hands',
    'INV_Waist',
    'INV_Back',
    'INV_Legs',
    'INV_Feet',
    'INV_Artifact1',
    'INV_Artifact2',
    'INV_Crafting',
    'INV_Buyback',
    'INV_Bank',
    'INV_Auction',
    'INV_TeamBank',
    'INV_CommandBank',
    'INV_Incoming'
);

