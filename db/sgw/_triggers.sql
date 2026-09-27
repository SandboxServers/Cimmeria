--
-- Triggers on the public schema. Loaded last by db/database.sql, after the
-- tables, keys and the functions they execute.
--

--
-- Name: sgw_organization_members_after_delete; Type: TRIGGER; Schema: public; Owner: -
--
-- Promotes a new leader, or disbands the organization, when a member row
-- goes (D-ORG12, D-ORG20). See org_member_after_delete() in _functions.sql.
--

CREATE TRIGGER sgw_organization_members_after_delete AFTER DELETE ON sgw_organization_members FOR EACH ROW EXECUTE FUNCTION org_member_after_delete();

--
-- Name: sgw_organization_members_before_update; Type: TRIGGER; Schema: public; Owner: -
--
-- Member identity is immutable and the Leader rank moves only by the
-- delete trigger's promotion. See org_member_before_update() in _functions.sql.
--

CREATE TRIGGER sgw_organization_members_before_update BEFORE UPDATE ON sgw_organization_members FOR EACH ROW EXECUTE FUNCTION org_member_before_update();

--
-- Name: sgw_player_before_delete_lock_orgs; Type: TRIGGER; Schema: public; Owner: -
--
-- Locks a deleted character's organizations before the member-row cascade,
-- so every character delete keeps the lock order. See
-- org_player_before_delete() in _functions.sql.
--

CREATE TRIGGER sgw_player_before_delete_lock_orgs BEFORE DELETE ON sgw_player FOR EACH ROW EXECUTE FUNCTION org_player_before_delete();
