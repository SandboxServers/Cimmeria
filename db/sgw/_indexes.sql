--
-- TOC entry 2667 (class 1259 OID 63871)
-- Name: mail_lookup_index; Type: INDEX; Schema: public; Owner: -; Tablespace: 
--

CREATE INDEX mail_lookup_index ON sgw_gate_mail USING btree (character_id);

--
-- TOC entry 2668 (class 1259 OID 63872)
-- Name: mail_reverse_lookup_index; Type: INDEX; Schema: public; Owner: -; Tablespace: 
--

CREATE INDEX mail_reverse_lookup_index ON sgw_gate_mail USING btree (sender_id);

--
-- TOC entry 2673 (class 1259 OID 63875)
-- Name: sgw_inventory_Index01; Type: INDEX; Schema: public; Owner: -; Tablespace: 
--

CREATE INDEX "sgw_inventory_Index01" ON sgw_inventory USING btree (character_id);

--
-- Index: sgw_contact_list_member_player_name_idx
-- Supports the login/logout presence fanout query:
--   SELECT cl.player_id FROM sgw_contact_list_member m JOIN sgw_contact_list cl USING (list_id) WHERE m.player_name = $1
--

CREATE INDEX sgw_contact_list_member_player_name_idx ON sgw_contact_list_member USING btree (player_name);

--
-- Index: sgw_organization_members_player_id_idx
-- Supports "which orgs does this player belong to?" queries on login
-- and character-deletion cascades that walk from player_id.
--

CREATE INDEX sgw_organization_members_player_id_idx ON sgw_organization_members USING btree (player_id);

