--
-- Persistent organizations: Teams (org_type 1) and Commands (org_type 2).
--
-- Squads (org_type 0) are cell state and never persisted (D-ORG03), so the
-- org_type CHECK excludes 0. Campaign ledger:
-- docs/analysis/organizations/work-packets.md, "Schema".
--
-- org_id shares one wire id space with squads (D-ORG05): Team and Command
-- ids are 1..0x3FFF_FFFF, squad ids start at 0x4000_0000. The CHECK keeps a
-- hand-inserted id out of the squad range too.
--
-- There is no leader column. The leader is the member whose rank is 8
-- (EORG_RANK_Leader); a second copy would have to be kept in step with the
-- member rows (PR #584's leader_player_id, audit A-30). The AFTER DELETE
-- trigger on sgw_organization_members keeps an organization from losing
-- its leader (D-ORG12, D-ORG20).
--
-- name is the display name, already normalised by
-- cimmeria_entity::organization::org_text (D-ORG10). name_key is its
-- case-folded form, unique per type: "Tau'ri" and "TAU'RI" collide.
--
-- cash is the treasury, in the wire's UINT64 unit. The Bank / Vault campaign
-- owns every change to it; the CHECK is the last line against a bad debit.
-- experience is always 0 today: nothing says how an organization earns it.
--
-- vault_slots is the Team vault's size (container 19): 40 at creation,
-- grown in +10 steps to 100 by BV-09 and never shrunk (D-BV14). A Command's
-- vault (container 20) is fixed at 100 and does not read the column. The
-- vault's items are sgw_organization_vault_items (bank-vault BV-07).
--
-- The UNIQUE (org_type, name_key) and UNIQUE (org_id, org_type) constraints
-- are in _primary_keys.sql. The second is the target of the members'
-- composite foreign key, which stops a member row's org_type drifting from
-- its organization's.
--

CREATE TABLE sgw_organizations (
    org_id integer NOT NULL DEFAULT nextval('sgw_organizations_org_id_seq'),
    org_type smallint NOT NULL,
    name character varying(60) NOT NULL,
    name_key character varying(60) NOT NULL,
    motd character varying(255) NOT NULL DEFAULT '',
    cash bigint NOT NULL DEFAULT 0,
    experience bigint NOT NULL DEFAULT 0,
    vault_slots smallint NOT NULL DEFAULT 40,
    created_at timestamp with time zone NOT NULL DEFAULT now(),
    CONSTRAINT sgw_organizations_org_id_range_check
        CHECK (org_id BETWEEN 1 AND 1073741823),
    CONSTRAINT sgw_organizations_org_type_check
        CHECK (org_type IN (1, 2)),
    CONSTRAINT sgw_organizations_name_nonempty_check
        CHECK (char_length(name) > 0 AND char_length(name_key) > 0),
    CONSTRAINT sgw_organizations_cash_nonneg_check
        CHECK (cash >= 0),
    CONSTRAINT sgw_organizations_vault_slots_check
        CHECK (vault_slots BETWEEN 40 AND 100 AND vault_slots % 10 = 0)
);
