--
-- Members of a persistent organization.
--
-- Members are anchored to player_id (FK to sgw_player, ON DELETE CASCADE),
-- so a roster row survives a rename and goes with its character.
--
-- org_type is a copy of the organization's type. It exists for
-- UNIQUE (player_id, org_type), which enforces D-ORG18 (one Team and one
-- Command per player) in the database. The composite foreign key
-- (org_id, org_type) -> sgw_organizations (org_id, org_type) stops the copy
-- drifting from its organization.
--
-- rank must be one of the organization's rank rows: the composite foreign
-- key (org_id, rank) -> sgw_organization_ranks refuses rank 0, and Team
-- rank 5, because no such row exists. The leader is the member at rank 8.
--
-- joined_at orders "longest-standing" when the AFTER DELETE trigger picks a
-- new leader (D-ORG12); player_id breaks a tie.
--
-- note is the member's own roster note; officer_note is written by officers
-- (D-ORG10 caps: 128 UTF-16 units each).
--

CREATE TABLE sgw_organization_members (
    org_id integer NOT NULL,
    player_id integer NOT NULL,
    org_type smallint NOT NULL,
    rank smallint NOT NULL,
    note character varying(128) NOT NULL DEFAULT '',
    officer_note character varying(128) NOT NULL DEFAULT '',
    joined_at timestamp with time zone NOT NULL DEFAULT now()
);
