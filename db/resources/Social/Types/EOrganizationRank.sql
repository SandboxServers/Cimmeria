--
-- TOC entry 1019 (class 1247 OID 61868)
-- Name: EOrganizationRank; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EOrganizationRank" AS ENUM (
    'EORG_RANK_None',
    'EORG_RANK_Initiate',
    'EORG_RANK_Member',
    'EORG_RANK_SeniorMember',
    'EORG_RANK_Veteran',
    'EORG_RANK_SeniorVeteran',
    'EORG_RANK_Officer',
    'EORG_RANK_SeniorOfficer',
    'EORG_RANK_Leader'
);

