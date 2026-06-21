--
-- TOC entry 2687 (class 2606 OID 63876)
-- Name: missions_player_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_mission
    ADD CONSTRAINT missions_player_id_fkey FOREIGN KEY (player_id) REFERENCES sgw_player(player_id) ON UPDATE RESTRICT ON DELETE CASCADE;

--
-- TOC entry 2683 (class 2606 OID 63881)
-- Name: sgw_gate_mail_character_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_gate_mail
    ADD CONSTRAINT sgw_gate_mail_character_id_fkey FOREIGN KEY (character_id) REFERENCES sgw_player(player_id) ON UPDATE RESTRICT ON DELETE CASCADE;

--
-- TOC entry 2684 (class 2606 OID 63886)
-- Name: sgw_gate_mail_sender_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_gate_mail
    ADD CONSTRAINT sgw_gate_mail_sender_id_fkey FOREIGN KEY (sender_id) REFERENCES sgw_player(player_id) ON UPDATE RESTRICT ON DELETE SET NULL;

--
-- TOC entry 2685 (class 2606 OID 63891)
-- Name: sgw_inventory_character_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_inventory
    ADD CONSTRAINT sgw_inventory_character_id_fkey FOREIGN KEY (character_id) REFERENCES sgw_player(player_id) ON UPDATE RESTRICT ON DELETE CASCADE;

--
-- TOC entry 2686 (class 2606 OID 63896)
-- Name: sgw_inventory_type_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_inventory
    ADD CONSTRAINT sgw_inventory_type_id_fkey FOREIGN KEY (type_id) REFERENCES resources.items(item_id) ON UPDATE CASCADE ON DELETE RESTRICT;

--
-- TOC entry 2680 (class 2606 OID 63901)
-- Name: sgw_player_account_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_player
    ADD CONSTRAINT sgw_player_account_id_fkey FOREIGN KEY (account_id) REFERENCES account(account_id) ON UPDATE RESTRICT ON DELETE CASCADE;

--
-- TOC entry 2681 (class 2606 OID 63916)
-- Name: sgw_player_world_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_player
    ADD CONSTRAINT sgw_player_world_id_fkey FOREIGN KEY (world_id) REFERENCES resources.worlds(world_id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- TOC entry 2682 (class 2606 OID 63921)
-- Name: sgw_player_world_location_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_player
    ADD CONSTRAINT sgw_player_world_location_fkey FOREIGN KEY (world_location) REFERENCES resources.worlds(world) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- Name: sgw_player_discipline_expertise_player_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--
-- Per-(player, discipline) crafting expertise table; rows are removed when
-- the parent player is deleted. Declared here (not inline in the CREATE TABLE)
-- because sgw_player's PK constraint isn't established until _primary_keys.sql
-- runs.
--

ALTER TABLE ONLY sgw_player_discipline_expertise
    ADD CONSTRAINT sgw_player_discipline_expertise_player_id_fkey FOREIGN KEY (player_id) REFERENCES sgw_player(player_id) ON UPDATE RESTRICT ON DELETE CASCADE;

--
-- Name: sgw_contact_list_player_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--
-- ON DELETE CASCADE ensures all lists (and via FK below, all members) are
-- removed when the owning character is deleted — invariant #4 (no orphaned
-- social data after character deletion).
--

ALTER TABLE ONLY sgw_contact_list
    ADD CONSTRAINT sgw_contact_list_player_id_fkey FOREIGN KEY (player_id) REFERENCES sgw_player(player_id) ON UPDATE RESTRICT ON DELETE CASCADE;

--
-- Name: sgw_contact_list_member_list_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_contact_list_member
    ADD CONSTRAINT sgw_contact_list_member_list_id_fkey FOREIGN KEY (list_id) REFERENCES sgw_contact_list(list_id) ON UPDATE RESTRICT ON DELETE CASCADE;

--
-- Name: sgw_organizations_leader_player_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--
-- Leader column tracks current leader. ON DELETE RESTRICT prevents deleting a
-- character who is still a guild leader — the application must transfer or
-- disband first.
--

ALTER TABLE ONLY sgw_organizations
    ADD CONSTRAINT sgw_organizations_leader_player_id_fkey FOREIGN KEY (leader_player_id) REFERENCES sgw_player(player_id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- Name: sgw_organization_ranks_org_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--
-- Rank rows cascade-delete when the organization is deleted.
--

ALTER TABLE ONLY sgw_organization_ranks
    ADD CONSTRAINT sgw_organization_ranks_org_id_fkey FOREIGN KEY (org_id) REFERENCES sgw_organizations(org_id) ON UPDATE RESTRICT ON DELETE CASCADE;

--
-- Name: sgw_organization_members_org_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_organization_members
    ADD CONSTRAINT sgw_organization_members_org_id_fkey FOREIGN KEY (org_id) REFERENCES sgw_organizations(org_id) ON UPDATE RESTRICT ON DELETE CASCADE;

--
-- Name: sgw_organization_members_player_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--
-- ON DELETE CASCADE removes the member row when the owning character is deleted
-- (invariant: no orphaned roster rows after character deletion).
--

ALTER TABLE ONLY sgw_organization_members
    ADD CONSTRAINT sgw_organization_members_player_id_fkey FOREIGN KEY (player_id) REFERENCES sgw_player(player_id) ON UPDATE RESTRICT ON DELETE CASCADE;

