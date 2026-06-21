--
-- Members of a persistent organization.
--
-- Member identity uses player_id (FK → sgw_player) consistent with the
-- contact-list convention: the wire sends player names for lookups, but
-- the DB anchors members to player_id so that roster rows survive name
-- changes and are cleaned up on character deletion via CASCADE.
--
-- rank_value mirrors the OrgRank wire UINT8 (0–8); the CHECK constrains
-- it to the valid range.
--
-- notes is the member's player-editable public roster note; officer_note
-- is the officer-only annotation visible only to rank >= Officer.
--
-- joined_at is set server-side on invite acceptance.
--

CREATE TABLE sgw_organization_members (
    org_id integer NOT NULL,
    player_id integer NOT NULL,
    rank_value smallint NOT NULL,
    notes text,
    officer_note text,
    joined_at timestamp with time zone NOT NULL DEFAULT now(),
    CONSTRAINT sgw_organization_members_rank_value_check
        CHECK (rank_value >= 0 AND rank_value <= 8)
);
