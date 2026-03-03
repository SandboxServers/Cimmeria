--
-- TOC entry 2914 (class 2606 OID 63189)
-- Name: abilities_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY abilities
    ADD CONSTRAINT abilities_pkey PRIMARY KEY (ability_id);

--
-- TOC entry 2916 (class 2606 OID 63191)
-- Name: ability_moniker_groups_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY ability_moniker_groups
    ADD CONSTRAINT ability_moniker_groups_pkey PRIMARY KEY (group_id);

--
-- TOC entry 2918 (class 2606 OID 63193)
-- Name: ability_set_abilities_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY ability_set_abilities
    ADD CONSTRAINT ability_set_abilities_pkey PRIMARY KEY (ability_set_id);

--
-- TOC entry 2920 (class 2606 OID 63195)
-- Name: ability_sets2_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY ability_sets
    ADD CONSTRAINT ability_sets2_pkey PRIMARY KEY (ability_set_id);

--
-- TOC entry 2922 (class 2606 OID 63197)
-- Name: applied_science_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY applied_science
    ADD CONSTRAINT applied_science_pkey PRIMARY KEY (id);

--
-- TOC entry 2924 (class 2606 OID 63199)
-- Name: archetype_ability_tree_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY archetype_ability_tree
    ADD CONSTRAINT archetype_ability_tree_pkey PRIMARY KEY (archetype, tree_index, ability_index);

--
-- TOC entry 2926 (class 2606 OID 63201)
-- Name: archetypes_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY archetypes
    ADD CONSTRAINT archetypes_pkey PRIMARY KEY (archetype);

--
-- TOC entry 2930 (class 2606 OID 63203)
-- Name: blueprints_components_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY blueprints_components
    ADD CONSTRAINT blueprints_components_pkey PRIMARY KEY (blueprint_id, component_set_id, item_id);

--
-- TOC entry 2928 (class 2606 OID 63205)
-- Name: blueprints_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY blueprints
    ADD CONSTRAINT blueprints_pkey PRIMARY KEY (blueprint_id);

--
-- TOC entry 2934 (class 2606 OID 63207)
-- Name: body_components_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY body_components
    ADD CONSTRAINT body_components_pkey PRIMARY KEY (component_name);

--
-- TOC entry 2936 (class 2606 OID 63209)
-- Name: body_sets_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY body_sets
    ADD CONSTRAINT body_sets_pkey PRIMARY KEY (body_set);

--
-- TOC entry 2940 (class 2606 OID 63211)
-- Name: char_creation_abilities_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY char_creation_abilities
    ADD CONSTRAINT char_creation_abilities_pkey PRIMARY KEY (char_def_id, ability_id);

--
-- TOC entry 2942 (class 2606 OID 63213)
-- Name: char_creation_choices_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY char_creation_choices
    ADD CONSTRAINT char_creation_choices_pkey PRIMARY KEY (choice_id);

--
-- TOC entry 2938 (class 2606 OID 63215)
-- Name: char_creation_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY char_creation
    ADD CONSTRAINT char_creation_pkey PRIMARY KEY (char_def_id);

--
-- TOC entry 2944 (class 2606 OID 63217)
-- Name: char_creation_visgroups_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY char_creation_visgroups
    ADD CONSTRAINT char_creation_visgroups_pkey PRIMARY KEY (vis_group_id);

--
-- TOC entry 2932 (class 2606 OID 63219)
-- Name: component_visuals_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY body_component_visuals
    ADD CONSTRAINT component_visuals_pkey PRIMARY KEY (component_name, index);

--
-- TOC entry 2946 (class 2606 OID 63221)
-- Name: containers_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY containers
    ADD CONSTRAINT containers_pkey PRIMARY KEY (container_id);

--
-- TOC entry 2956 (class 2606 OID 63223)
-- Name: dialog_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY dialogs
    ADD CONSTRAINT dialog_pkey PRIMARY KEY (dialog_id);

--
-- TOC entry 3061 (class 2606 OID 64183)
-- Name: dialog_screen_buttons_2_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY dialog_screen_buttons
    ADD CONSTRAINT dialog_screen_buttons_2_pkey PRIMARY KEY (screen_button_id);

--
-- TOC entry 3063 (class 2606 OID 64185)
-- Name: dialog_screen_buttons_2_screen_id_button_id_key; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY dialog_screen_buttons
    ADD CONSTRAINT dialog_screen_buttons_2_screen_id_button_id_key UNIQUE (screen_id, button_id);

--
-- TOC entry 2948 (class 2606 OID 63227)
-- Name: dialog_screens_dialog_id_index_key; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY dialog_screens
    ADD CONSTRAINT dialog_screens_dialog_id_index_key UNIQUE (dialog_id, index);

--
-- TOC entry 2950 (class 2606 OID 63229)
-- Name: dialog_screens_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY dialog_screens
    ADD CONSTRAINT dialog_screens_pkey PRIMARY KEY (screen_id);

--
-- TOC entry 2952 (class 2606 OID 63231)
-- Name: dialog_set_maps_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY dialog_set_maps
    ADD CONSTRAINT dialog_set_maps_pkey PRIMARY KEY (dialog_set_map_id);

--
-- TOC entry 2954 (class 2606 OID 63233)
-- Name: dialog_sets_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY dialog_sets
    ADD CONSTRAINT dialog_sets_pkey PRIMARY KEY (dialog_set_id);

--
-- TOC entry 2958 (class 2606 OID 63235)
-- Name: disciplines_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY disciplines
    ADD CONSTRAINT disciplines_pkey PRIMARY KEY (discipline_id);

--
-- TOC entry 3065 (class 2606 OID 64206)
-- Name: effect_nvps_2_effect_id_name_key; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY effect_nvps
    ADD CONSTRAINT effect_nvps_2_effect_id_name_key UNIQUE (effect_id, name);

--
-- TOC entry 3067 (class 2606 OID 64204)
-- Name: effect_nvps_2_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY effect_nvps
    ADD CONSTRAINT effect_nvps_2_pkey PRIMARY KEY (nvp_id);

--
-- TOC entry 2960 (class 2606 OID 63239)
-- Name: effects_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY effects
    ADD CONSTRAINT effects_pkey PRIMARY KEY (effect_id);

--
-- TOC entry 2962 (class 2606 OID 63241)
-- Name: entity_interactions_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY entity_interactions
    ADD CONSTRAINT entity_interactions_pkey PRIMARY KEY (entity_interaction_id);

--
-- TOC entry 2964 (class 2606 OID 63243)
-- Name: entity_templates_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY entity_templates
    ADD CONSTRAINT entity_templates_pkey PRIMARY KEY (template_id);

--
-- TOC entry 2966 (class 2606 OID 63245)
-- Name: entity_templates_template_name_key; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY entity_templates
    ADD CONSTRAINT entity_templates_template_name_key UNIQUE (template_name);

--
-- TOC entry 2968 (class 2606 OID 63247)
-- Name: error_text_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY error_texts
    ADD CONSTRAINT error_text_pkey PRIMARY KEY (error_id);

--
-- TOC entry 2972 (class 2606 OID 63249)
-- Name: event_sets_instances_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY event_sets_sequences
    ADD CONSTRAINT event_sets_instances_pkey PRIMARY KEY (event_set_id, sequence_id);

--
-- TOC entry 2970 (class 2606 OID 63251)
-- Name: event_sets_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY event_sets
    ADD CONSTRAINT event_sets_pkey PRIMARY KEY (event_set_id);

--
-- TOC entry 2974 (class 2606 OID 63253)
-- Name: generic_regions_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY generic_regions
    ADD CONSTRAINT generic_regions_pkey PRIMARY KEY (region_id);

--
-- TOC entry 2976 (class 2606 OID 63255)
-- Name: interactions_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY interactions
    ADD CONSTRAINT interactions_pkey PRIMARY KEY (interaction_id);

--
-- TOC entry 2978 (class 2606 OID 63257)
-- Name: item_list_items_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY item_list_items
    ADD CONSTRAINT item_list_items_pkey PRIMARY KEY (item_id);

--
-- TOC entry 2980 (class 2606 OID 63259)
-- Name: item_list_prices_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY item_list_prices
    ADD CONSTRAINT item_list_prices_pkey PRIMARY KEY (item_id, design_id);

--
-- TOC entry 2982 (class 2606 OID 63261)
-- Name: item_lists_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY item_lists
    ADD CONSTRAINT item_lists_pkey PRIMARY KEY (item_list_id);

--
-- TOC entry 3070 (class 2606 OID 64296)
-- Name: items_event_sets_2_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY items_event_sets
    ADD CONSTRAINT items_event_sets_2_pkey PRIMARY KEY (item_event_id);

--
-- TOC entry 2984 (class 2606 OID 63263)
-- Name: items_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY items
    ADD CONSTRAINT items_pkey PRIMARY KEY (item_id);

--
-- TOC entry 2986 (class 2606 OID 63265)
-- Name: loot_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY loot
    ADD CONSTRAINT loot_pkey PRIMARY KEY (loot_id);

--
-- TOC entry 2989 (class 2606 OID 63267)
-- Name: loot_tables_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY loot_tables
    ADD CONSTRAINT loot_tables_pkey PRIMARY KEY (loot_table_id);

--
-- TOC entry 2991 (class 2606 OID 63269)
-- Name: mission_objectives_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY mission_objectives
    ADD CONSTRAINT mission_objectives_pkey PRIMARY KEY (objective_id);

--
-- TOC entry 2999 (class 2606 OID 63271)
-- Name: mission_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY missions
    ADD CONSTRAINT mission_pkey PRIMARY KEY (mission_id);

--
-- TOC entry 2993 (class 2606 OID 63273)
-- Name: mission_reward_groups_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY mission_reward_groups
    ADD CONSTRAINT mission_reward_groups_pkey PRIMARY KEY (group_id);

--
-- TOC entry 3059 (class 2606 OID 64135)
-- Name: mission_rewards_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY mission_rewards
    ADD CONSTRAINT mission_rewards_pkey PRIMARY KEY (reward_id);

--
-- TOC entry 2995 (class 2606 OID 63277)
-- Name: mission_steps_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY mission_steps
    ADD CONSTRAINT mission_steps_pkey PRIMARY KEY (step_id);

--
-- TOC entry 2997 (class 2606 OID 63279)
-- Name: mission_tasks_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY mission_tasks
    ADD CONSTRAINT mission_tasks_pkey PRIMARY KEY (task_id);

--
-- TOC entry 3001 (class 2606 OID 63281)
-- Name: monikers_name_key; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY monikers
    ADD CONSTRAINT monikers_name_key UNIQUE (name);

--
-- TOC entry 3003 (class 2606 OID 63283)
-- Name: monikers_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY monikers
    ADD CONSTRAINT monikers_pkey PRIMARY KEY (moniker_id);

--
-- TOC entry 3005 (class 2606 OID 63285)
-- Name: paths_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY paths
    ADD CONSTRAINT paths_pkey PRIMARY KEY (path_id, index);

--
-- TOC entry 3007 (class 2606 OID 63287)
-- Name: point_set_points_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY point_set_points
    ADD CONSTRAINT point_set_points_pkey PRIMARY KEY (point_id);

--
-- TOC entry 3009 (class 2606 OID 63289)
-- Name: point_sets_name_key; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY point_sets
    ADD CONSTRAINT point_sets_name_key UNIQUE (name);

--
-- TOC entry 3011 (class 2606 OID 63291)
-- Name: point_sets_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY point_sets
    ADD CONSTRAINT point_sets_pkey PRIMARY KEY (set_id);

--
-- TOC entry 3013 (class 2606 OID 63293)
-- Name: racial_paradigm_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY racial_paradigm
    ADD CONSTRAINT racial_paradigm_pkey PRIMARY KEY (id);

--
-- TOC entry 3015 (class 2606 OID 63295)
-- Name: resource_types_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY resource_types
    ADD CONSTRAINT resource_types_pkey PRIMARY KEY ("table");

--
-- TOC entry 3018 (class 2606 OID 63297)
-- Name: resource_versions_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY resource_versions
    ADD CONSTRAINT resource_versions_pkey PRIMARY KEY (type, version);

--
-- TOC entry 3020 (class 2606 OID 63299)
-- Name: respawners_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY respawners
    ADD CONSTRAINT respawners_pkey PRIMARY KEY (respawner_id);

--
-- TOC entry 3023 (class 2606 OID 63301)
-- Name: ring_transport_regions_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY ring_transport_regions
    ADD CONSTRAINT ring_transport_regions_pkey PRIMARY KEY (region_id);

--
-- TOC entry 3025 (class 2606 OID 63303)
-- Name: sequences_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY sequences
    ADD CONSTRAINT sequences_pkey PRIMARY KEY (sequence_id);

--
-- TOC entry 3027 (class 2606 OID 63305)
-- Name: sequences_scriptnvp_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY sequences_nvp
    ADD CONSTRAINT sequences_scriptnvp_pkey PRIMARY KEY (sequence_id, name);

--
-- TOC entry 3029 (class 2606 OID 63307)
-- Name: skeletal_meshes_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY skeletal_meshes
    ADD CONSTRAINT skeletal_meshes_pkey PRIMARY KEY (mesh_name);

--
-- TOC entry 3031 (class 2606 OID 63309)
-- Name: spawn_points_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY spawn_points
    ADD CONSTRAINT spawn_points_pkey PRIMARY KEY (point_id);

--
-- TOC entry 3033 (class 2606 OID 63311)
-- Name: spawn_sets_name_key; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY spawn_sets
    ADD CONSTRAINT spawn_sets_name_key UNIQUE (name);

--
-- TOC entry 3035 (class 2606 OID 63313)
-- Name: spawn_sets_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY spawn_sets
    ADD CONSTRAINT spawn_sets_pkey PRIMARY KEY (set_id);

--
-- TOC entry 3037 (class 2606 OID 63315)
-- Name: spawnlist_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY spawnlist
    ADD CONSTRAINT spawnlist_pkey PRIMARY KEY (spawn_id);

--
-- TOC entry 3039 (class 2606 OID 63317)
-- Name: spawnlist_tag_key; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY spawnlist
    ADD CONSTRAINT spawnlist_tag_key UNIQUE (tag);

--
-- TOC entry 3042 (class 2606 OID 63319)
-- Name: speakers_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY speakers
    ADD CONSTRAINT speakers_pkey PRIMARY KEY (speaker_id);

--
-- TOC entry 3044 (class 2606 OID 63321)
-- Name: special_words_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY special_words
    ADD CONSTRAINT special_words_pkey PRIMARY KEY (special_word);

--
-- TOC entry 3046 (class 2606 OID 63323)
-- Name: stargates_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY stargates
    ADD CONSTRAINT stargates_pkey PRIMARY KEY (stargate_id);

--
-- TOC entry 3048 (class 2606 OID 63325)
-- Name: static_meshes_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY static_meshes
    ADD CONSTRAINT static_meshes_pkey PRIMARY KEY (mesh_name);

--
-- TOC entry 3050 (class 2606 OID 63327)
-- Name: texts_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY texts
    ADD CONSTRAINT texts_pkey PRIMARY KEY (moniker_id);

--
-- TOC entry 3052 (class 2606 OID 63329)
-- Name: trainer_abilities_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY trainer_abilities
    ADD CONSTRAINT trainer_abilities_pkey PRIMARY KEY (list_id, archetype, ability_id);

--
-- TOC entry 3054 (class 2606 OID 63331)
-- Name: trainer_ability_lists_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY trainer_ability_lists
    ADD CONSTRAINT trainer_ability_lists_pkey PRIMARY KEY (list_id);

--
-- TOC entry 3056 (class 2606 OID 63333)
-- Name: worlds_pkey; Type: CONSTRAINT; Schema: resources; Owner: -; Tablespace: 
--

ALTER TABLE ONLY worlds
    ADD CONSTRAINT worlds_pkey PRIMARY KEY (world_id);

