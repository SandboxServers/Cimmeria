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
