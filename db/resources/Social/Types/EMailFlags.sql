--
-- TOC entry 965 (class 1247 OID 61538)
-- Name: EMailFlags; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EMailFlags" AS ENUM (
    'MAIL_Archive',
    'MAIL_COD',
    'MAIL_ToVault',
    'MAIL_ToTeam',
    'MAIL_ToCommand',
    'MAIL_ToCommandOfficers',
    'MAIL_ToCommandRank0',
    'MAIL_ToCommandRank1',
    'MAIL_ToCommandRank2',
    'MAIL_ToCommandRank3',
    'MAIL_ToCommandRank4',
    'MAIL_ToCommandRank5',
    'MAIL_ToCommandRank6',
    'MAIL_ToCommandRank7'
);

