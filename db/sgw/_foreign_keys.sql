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
-- Social-systems SS-M2: the escrowed item goes with its mail. CASCADE so a
-- character deletion (which cascades the mail) is not blocked; the player's
-- own deleteMailMessage refuses attached mail in the application.
--

ALTER TABLE ONLY sgw_gate_mail_item
    ADD CONSTRAINT sgw_gate_mail_item_mail_id_fkey FOREIGN KEY (mail_id) REFERENCES sgw_gate_mail(mail_id) ON UPDATE RESTRICT ON DELETE CASCADE;

ALTER TABLE ONLY sgw_gate_mail_item
    ADD CONSTRAINT sgw_gate_mail_item_type_id_fkey FOREIGN KEY (type_id) REFERENCES resources.items(item_id) ON UPDATE CASCADE ON DELETE RESTRICT;

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
-- Name: sgw_player_content_cooldown_player_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--
-- Content-action cooldowns (SS-U3); a deleted character takes its rows with
-- it.
--

ALTER TABLE ONLY sgw_player_content_cooldown
    ADD CONSTRAINT sgw_player_content_cooldown_player_id_fkey FOREIGN KEY (player_id) REFERENCES sgw_player(player_id) ON UPDATE RESTRICT ON DELETE CASCADE;

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
-- Name: sgw_organization_ranks_org_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--
-- Composite, so the rank row's org_type copy (which the per-type rank CHECK
-- reads) always equals its organization's type.
--

ALTER TABLE ONLY sgw_organization_ranks
    ADD CONSTRAINT sgw_organization_ranks_org_fkey FOREIGN KEY (org_id, org_type) REFERENCES sgw_organizations(org_id, org_type) ON UPDATE RESTRICT ON DELETE CASCADE;

--
-- Name: sgw_organization_members_org_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--
-- Composite, so the member's org_type copy (which UNIQUE (player_id,
-- org_type) needs) always equals its organization's type.
--

ALTER TABLE ONLY sgw_organization_members
    ADD CONSTRAINT sgw_organization_members_org_fkey FOREIGN KEY (org_id, org_type) REFERENCES sgw_organizations(org_id, org_type) ON UPDATE RESTRICT ON DELETE CASCADE;

--
-- Name: sgw_organization_vault_items_org_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--
-- Composite, so a vault row's org_type copy (which the container CHECK
-- reads) always equals its organization's type. ON DELETE RESTRICT, never
-- CASCADE (D-BV18): deleting an organization never deletes its vault. See
-- the table header in sgw_organization_vault_items.sql.
--

ALTER TABLE ONLY sgw_organization_vault_items
    ADD CONSTRAINT sgw_organization_vault_items_org_fkey FOREIGN KEY (org_id, org_type) REFERENCES sgw_organizations(org_id, org_type) ON UPDATE RESTRICT ON DELETE RESTRICT;

ALTER TABLE ONLY sgw_organization_vault_items
    ADD CONSTRAINT sgw_organization_vault_items_type_id_fkey FOREIGN KEY (type_id) REFERENCES resources.items(item_id) ON UPDATE CASCADE ON DELETE RESTRICT;

--
-- Name: sgw_organization_members_rank_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--
-- A member's rank must be one of the organization's rank rows, so rank 0,
-- and a rank the type does not use, is refused. NO ACTION on delete: a
-- rank row that members hold cannot be deleted on its own (that would
-- silently drop the members). A disband still works, because the check
-- runs at the end of the statement, after the members cascade from
-- sgw_organizations has run too (disband_cascades_ranks_and_members).
--

ALTER TABLE ONLY sgw_organization_members
    ADD CONSTRAINT sgw_organization_members_rank_fkey FOREIGN KEY (org_id, rank) REFERENCES sgw_organization_ranks(org_id, rank) ON UPDATE RESTRICT;

--
-- Name: sgw_organization_members_player_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--
-- A member row goes with its character. The member-delete trigger then
-- promotes a new leader or disbands the organization (D-ORG12).
--
-- Composite on (player_id, account_id), so the member's account_id copy
-- (which the trigger's audit rows need after the character is gone) cannot
-- differ from the character's. ON UPDATE RESTRICT, not CASCADE: a character
-- never changes account, and the member row's BEFORE UPDATE trigger refuses
-- an account_id change anyway, so a cascade could only ever fail. Moving a
-- character between accounts would have to leave its organizations first.
--

ALTER TABLE ONLY sgw_organization_members
    ADD CONSTRAINT sgw_organization_members_player_fkey FOREIGN KEY (player_id, account_id) REFERENCES sgw_player(player_id, account_id) ON UPDATE RESTRICT ON DELETE CASCADE;
--
-- Name: sgw_auction_seller_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--
-- ON DELETE CASCADE (D-BM09): a deleted character's listings go with it,
-- settled or open. An open listing's item is a container-18 row of the
-- seller's inventory and cascades with it; the BEFORE DELETE trigger
-- bm_player_before_delete() refunds the standing bidder first. RESTRICT
-- blocked every character that had ever listed from being deleted, because
-- settled rows are kept.
--

ALTER TABLE ONLY sgw_auction
    ADD CONSTRAINT sgw_auction_seller_id_fkey FOREIGN KEY (seller_id) REFERENCES sgw_player(player_id) ON UPDATE RESTRICT ON DELETE CASCADE;

--
-- Name: sgw_auction_current_bidder_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--
-- ON DELETE SET NULL (D-BM09): a settled row keeps its history without its
-- buyer. On an open row the BEFORE DELETE trigger bm_player_before_delete()
-- has already cleared the bid (current_bid = 0), so no phantom bid is left
-- for the sweep: the auction simply reopens.
--

ALTER TABLE ONLY sgw_auction
    ADD CONSTRAINT sgw_auction_current_bidder_fkey FOREIGN KEY (current_bidder) REFERENCES sgw_player(player_id) ON UPDATE RESTRICT ON DELETE SET NULL;

--
-- Name: sgw_auction_bid_sequence_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_auction_bid
    ADD CONSTRAINT sgw_auction_bid_sequence_id_fkey FOREIGN KEY (sequence_id) REFERENCES sgw_auction(sequence_id) ON UPDATE RESTRICT ON DELETE CASCADE;

--
-- Name: sgw_auction_bid_bidder_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_auction_bid
    ADD CONSTRAINT sgw_auction_bid_bidder_id_fkey FOREIGN KEY (bidder_id) REFERENCES sgw_player(player_id) ON UPDATE RESTRICT ON DELETE CASCADE;

