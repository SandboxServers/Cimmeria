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

--
-- Name: sgw_contact_list_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_contact_list
    ADD CONSTRAINT sgw_contact_list_pkey PRIMARY KEY (list_id);

--
-- Name: sgw_contact_list_player_id_name_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_contact_list
    ADD CONSTRAINT sgw_contact_list_player_id_name_key UNIQUE (player_id, name);

--
-- Name: sgw_contact_list_member_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_contact_list_member
    ADD CONSTRAINT sgw_contact_list_member_pkey PRIMARY KEY (list_id, player_name);

--
-- Name: sgw_organizations_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_organizations
    ADD CONSTRAINT sgw_organizations_pkey PRIMARY KEY (org_id);

--
-- Name: sgw_organizations_org_type_name_key_key; Type: CONSTRAINT; Schema: public; Owner: -
--
-- Organization names are unique per type on the case-folded key (D-ORG10):
-- a Team and a Command may share a name.
--

ALTER TABLE ONLY sgw_organizations
    ADD CONSTRAINT sgw_organizations_org_type_name_key_key UNIQUE (org_type, name_key);

--
-- Name: sgw_organizations_org_id_org_type_key; Type: CONSTRAINT; Schema: public; Owner: -
--
-- Redundant as a key (org_id alone is unique); it exists as the target of
-- sgw_organization_members_org_fkey, which pins each member row's org_type
-- copy to its organization's.
--

ALTER TABLE ONLY sgw_organizations
    ADD CONSTRAINT sgw_organizations_org_id_org_type_key UNIQUE (org_id, org_type);

--
-- Name: sgw_organization_ranks_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_organization_ranks
    ADD CONSTRAINT sgw_organization_ranks_pkey PRIMARY KEY (org_id, rank);

--
-- Name: sgw_organization_members_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_organization_members
    ADD CONSTRAINT sgw_organization_members_pkey PRIMARY KEY (org_id, player_id);

--
-- Name: sgw_organization_members_player_id_org_type_key; Type: CONSTRAINT; Schema: public; Owner: -
--
-- One Team and one Command per player (D-ORG18). Also serves the login
-- query "which organizations is this player in" (leading player_id).
--

ALTER TABLE ONLY sgw_organization_members
    ADD CONSTRAINT sgw_organization_members_player_id_org_type_key UNIQUE (player_id, org_type);

--
-- Name: sgw_organization_events_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_organization_events
    ADD CONSTRAINT sgw_organization_events_pkey PRIMARY KEY (event_id);

