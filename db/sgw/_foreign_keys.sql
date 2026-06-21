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
-- Name: sgw_auction_seller_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--
-- Deleting the seller removes their listings (the escrowed item is handled
-- server-side before the row goes away in later phases).
--

ALTER TABLE ONLY sgw_auction
    ADD CONSTRAINT sgw_auction_seller_id_fkey FOREIGN KEY (seller_id) REFERENCES sgw_player(player_id) ON UPDATE RESTRICT ON DELETE CASCADE;

--
-- Name: sgw_auction_current_bidder_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--
-- current_bidder is nullable (no bids yet); on bidder deletion the high-bid
-- pointer is cleared rather than dropping the auction.
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

