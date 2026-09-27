--
-- Sequence: sgw_organizations_org_id_seq
-- Drives sgw_organizations.org_id (owned via _sequence_ownership.sql).
--
-- MAXVALUE is the top of the Team/Command id range (D-ORG05). Ids from
-- 0x4000_0000 up belong to the cell's squad counter, so the sequence must
-- stop below it: NO CYCLE makes an exhausted sequence an error instead of a
-- reused id.
--

CREATE SEQUENCE sgw_organizations_org_id_seq
    START WITH 1
    INCREMENT BY 1
    MINVALUE 1
    MAXVALUE 1073741823
    NO CYCLE
    CACHE 1;
