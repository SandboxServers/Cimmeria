--
-- TOC entry 3068 (class 1259 OID 64212)
-- Name: effect_nvps_effect_id_idx; Type: INDEX; Schema: resources; Owner: -; Tablespace: 
--

CREATE INDEX effect_nvps_effect_id_idx ON effect_nvps USING btree (effect_id);

--
-- TOC entry 2987 (class 1259 OID 63335)
-- Name: loot_template_id_idx; Type: INDEX; Schema: resources; Owner: -; Tablespace: 
--

CREATE INDEX loot_template_id_idx ON loot USING btree (loot_table_id);

--
-- TOC entry 3016 (class 1259 OID 63336)
-- Name: resource_types_type_idx; Type: INDEX; Schema: resources; Owner: -; Tablespace: 
--

CREATE INDEX resource_types_type_idx ON resource_types USING btree (type);

--
-- TOC entry 3021 (class 1259 OID 63337)
-- Name: respawners_world_id_idx; Type: INDEX; Schema: resources; Owner: -; Tablespace: 
--

CREATE INDEX respawners_world_id_idx ON respawners USING btree (world_id);

--
-- TOC entry 3040 (class 1259 OID 63338)
-- Name: spawnlist_world_id_idx; Type: INDEX; Schema: resources; Owner: -; Tablespace: 
--

CREATE INDEX spawnlist_world_id_idx ON spawnlist USING btree (world_id);

--
-- TOC entry 3057 (class 1259 OID 63339)
-- Name: worlds_world_idx; Type: INDEX; Schema: resources; Owner: -; Tablespace: 
--

CREATE UNIQUE INDEX worlds_world_idx ON worlds USING btree (world);

