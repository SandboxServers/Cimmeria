--
-- Persistent organizations owned by player characters.
--
-- Only Teams (org_type=1) and Commands (org_type=2) are stored here.
-- Squads are ephemeral/session-only and are never persisted — see
-- EPersistentOrganizationType which lists only POT_Team and POT_Command.
--
-- org_type CHECK mirrors EPersistentOrganizationType: 1=Team, 2=Command.
-- The CHECK excludes 0 (Squad) intentionally.
--
-- leader_player_id references the current Leader member. On leadership
-- transfer the column is updated in the same transaction that updates
-- the member's rank row.
--
-- cash stores the organisation treasury in the wire's UINT64 denomination.
-- bigint (signed 64-bit) covers the full positive range a player can
-- accumulate; negative values are impossible in normal play and rejected
-- by the application layer.
--

CREATE TABLE sgw_organizations (
    org_id integer NOT NULL DEFAULT nextval('sgw_organizations_org_id_seq'),
    org_type smallint NOT NULL,
    name character varying(128) NOT NULL,
    leader_player_id integer NOT NULL,
    motd text,
    cash bigint NOT NULL DEFAULT 0,
    created_at timestamp with time zone NOT NULL DEFAULT now(),
    CONSTRAINT sgw_organizations_org_type_check
        CHECK (org_type IN (1, 2)),
    -- Treasury is never negative; guard against bugs / manual edits.
    CONSTRAINT sgw_organizations_cash_nonneg_check
        CHECK (cash >= 0)
);
