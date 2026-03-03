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

