--
-- TOC entry 968 (class 1247 OID 61568)
-- Name: EMailResultCodes; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EMailResultCodes" AS ENUM (
    'MAILRESULT_Sent',
    'MAILRESULT_NoRecipients',
    'MAILRESULT_ItemNotAvailable',
    'MAILRESULT_AttachmentsAndMultipleRecipients',
    'MAILRESULT_NotEnoughCash',
    'MAILRESULT_VaultButNoItem',
    'MAILRESULT_VaultPlusCash',
    'MAILRESULT_SentToVault'
);

