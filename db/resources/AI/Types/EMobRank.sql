--
-- TOC entry 1004 (class 1247 OID 61762)
-- Name: EMobRank; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EMobRank" AS ENUM (
    'MOBRANK_Animal',
    'MOBRANK_Standard',
    'MOBRANK_EliteAnimal',
    'MOBRANK_StandardElite',
    'MOBRANK_NamedRare',
    'MOBRANK_Boss'
);

