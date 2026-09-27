--
-- What the member-delete trigger did, for telemetry.
--
-- The trigger (org_member_after_delete, _functions.sql) runs inside
-- Postgres, where tracing cannot see it. Each promotion, disband or
-- memberless result it produces is written here, and Rust logs the row to
-- the `org` log target and then stamps exported_at:
--
--   - organization::character_delete::delete_character exports the rows its
--     own transaction produced right after it commits (INFO);
--   - persistence::remove_member exports its transaction's rows inside that
--     transaction (DEBUG, since the caller may still roll back);
--   - a startup sweep (organization::audit) exports anything left
--     unstamped, such as a bare DELETE from psql or a test (INFO).
--
-- Delivery is at least once: a crash after the log and before the stamp
-- commits re-sends the row at the next startup. Every exported event
-- carries org_event_id so queries can drop duplicates.
--
-- event: leader_changed | disbanded | left_memberless.
-- reason: character_deleted (the sgw_player row was gone when the trigger
-- ran) | member_removed (any other member delete).
-- from_*: the member whose row was deleted. to_*: the member promoted to
-- Leader, NULL unless event = leader_changed. The identities are copied
-- from the member rows at delete time, so they survive the character.
-- tx_id: the transaction that wrote the row (txid_current()), which is how
-- the exporters find their own rows.
--
-- No foreign keys: a disbanded organization and a deleted character are
-- exactly what these rows describe.
--

CREATE TABLE sgw_organization_events (
    org_event_id bigint NOT NULL DEFAULT nextval('sgw_organization_events_org_event_id_seq'),
    org_id integer NOT NULL,
    event character varying(32) NOT NULL,
    reason character varying(32) NOT NULL,
    from_player_id integer NOT NULL,
    from_account_id integer NOT NULL,
    to_player_id integer,
    to_account_id integer,
    tx_id bigint NOT NULL DEFAULT txid_current(),
    at timestamp with time zone NOT NULL DEFAULT now(),
    exported_at timestamp with time zone,
    CONSTRAINT sgw_organization_events_event_check
        CHECK (event IN ('leader_changed', 'disbanded', 'left_memberless')),
    CONSTRAINT sgw_organization_events_reason_check
        CHECK (reason IN ('character_deleted', 'member_removed')),
    CONSTRAINT sgw_organization_events_to_check
        CHECK ((event = 'leader_changed') = (to_player_id IS NOT NULL AND to_account_id IS NOT NULL))
);
