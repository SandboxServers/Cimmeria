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
-- Index: sgw_contact_list_member_list_lower_name_key
-- Social-systems SS-C1: one entry per name per list, case-insensitively
-- (D-SS13 fold). The Ignore check matches names case-insensitively, so "Bob"
-- and "bob" on one list would be the same entry twice; the contact-list
-- member ops insert with ON CONFLICT DO NOTHING, which this index also feeds.
--

CREATE UNIQUE INDEX sgw_contact_list_member_list_lower_name_key ON sgw_contact_list_member USING btree (list_id, lower((player_name)::text));

--
-- Index: sgw_player_player_name_lower_idx
-- Social-systems SS-M1: D-SS13's case-insensitive fallback when a gate-mail
-- recipient name has no exact match. The mail send path queries
-- lower(player_name) = ANY($2), which is this expression.
--

CREATE INDEX sgw_player_player_name_lower_idx ON sgw_player USING btree (lower((player_name)::text));

--
-- Index: sgw_organization_members_one_leader_idx
-- One Leader per organization. "The leader is the member at rank 8" is the
-- whole leadership model (there is no leader column), so the database
-- refuses a second one. Also serves the member-delete trigger's
-- "is there still a leader" lookup.
--

CREATE UNIQUE INDEX sgw_organization_members_one_leader_idx ON sgw_organization_members USING btree (org_id) WHERE rank = 8;

--
-- Index: sgw_organization_events_unexported_idx
-- The exporters' work queue: rows not yet logged, by writing transaction
-- (the character-delete handler) or in id order (the startup sweep).
--

CREATE INDEX sgw_organization_events_unexported_idx ON sgw_organization_events USING btree (tx_id, event_id) WHERE exported_at IS NULL;

