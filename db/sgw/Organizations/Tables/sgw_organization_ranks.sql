--
-- Per-rank configuration for a persistent organization.
--
-- Each (org_id, rank_value) pair stores an optional custom display name
-- and the 26-bit EORG_PERM_* permission mask for that rank.
--
-- rank_value is the wire UINT8 (0–8), matching EORG_RANK_* exactly:
--   0=None, 1=Initiate, 2=Member, 3=SeniorMember, 4=Veteran,
--   5=SeniorVeteran, 6=Officer, 7=SeniorOfficer, 8=Leader.
--
-- permission_mask stores the OrgPermission bitmask as a signed integer.
-- All 26 EORG_PERM_* bits combined = 0x3FFFFFF ≈ 67M, which fits in a
-- positive i32.  The application layer treats this as an unsigned 26-bit
-- value; the column type is integer for cross-ORM compatibility.
--
-- Rows are inserted for all 9 rank values when an org is created.
-- custom_name NULL means the client shows the default rank name.
--

CREATE TABLE sgw_organization_ranks (
    org_id integer NOT NULL,
    rank_value smallint NOT NULL,
    custom_name character varying(64),
    permission_mask integer NOT NULL DEFAULT 0,
    CONSTRAINT sgw_organization_ranks_rank_value_check
        CHECK (rank_value >= 0 AND rank_value <= 8),
    -- OrgPermission is a 26-bit mask (0x3FFFFFF = 67108863). Reject negative or
    -- out-of-range values so a buggy/hostile path can't persist e.g. 0xFFFFFFFF.
    CONSTRAINT sgw_organization_ranks_permission_mask_check
        CHECK (permission_mask >= 0 AND permission_mask <= 67108863)
);
