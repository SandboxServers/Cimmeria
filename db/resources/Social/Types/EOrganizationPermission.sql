--
-- TOC entry 1016 (class 1247 OID 61814)
-- Name: EOrganizationPermission; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EOrganizationPermission" AS ENUM (
    'EORG_PERM_DoNotUse',
    'EORG_PERM_Invite',
    'EORG_PERM_Promote',
    'EORG_PERM_Demote',
    'EORG_PERM_Eject',
    'EORG_PERM_RosterNotes',
    'EORG_PERM_OfficerNotes',
    'EORG_PERM_RankNames',
    'EORG_PERM_OfficerChat',
    'EORG_PERM_EmailLists',
    'EORG_PERM_MOTD',
    'EORG_PERM_HistoryLog',
    'EORG_PERM_Calendar',
    'EORG_PERM_RecruitDesc',
    'EORG_PERM_Adjectives',
    'EORG_PERM_Insignia',
    'EORG_PERM_DepositBank',
    'EORG_PERM_WithdrawBank',
    'EORG_PERM_DepositCash',
    'EORG_PERM_WithdrawCash',
    'EORG_PERM_ViewBankLogs',
    'EORG_PERM_LeaderChat',
    'EORG_PERM_AllianceChat',
    'EORG_PERM_AlterPerms',
    'EORG_PERM_TransferLeader',
    'EORG_PERM_AllianceCmds'
);

