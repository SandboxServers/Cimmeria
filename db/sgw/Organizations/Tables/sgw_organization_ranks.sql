--
-- The ranks of one persistent organization: a display name and a
-- permission mask per rank.
--
-- One row per rank the organization's type uses (D-ORG07): Team ranks 2, 3
-- and 8; Command ranks 1 to 8. Rank 0 (EORG_RANK_None) is never a member's
-- rank, so no type has a row for it. The database enforces the per-type set:
-- org_type is a copy of the organization's type, pinned to it by the
-- composite foreign key (org_id, org_type), and
-- sgw_organization_ranks_rank_in_type_check allows only that type's ranks.
-- Members point at these rows, so a member can hold only a rank the type
-- uses. create_org writes the rows from
-- cimmeria_entity::organization::default_rank_permissions (D-ORG08 as
-- amended by D-ORG21).
--
-- name NULL means "show the client's default name for this rank".
--
-- permissions is the 26-bit EOrganizationPermission mask (ALL = 0x3FF_FFFF =
-- 67108863). The range CHECK refuses a negative or out-of-range mask, so a
-- bad path cannot store 0xFFFFFFFF. The Leader row (rank 8) is pinned to
-- every bit (D-ORG08): no editor, GM command included, may change it.
--

CREATE TABLE sgw_organization_ranks (
    org_id integer NOT NULL,
    org_type smallint NOT NULL,
    rank smallint NOT NULL,
    name character varying(32),
    permissions integer NOT NULL,
    CONSTRAINT sgw_organization_ranks_rank_in_type_check
        CHECK ((org_type = 1 AND rank IN (2, 3, 8))
            OR (org_type = 2 AND rank BETWEEN 1 AND 8)),
    CONSTRAINT sgw_organization_ranks_permissions_check
        CHECK (permissions BETWEEN 0 AND 67108863),
    CONSTRAINT sgw_organization_ranks_leader_all_check
        CHECK (rank <> 8 OR permissions = 67108863)
);
