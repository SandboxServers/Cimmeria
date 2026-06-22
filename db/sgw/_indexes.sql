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
-- Index: sgw_auction_expiry_idx
-- Supports the Phase 3 expiry sweep: WHERE status = 0 AND expires_at <= now.
--

CREATE INDEX sgw_auction_expiry_idx ON sgw_auction USING btree (status, expires_at);

--
-- Index: sgw_auction_seller_idx
-- Supports "my listings" lookups by seller.
--

CREATE INDEX sgw_auction_seller_idx ON sgw_auction USING btree (seller_id);

--
-- Index: sgw_auction_bid_sequence_idx
-- Supports bid-history lookups for one auction.
--

CREATE INDEX sgw_auction_bid_sequence_idx ON sgw_auction_bid USING btree (sequence_id);

