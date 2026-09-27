--
-- Functions on the public schema. Loaded by db/database.sql after the
-- tables and keys, before the seed data; the triggers that execute them are in
-- _triggers.sql, loaded last.
--

--
-- Function: org_vault_is_empty_sql(org_id)
--
-- TRUE when the organization's vault holds no items and its treasury no
-- cash. The member-delete trigger (org_member_after_delete) calls it when
-- the last member of an organization is deleted: an empty vault disbands the
-- organization, a non-empty one leaves it memberless so a GM can recover the
-- contents (D-ORG20).
--
-- The vault is sgw_organization_vault_items (bank-vault BV-07) and the
-- treasury is sgw_organizations.cash. A missing organization counts as
-- empty (there is nothing to keep). Its Rust twin,
-- cimmeria_base_session::base::organization::api::org_vault_is_empty, which
-- every voluntary disband calls, runs this function rather than repeating
-- the query, so the two cannot disagree. Callers hold the organization row
-- lock, and every vault or cash write takes it first, so the answer holds
-- until they commit.
--

CREATE FUNCTION org_vault_is_empty_sql(p_org_id integer) RETURNS boolean
    LANGUAGE sql STABLE
    AS $$
    SELECT NOT EXISTS (
               SELECT 1 FROM sgw_organization_vault_items WHERE org_id = p_org_id
           )
       AND COALESCE(
               (SELECT cash = 0 FROM sgw_organizations WHERE org_id = p_org_id),
               true
           )
$$;

--
-- Function: org_member_after_delete()
-- Trigger:  sgw_organization_members_after_delete (_triggers.sql)
--
-- Keeps an organization from being left without a leader, whatever deleted
-- the member row: a character delete (the sgw_player cascade), a leave, a
-- kick, a GM tool or test cleanup (D-ORG12, D-ORG20).
--
-- After a member row is deleted:
--
--   1. Lock the organization row (ORG-LOCK, D-ORG04), so the remaining
--      members are counted under the same lock every Rust mutation takes and
--      two last members leaving at once cannot both see the other. If the
--      row is gone the organization itself is being deleted (a disband, or
--      this trigger's own DELETE below cascading back): do nothing.
--   2. If a member still holds rank 8 (Leader), do nothing.
--   3. Otherwise promote the highest-ranked member to Leader, the
--      longest-standing (earliest joined_at, then lowest player_id) among
--      equals.
--   4. If no member remains: when org_vault_is_empty_sql says the vault is
--      empty, delete the organization (its ranks cascade); when it is not,
--      leave a memberless organization that keeps its vault for GM recovery.
--
-- Rules 2 and 3 run for any deleted member, not only the leader, so a
-- leaderless organization, however it arose, is healed on the next delete.
--
-- Every promotion, disband and memberless result also writes a row to
-- sgw_organization_events (reason character_deleted or member_removed), which
-- Rust exports to the `org` log target: tracing cannot see inside Postgres.
--
-- Lock order. Deleting a member row without holding its organization's
-- lock would invert ORG-LOCK: the member row stays locked until commit
-- while this trigger waits for the organization, and a transaction holding
-- the organization and wanting that member row (a kick) deadlocks against
-- it. So every path that deletes member rows holds the organization first:
-- the Rust mutations call lock_org, and a character delete (from any path,
-- an account cascade included) is locked by org_player_before_delete
-- before the cascade reaches the member rows. Here the FOR UPDATE then
-- re-takes a lock the transaction already holds.
--
-- Isolation. The function relies on READ COMMITTED, the server's level:
-- after the FOR UPDATE wait each statement below takes a fresh snapshot and
-- sees what the lock holder committed. Under REPEATABLE READ the FOR UPDATE
-- on a row changed since the snapshot fails with a serialization error.
--
-- AFTER ROW triggers fire when the deleting statement ends, so when one
-- statement deletes several members of the same organization, the first
-- firing already sees them all gone.
--

CREATE FUNCTION org_member_after_delete() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
DECLARE
    v_next_leader integer;
    v_next_account integer;
    v_reason varchar(32);
BEGIN
    PERFORM 1 FROM sgw_organizations WHERE org_id = OLD.org_id FOR UPDATE;
    IF NOT FOUND THEN
        RETURN NULL;
    END IF;

    IF EXISTS (
        SELECT 1 FROM sgw_organization_members
         WHERE org_id = OLD.org_id AND rank = 8
    ) THEN
        RETURN NULL;
    END IF;

    -- The character row goes first in a character delete (the member row
    -- is its cascade), so a missing sgw_player row names the cause.
    IF EXISTS (SELECT 1 FROM sgw_player WHERE player_id = OLD.player_id) THEN
        v_reason := 'member_removed';
    ELSE
        v_reason := 'character_deleted';
    END IF;

    SELECT player_id, account_id INTO v_next_leader, v_next_account
      FROM sgw_organization_members
     WHERE org_id = OLD.org_id
     ORDER BY rank DESC, joined_at ASC, player_id ASC
     LIMIT 1;

    IF FOUND THEN
        UPDATE sgw_organization_members
           SET rank = 8
         WHERE org_id = OLD.org_id AND player_id = v_next_leader;
        INSERT INTO sgw_organization_events
            (org_id, event, reason, from_player_id, from_account_id, to_player_id, to_account_id)
        VALUES
            (OLD.org_id, 'leader_changed', v_reason, OLD.player_id, OLD.account_id,
             v_next_leader, v_next_account);
        RETURN NULL;
    END IF;

    IF org_vault_is_empty_sql(OLD.org_id) THEN
        DELETE FROM sgw_organizations WHERE org_id = OLD.org_id;
        INSERT INTO sgw_organization_events
            (org_id, event, reason, from_player_id, from_account_id)
        VALUES
            (OLD.org_id, 'disbanded', v_reason, OLD.player_id, OLD.account_id);
    ELSE
        INSERT INTO sgw_organization_events
            (org_id, event, reason, from_player_id, from_account_id)
        VALUES
            (OLD.org_id, 'left_memberless', v_reason, OLD.player_id, OLD.account_id);
    END IF;
    RETURN NULL;
END;
$$;

--
-- Function: org_member_before_update()
-- Trigger:  sgw_organization_members_before_update (_triggers.sql)
--
-- A member row's identity is immutable: org_id, player_id, org_type and
-- account_id never change (a move between organizations is a delete and an
-- insert, so the delete trigger sees it). The Leader rank moves only
-- through the member-delete trigger's promotion: a rank change to or from 8
-- is refused unless it runs inside another trigger (pg_trigger_depth() > 1,
-- which the promotion UPDATE inside org_member_after_delete always is). The
-- Rust layer refuses the same changes (OrgStoreError::LeaderPinned); this
-- stops psql, a GM tool or a future code path from making a second leader
-- or a leaderless organization.
--

CREATE FUNCTION org_member_before_update() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    IF NEW.org_id <> OLD.org_id OR NEW.player_id <> OLD.player_id
       OR NEW.org_type <> OLD.org_type OR NEW.account_id <> OLD.account_id THEN
        RAISE EXCEPTION 'organization member identity is immutable (org %, player %)',
            OLD.org_id, OLD.player_id
            USING ERRCODE = 'check_violation',
                  CONSTRAINT = 'sgw_organization_members_identity_immutable';
    END IF;
    IF NEW.rank <> OLD.rank AND (NEW.rank = 8 OR OLD.rank = 8)
       AND pg_trigger_depth() <= 1 THEN
        RAISE EXCEPTION 'the Leader rank moves only by promotion (org %, player %)',
            OLD.org_id, OLD.player_id
            USING ERRCODE = 'check_violation',
                  CONSTRAINT = 'sgw_organization_members_leader_pinned';
    END IF;
    RETURN NEW;
END;
$$;

--
-- Function: org_player_before_delete()
-- Trigger:  sgw_player_before_delete_lock_orgs (_triggers.sql)
--
-- Makes every character delete take the ORG-LOCK order on its own: a
-- DELETE FROM sgw_player from any path (organization::character_delete,
-- an account delete cascading to its characters, psql, a test) locks the
-- character's organization rows, in org_id order, before the cascade
-- deletes its member rows and the member-delete trigger runs.
--
-- A BEFORE DELETE row trigger fires after Postgres has locked the
-- sgw_player row and before any cascade, so the order is: the character's
-- sgw_player row, then its organizations, then (through the cascade) its
-- member rows. Nothing else waits on a character's sgw_player row while
-- holding one of that character's organizations: add_member checks
-- membership before it touches the player row, so it only ever waits on a
-- non-member's row; the kick and rank paths touch member rows, not player
-- rows. See the lock-order note in crates/base-session/src/base/organization/api.rs.
--
-- The membership read runs after the player row is locked, and a new
-- member row needs a KEY SHARE lock on that player row (its foreign key),
-- so no organization can be joined between this read and the delete.
--
-- One statement that deletes several characters (an account delete's
-- cascade) would run this once per character, locking the organizations of
-- the first character before the second character's row. That is not
-- org_id order across the statement, so org_account_before_delete locks
-- all of an account's characters, then all of their organizations, before
-- the cascade starts; here the FOR UPDATE then re-takes held locks.
--

CREATE FUNCTION org_player_before_delete() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    PERFORM 1
       FROM sgw_organizations
      WHERE org_id IN (SELECT org_id FROM sgw_organization_members
                        WHERE player_id = OLD.player_id)
      ORDER BY org_id
        FOR UPDATE;
    RETURN OLD;
END;
$$;

--
-- Function: org_account_before_delete()
-- Trigger:  account_before_delete_lock_orgs (_triggers.sql)
--
-- An account delete cascades to every character of the account in one
-- statement, and org_player_before_delete alone would lock organizations
-- character by character: the first character's organizations, then the
-- second character's row and its organizations. Two characters in two
-- organizations (A in the higher org_id, B in the lower) then take them
-- in descending order, and a concurrent single-character delete of a
-- member of both (ascending) deadlocks against it.
--
-- So an account delete takes the character-delete order for the whole
-- account at once: the account row (Postgres locks it before a BEFORE
-- trigger runs), then every character's sgw_player row in player_id
-- order, then every organization any of them belongs to in org_id order.
-- It holds no organization while it waits on a character row, so it never
-- holds an organization that a single-character delete (which holds its
-- player row and waits on its organizations) needs while waiting on that
-- character. The cascade and org_player_before_delete then re-take held
-- locks.
--

CREATE FUNCTION org_account_before_delete() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    PERFORM 1
       FROM sgw_player
      WHERE account_id = OLD.account_id
      ORDER BY player_id
        FOR UPDATE;
    PERFORM 1
       FROM sgw_organizations
      WHERE org_id IN (SELECT m.org_id
                         FROM sgw_organization_members m
                         JOIN sgw_player p ON p.player_id = m.player_id
                        WHERE p.account_id = OLD.account_id)
      ORDER BY org_id
        FOR UPDATE;
    RETURN OLD;
END;
$$;

--
-- Function: bm_player_before_delete()
-- Trigger:  sgw_player_before_delete_auctions (_triggers.sql)
--
-- Keeps the Black Market consistent when a character is deleted, from any
-- path (decision D-BM09 in docs/analysis/black-market/README.md):
--
-- - Open auctions the character is selling go with it: sgw_auction.seller_id
--   is ON DELETE CASCADE, and the listed item (a container-18 row of the
--   seller's inventory) cascades with the inventory. Their standing bidders
--   are refunded here first, so nobody else loses held cash.
-- - Open auctions the character is the standing bidder on reopen: the bid
--   is cleared here (the held cash leaves with the character, like the rest
--   of its naquadah), and current_bidder is ON DELETE SET NULL for settled
--   rows, which keep their history.
--
-- Lock order: the sgw_player row being deleted (locked before a BEFORE
-- trigger runs), then the affected auction rows in sequence_id order, then
-- the refunded bidders' sgw_player rows. A bid in flight on one of these
-- auctions holds the auction row and waits on bidder rows, which is the
-- same order; a buyout settling to this seller can deadlock with it, which
-- Postgres detects and rolls back (the player retries).
--
-- A refund is capped at the integer maximum rather than overflowing, which
-- would abort the delete.
--

CREATE FUNCTION bm_player_before_delete() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    PERFORM 1
       FROM sgw_auction
      WHERE status = 0
        AND (seller_id = OLD.player_id OR current_bidder = OLD.player_id)
      ORDER BY sequence_id
        FOR UPDATE;
    UPDATE sgw_auction
       SET current_bid = 0, current_bidder = NULL
     WHERE status = 0 AND current_bidder = OLD.player_id;
    UPDATE sgw_player p
       SET naquadah = LEAST(p.naquadah::bigint + r.total, 2147483647)::integer
      FROM (SELECT current_bidder AS player_id, SUM(current_bid)::bigint AS total
              FROM sgw_auction
             WHERE status = 0 AND seller_id = OLD.player_id
               AND current_bidder IS NOT NULL AND current_bid > 0
             GROUP BY current_bidder) r
     WHERE p.player_id = r.player_id;
    RETURN OLD;
END;
$$;
