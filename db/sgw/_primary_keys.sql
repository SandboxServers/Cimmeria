--
-- TOC entry 2662 (class 2606 OID 63856)
-- Name: account_pkey; Type: CONSTRAINT; Schema: public; Owner: -; Tablespace: 
--

ALTER TABLE ONLY account
    ADD CONSTRAINT account_pkey PRIMARY KEY (account_id);

--
-- TOC entry 2677 (class 2606 OID 63858)
-- Name: missions_pkey; Type: CONSTRAINT; Schema: public; Owner: -; Tablespace: 
--

ALTER TABLE ONLY sgw_mission
    ADD CONSTRAINT missions_pkey PRIMARY KEY (player_id, mission_id);

--
-- TOC entry 2670 (class 2606 OID 63860)
-- Name: sgw_gate_mail_pkey; Type: CONSTRAINT; Schema: public; Owner: -; Tablespace: 
--

ALTER TABLE ONLY sgw_gate_mail
    ADD CONSTRAINT sgw_gate_mail_pkey PRIMARY KEY (mail_id);

--
-- TOC entry 2672 (class 2606 OID 63862)
-- Name: sgw_inventory_base_pkey; Type: CONSTRAINT; Schema: public; Owner: -; Tablespace: 
--

ALTER TABLE ONLY sgw_inventory_base
    ADD CONSTRAINT sgw_inventory_base_pkey PRIMARY KEY (item_id);

--
-- TOC entry 2675 (class 2606 OID 63864)
-- Name: sgw_inventory_pkey; Type: CONSTRAINT; Schema: public; Owner: -; Tablespace: 
--

ALTER TABLE ONLY sgw_inventory
    ADD CONSTRAINT sgw_inventory_pkey PRIMARY KEY (item_id);

--
-- TOC entry 2664 (class 2606 OID 63866)
-- Name: sgw_player_pkey; Type: CONSTRAINT; Schema: public; Owner: -; Tablespace: 
--

ALTER TABLE ONLY sgw_player
    ADD CONSTRAINT sgw_player_pkey PRIMARY KEY (player_id);

--
-- TOC entry 2666 (class 2606 OID 63868)
-- Name: sgw_player_player_name_key; Type: CONSTRAINT; Schema: public; Owner: -; Tablespace: 
--

ALTER TABLE ONLY sgw_player
    ADD CONSTRAINT sgw_player_player_name_key UNIQUE (player_name);

--
-- TOC entry 2679 (class 2606 OID 63870)
-- Name: shards_pkey; Type: CONSTRAINT; Schema: public; Owner: -; Tablespace: 
--

ALTER TABLE ONLY shards
    ADD CONSTRAINT shards_pkey PRIMARY KEY (shard_id);

