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
-- STUB. The organizations campaign ships it returning TRUE; the Bank / Vault
-- campaign replaces it (CREATE OR REPLACE, same signature) when the Team and
-- Command vaults land. Its Rust twin is
-- cimmeria_base_session::base::organization::api::org_vault_is_empty, which
-- every voluntary disband calls; the two must agree.
--

CREATE FUNCTION org_vault_is_empty_sql(p_org_id integer) RETURNS boolean
    LANGUAGE sql STABLE
    AS $$
    SELECT true
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
-- Lock order (D-ORG04: organization row, then sgw_player, then items).
-- Deleting a member row without holding its organization's lock inverts
-- that order: the member row stays locked until commit while this trigger
-- waits for the organization, and a transaction that holds the
-- organization and wants that member row (a kick, or this same trigger
-- promoting that member) deadlocks against it. So every path that deletes
-- member rows locks the organizations first: the Rust mutations call
-- lock_org, and a character delete goes through
-- organization::character_delete::delete_character, which locks the
-- character's organizations in org_id order before it deletes the
-- sgw_player row. Here the FOR UPDATE then re-takes a lock the
-- transaction already holds. A bare DELETE FROM sgw_player still works; it
-- just runs the deadlock risk.
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

    SELECT player_id INTO v_next_leader
      FROM sgw_organization_members
     WHERE org_id = OLD.org_id
     ORDER BY rank DESC, joined_at ASC, player_id ASC
     LIMIT 1;

    IF FOUND THEN
        UPDATE sgw_organization_members
           SET rank = 8
         WHERE org_id = OLD.org_id AND player_id = v_next_leader;
        RETURN NULL;
    END IF;

    IF org_vault_is_empty_sql(OLD.org_id) THEN
        DELETE FROM sgw_organizations WHERE org_id = OLD.org_id;
    END IF;
    RETURN NULL;
END;
$$;
