--
-- TOC entry 3071 (class 2606 OID 63362)
-- Name: abilities_event_set_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY abilities
    ADD CONSTRAINT abilities_event_set_id_fkey FOREIGN KEY (event_set_id) REFERENCES event_sets(event_set_id) ON UPDATE CASCADE ON DELETE RESTRICT;

--
-- TOC entry 3072 (class 2606 OID 63367)
-- Name: ability_moniker_groups_ability_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY ability_moniker_groups
    ADD CONSTRAINT ability_moniker_groups_ability_id_fkey FOREIGN KEY (ability_id) REFERENCES abilities(ability_id);

--
-- TOC entry 3073 (class 2606 OID 63372)
-- Name: ability_set_abilities_ability_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY ability_set_abilities
    ADD CONSTRAINT ability_set_abilities_ability_id_fkey FOREIGN KEY (ability_id) REFERENCES abilities(ability_id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- TOC entry 3074 (class 2606 OID 63377)
-- Name: ability_set_abilities_ability_set_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY ability_set_abilities
    ADD CONSTRAINT ability_set_abilities_ability_set_id_fkey FOREIGN KEY (ability_set_id) REFERENCES ability_sets(ability_set_id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- TOC entry 3075 (class 2606 OID 63382)
-- Name: archetype_ability_tree_ability_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY archetype_ability_tree
    ADD CONSTRAINT archetype_ability_tree_ability_id_fkey FOREIGN KEY (ability_id) REFERENCES abilities(ability_id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- TOC entry 3078 (class 2606 OID 63387)
-- Name: blueprints_components_blueprint_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY blueprints_components
    ADD CONSTRAINT blueprints_components_blueprint_id_fkey FOREIGN KEY (blueprint_id) REFERENCES blueprints(blueprint_id) ON UPDATE CASCADE ON DELETE CASCADE;

--
-- TOC entry 3079 (class 2606 OID 63392)
-- Name: blueprints_components_item_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY blueprints_components
    ADD CONSTRAINT blueprints_components_item_id_fkey FOREIGN KEY (item_id) REFERENCES items(item_id) ON UPDATE CASCADE ON DELETE RESTRICT;

--
-- TOC entry 3076 (class 2606 OID 63397)
-- Name: blueprints_discipline_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY blueprints
    ADD CONSTRAINT blueprints_discipline_id_fkey FOREIGN KEY (discipline_id) REFERENCES disciplines(discipline_id) ON UPDATE CASCADE ON DELETE CASCADE;

--
-- TOC entry 3077 (class 2606 OID 63402)
-- Name: blueprints_product_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY blueprints
    ADD CONSTRAINT blueprints_product_id_fkey FOREIGN KEY (product_id) REFERENCES items(item_id) ON UPDATE CASCADE ON DELETE RESTRICT;

--
-- TOC entry 3080 (class 2606 OID 63407)
-- Name: char_creation_abilities_ability_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY char_creation_abilities
    ADD CONSTRAINT char_creation_abilities_ability_id_fkey FOREIGN KEY (ability_id) REFERENCES abilities(ability_id) ON UPDATE CASCADE ON DELETE CASCADE;

--
-- TOC entry 3081 (class 2606 OID 63412)
-- Name: char_creation_abilities_char_def_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY char_creation_abilities
    ADD CONSTRAINT char_creation_abilities_char_def_id_fkey FOREIGN KEY (char_def_id) REFERENCES char_creation(char_def_id) ON UPDATE CASCADE ON DELETE CASCADE;

--
-- TOC entry 3082 (class 2606 OID 63417)
-- Name: char_creation_choices_item_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY char_creation_choices
    ADD CONSTRAINT char_creation_choices_item_id_fkey FOREIGN KEY (item_id) REFERENCES items(item_id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- TOC entry 3083 (class 2606 OID 63422)
-- Name: char_creation_choices_vis_group_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY char_creation_choices
    ADD CONSTRAINT char_creation_choices_vis_group_id_fkey FOREIGN KEY (vis_group_id) REFERENCES char_creation_visgroups(vis_group_id) ON UPDATE CASCADE ON DELETE CASCADE;

--
-- TOC entry 3084 (class 2606 OID 63427)
-- Name: char_creation_visgroups_char_def_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY char_creation_visgroups
    ADD CONSTRAINT char_creation_visgroups_char_def_id_fkey FOREIGN KEY (char_def_id) REFERENCES char_creation(char_def_id) ON UPDATE CASCADE ON DELETE CASCADE;

--
-- TOC entry 3088 (class 2606 OID 63432)
-- Name: dialog_kismet_event_set_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY dialogs
    ADD CONSTRAINT dialog_kismet_event_set_id_fkey FOREIGN KEY (event_set_id) REFERENCES event_sets(event_set_id) ON UPDATE CASCADE ON DELETE RESTRICT;

--
-- TOC entry 3137 (class 2606 OID 64186)
-- Name: dialog_screen_buttons_2_screen_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY dialog_screen_buttons
    ADD CONSTRAINT dialog_screen_buttons_2_screen_id_fkey FOREIGN KEY (screen_id) REFERENCES dialog_screens(screen_id) ON UPDATE CASCADE ON DELETE CASCADE;

--
-- TOC entry 3086 (class 2606 OID 63442)
-- Name: dialog_screens_dialog_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY dialog_screens
    ADD CONSTRAINT dialog_screens_dialog_id_fkey FOREIGN KEY (dialog_id) REFERENCES dialogs(dialog_id) ON UPDATE CASCADE ON DELETE CASCADE;

--
-- TOC entry 3085 (class 2606 OID 64192)
-- Name: dialog_screens_speaker_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY dialog_screens
    ADD CONSTRAINT dialog_screens_speaker_id_fkey FOREIGN KEY (speaker_id) REFERENCES speakers(speaker_id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- TOC entry 3087 (class 2606 OID 63447)
-- Name: dialog_set_maps_dialog_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY dialog_set_maps
    ADD CONSTRAINT dialog_set_maps_dialog_id_fkey FOREIGN KEY (dialog_id) REFERENCES dialogs(dialog_id) ON UPDATE RESTRICT ON DELETE CASCADE;

--
-- TOC entry 3089 (class 2606 OID 63452)
-- Name: dialogs_accepts_mission_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY dialogs
    ADD CONSTRAINT dialogs_accepts_mission_fkey FOREIGN KEY (accepts_mission_id) REFERENCES missions(mission_id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- TOC entry 3090 (class 2606 OID 63457)
-- Name: disciplines_applied_science_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY disciplines
    ADD CONSTRAINT disciplines_applied_science_id_fkey FOREIGN KEY (applied_science_id) REFERENCES applied_science(id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- TOC entry 3091 (class 2606 OID 63462)
-- Name: disciplines_racial_paradigm_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY disciplines
    ADD CONSTRAINT disciplines_racial_paradigm_id_fkey FOREIGN KEY (racial_paradigm_id) REFERENCES racial_paradigm(id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- TOC entry 3138 (class 2606 OID 64207)
-- Name: effect_nvps_2_effect_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY effect_nvps
    ADD CONSTRAINT effect_nvps_2_effect_id_fkey FOREIGN KEY (effect_id) REFERENCES effects(effect_id) ON UPDATE RESTRICT ON DELETE CASCADE;

--
-- TOC entry 3092 (class 2606 OID 63472)
-- Name: effects_ability_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY effects
    ADD CONSTRAINT effects_ability_id_fkey FOREIGN KEY (ability_id) REFERENCES abilities(ability_id) ON UPDATE CASCADE ON DELETE CASCADE;

--
-- TOC entry 3093 (class 2606 OID 63477)
-- Name: effects_event_set_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY effects
    ADD CONSTRAINT effects_event_set_id_fkey FOREIGN KEY (event_set_id) REFERENCES event_sets(event_set_id) ON UPDATE CASCADE ON DELETE RESTRICT;

--
-- TOC entry 3094 (class 2606 OID 63482)
-- Name: entity_interactions_interaction_set_map_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY entity_interactions
    ADD CONSTRAINT entity_interactions_interaction_set_map_id_fkey FOREIGN KEY (interaction_set_map_id) REFERENCES dialog_set_maps(dialog_set_map_id) ON UPDATE RESTRICT ON DELETE CASCADE;

--
-- TOC entry 3095 (class 2606 OID 63487)
-- Name: entity_interactions_template_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY entity_interactions
    ADD CONSTRAINT entity_interactions_template_id_fkey FOREIGN KEY (template_id) REFERENCES entity_templates(template_id) ON UPDATE RESTRICT ON DELETE CASCADE;

--
-- TOC entry 3097 (class 2606 OID 63492)
-- Name: entity_templates_ability_set_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY entity_templates
    ADD CONSTRAINT entity_templates_ability_set_id_fkey FOREIGN KEY (ability_set_id) REFERENCES ability_sets(ability_set_id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- TOC entry 3098 (class 2606 OID 63497)
-- Name: entity_templates_buy_item_list_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY entity_templates
    ADD CONSTRAINT entity_templates_buy_item_list_fkey FOREIGN KEY (buy_item_list) REFERENCES item_lists(item_list_id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- TOC entry 3099 (class 2606 OID 63502)
-- Name: entity_templates_event_set_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY entity_templates
    ADD CONSTRAINT entity_templates_event_set_id_fkey FOREIGN KEY (event_set_id) REFERENCES event_sets(event_set_id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- TOC entry 3096 (class 2606 OID 70194)
-- Name: entity_templates_interaction_set_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY entity_templates
    ADD CONSTRAINT entity_templates_interaction_set_id_fkey FOREIGN KEY (interaction_set_id) REFERENCES dialog_sets(dialog_set_id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- TOC entry 3100 (class 2606 OID 63507)
-- Name: entity_templates_loot_table_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY entity_templates
    ADD CONSTRAINT entity_templates_loot_table_id_fkey FOREIGN KEY (loot_table_id) REFERENCES loot_tables(loot_table_id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- TOC entry 3101 (class 2606 OID 63512)
-- Name: entity_templates_name_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY entity_templates
    ADD CONSTRAINT entity_templates_name_id_fkey FOREIGN KEY (name_id) REFERENCES texts(moniker_id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- TOC entry 3102 (class 2606 OID 63517)
-- Name: entity_templates_recharge_item_list_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY entity_templates
    ADD CONSTRAINT entity_templates_recharge_item_list_fkey FOREIGN KEY (recharge_item_list) REFERENCES item_lists(item_list_id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- TOC entry 3103 (class 2606 OID 63522)
-- Name: entity_templates_repair_item_list_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY entity_templates
    ADD CONSTRAINT entity_templates_repair_item_list_fkey FOREIGN KEY (repair_item_list) REFERENCES item_lists(item_list_id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- TOC entry 3104 (class 2606 OID 63527)
-- Name: entity_templates_sell_item_list_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY entity_templates
    ADD CONSTRAINT entity_templates_sell_item_list_fkey FOREIGN KEY (sell_item_list) REFERENCES item_lists(item_list_id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- TOC entry 3105 (class 2606 OID 63532)
-- Name: entity_templates_trainer_ability_list_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY entity_templates
    ADD CONSTRAINT entity_templates_trainer_ability_list_id_fkey FOREIGN KEY (trainer_ability_list_id) REFERENCES trainer_ability_lists(list_id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- TOC entry 3106 (class 2606 OID 63537)
-- Name: entity_templates_weapon_item_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY entity_templates
    ADD CONSTRAINT entity_templates_weapon_item_id_fkey FOREIGN KEY (weapon_item_id) REFERENCES items(item_id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- TOC entry 3107 (class 2606 OID 63542)
-- Name: event_sets_instances_event_set_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY event_sets_sequences
    ADD CONSTRAINT event_sets_instances_event_set_id_fkey FOREIGN KEY (event_set_id) REFERENCES event_sets(event_set_id) ON UPDATE CASCADE ON DELETE CASCADE;

--
-- TOC entry 3108 (class 2606 OID 63547)
-- Name: event_sets_instances_kismet_event_set_seq_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY event_sets_sequences
    ADD CONSTRAINT event_sets_instances_kismet_event_set_seq_id_fkey FOREIGN KEY (sequence_id) REFERENCES sequences(sequence_id) ON UPDATE CASCADE ON DELETE RESTRICT;

--
-- TOC entry 3109 (class 2606 OID 63552)
-- Name: generic_regions_world_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY generic_regions
    ADD CONSTRAINT generic_regions_world_id_fkey FOREIGN KEY (world_id) REFERENCES worlds(world_id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- TOC entry 3110 (class 2606 OID 63557)
-- Name: item_list_items_design_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY item_list_items
    ADD CONSTRAINT item_list_items_design_id_fkey FOREIGN KEY (design_id) REFERENCES items(item_id) ON UPDATE CASCADE ON DELETE CASCADE;

--
-- TOC entry 3111 (class 2606 OID 63562)
-- Name: item_list_items_item_list_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY item_list_items
    ADD CONSTRAINT item_list_items_item_list_id_fkey FOREIGN KEY (item_list_id) REFERENCES item_lists(item_list_id) ON UPDATE CASCADE ON DELETE CASCADE;

--
-- TOC entry 3112 (class 2606 OID 63567)
-- Name: item_list_prices_design_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY item_list_prices
    ADD CONSTRAINT item_list_prices_design_id_fkey FOREIGN KEY (design_id) REFERENCES items(item_id) ON UPDATE CASCADE ON DELETE CASCADE;

--
-- TOC entry 3113 (class 2606 OID 63572)
-- Name: items_applied_science_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY items
    ADD CONSTRAINT items_applied_science_id_fkey FOREIGN KEY (applied_science_id) REFERENCES applied_science(id) ON UPDATE CASCADE ON DELETE RESTRICT;

--
-- TOC entry 3140 (class 2606 OID 64297)
-- Name: items_event_sets_2_ability_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY items_event_sets
    ADD CONSTRAINT items_event_sets_2_ability_id_fkey FOREIGN KEY (ability_id) REFERENCES abilities(ability_id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- TOC entry 3139 (class 2606 OID 64302)
-- Name: items_event_sets_2_item_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY items_event_sets
    ADD CONSTRAINT items_event_sets_2_item_id_fkey FOREIGN KEY (item_id) REFERENCES items(item_id) ON UPDATE RESTRICT ON DELETE CASCADE;

--
-- TOC entry 3114 (class 2606 OID 63587)
-- Name: loot_design_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY loot
    ADD CONSTRAINT loot_design_id_fkey FOREIGN KEY (design_id) REFERENCES items(item_id) ON UPDATE CASCADE ON DELETE CASCADE;

--
-- TOC entry 3115 (class 2606 OID 63592)
-- Name: loot_loot_table_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY loot
    ADD CONSTRAINT loot_loot_table_id_fkey FOREIGN KEY (loot_table_id) REFERENCES loot_tables(loot_table_id) ON UPDATE CASCADE ON DELETE CASCADE;

--
-- TOC entry 3116 (class 2606 OID 63597)
-- Name: mission_objectives_step_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY mission_objectives
    ADD CONSTRAINT mission_objectives_step_id_fkey FOREIGN KEY (step_id) REFERENCES mission_steps(step_id) ON UPDATE CASCADE ON DELETE CASCADE;

--
-- TOC entry 3117 (class 2606 OID 63602)
-- Name: mission_reward_groups_mission_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY mission_reward_groups
    ADD CONSTRAINT mission_reward_groups_mission_id_fkey FOREIGN KEY (mission_id) REFERENCES missions(mission_id) ON UPDATE CASCADE ON DELETE CASCADE;

--
-- TOC entry 3136 (class 2606 OID 64136)
-- Name: mission_rewards_group_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY mission_rewards
    ADD CONSTRAINT mission_rewards_group_id_fkey FOREIGN KEY (group_id) REFERENCES mission_reward_groups(group_id) ON UPDATE RESTRICT ON DELETE CASCADE;

--
-- TOC entry 3135 (class 2606 OID 64141)
-- Name: mission_rewards_item_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY mission_rewards
    ADD CONSTRAINT mission_rewards_item_id_fkey FOREIGN KEY (item_id) REFERENCES items(item_id) ON UPDATE RESTRICT ON DELETE CASCADE;

--
-- TOC entry 3118 (class 2606 OID 63617)
-- Name: mission_steps_mission_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY mission_steps
    ADD CONSTRAINT mission_steps_mission_id_fkey FOREIGN KEY (mission_id) REFERENCES missions(mission_id) ON UPDATE CASCADE ON DELETE CASCADE;

--
-- TOC entry 3119 (class 2606 OID 63622)
-- Name: mission_tasks_objective_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY mission_tasks
    ADD CONSTRAINT mission_tasks_objective_id_fkey FOREIGN KEY (objective_id) REFERENCES mission_objectives(objective_id) ON UPDATE CASCADE ON DELETE CASCADE;

--
-- TOC entry 3120 (class 2606 OID 63627)
-- Name: point_set_points_set_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY point_set_points
    ADD CONSTRAINT point_set_points_set_id_fkey FOREIGN KEY (set_id) REFERENCES point_sets(set_id) ON UPDATE RESTRICT ON DELETE CASCADE;

--
-- TOC entry 3121 (class 2606 OID 63632)
-- Name: point_sets_world_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY point_sets
    ADD CONSTRAINT point_sets_world_id_fkey FOREIGN KEY (world_id) REFERENCES worlds(world_id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- TOC entry 3122 (class 2606 OID 63637)
-- Name: respawners_world_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY respawners
    ADD CONSTRAINT respawners_world_id_fkey FOREIGN KEY (world_id) REFERENCES worlds(world_id) ON UPDATE CASCADE ON DELETE CASCADE;

--
-- TOC entry 3123 (class 2606 OID 63642)
-- Name: ring_transport_regions_display_name_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY ring_transport_regions
    ADD CONSTRAINT ring_transport_regions_display_name_id_fkey FOREIGN KEY (display_name_id) REFERENCES texts(moniker_id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- TOC entry 3124 (class 2606 OID 63647)
-- Name: ring_transport_regions_event_set_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY ring_transport_regions
    ADD CONSTRAINT ring_transport_regions_event_set_id_fkey FOREIGN KEY (event_set_id) REFERENCES event_sets(event_set_id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- TOC entry 3125 (class 2606 OID 63652)
-- Name: ring_transport_regions_world_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY ring_transport_regions
    ADD CONSTRAINT ring_transport_regions_world_id_fkey FOREIGN KEY (world_id) REFERENCES worlds(world_id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- TOC entry 3126 (class 2606 OID 63657)
-- Name: sequences_scriptnvp_kismet_event_set_seq_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY sequences_nvp
    ADD CONSTRAINT sequences_scriptnvp_kismet_event_set_seq_id_fkey FOREIGN KEY (sequence_id) REFERENCES sequences(sequence_id) ON UPDATE CASCADE ON DELETE CASCADE;

--
-- TOC entry 3127 (class 2606 OID 63662)
-- Name: spawn_points_spawn_set_name_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY spawn_points
    ADD CONSTRAINT spawn_points_spawn_set_name_fkey FOREIGN KEY (spawn_set_name) REFERENCES entity_templates(template_name);

--
-- TOC entry 3128 (class 2606 OID 63667)
-- Name: spawn_sets_world_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY spawn_sets
    ADD CONSTRAINT spawn_sets_world_id_fkey FOREIGN KEY (world_id) REFERENCES worlds(world_id) ON UPDATE RESTRICT ON DELETE RESTRICT;

--
-- TOC entry 3129 (class 2606 OID 63672)
-- Name: spawnlist_template_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY spawnlist
    ADD CONSTRAINT spawnlist_template_id_fkey FOREIGN KEY (template_id) REFERENCES entity_templates(template_id) ON UPDATE RESTRICT ON DELETE CASCADE;

--
-- TOC entry 3130 (class 2606 OID 63678)
-- Name: spawnlist_world_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY spawnlist
    ADD CONSTRAINT spawnlist_world_id_fkey FOREIGN KEY (world_id) REFERENCES worlds(world_id) ON UPDATE RESTRICT ON DELETE CASCADE;

--
-- TOC entry 3131 (class 2606 OID 63683)
-- Name: stargates_event_set_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY stargates
    ADD CONSTRAINT stargates_event_set_id_fkey FOREIGN KEY (event_set_id) REFERENCES event_sets(event_set_id);

--
-- TOC entry 3132 (class 2606 OID 63688)
-- Name: stargates_world_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY stargates
    ADD CONSTRAINT stargates_world_id_fkey FOREIGN KEY (world_id) REFERENCES worlds(world_id) ON UPDATE CASCADE ON DELETE CASCADE;

--
-- TOC entry 3133 (class 2606 OID 63693)
-- Name: trainer_abilities_ability_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY trainer_abilities
    ADD CONSTRAINT trainer_abilities_ability_id_fkey FOREIGN KEY (ability_id) REFERENCES abilities(ability_id) ON UPDATE RESTRICT ON DELETE CASCADE;

--
-- TOC entry 3134 (class 2606 OID 63698)
-- Name: trainer_abilities_list_id_fkey; Type: FK CONSTRAINT; Schema: resources; Owner: -
--

ALTER TABLE ONLY trainer_abilities
    ADD CONSTRAINT trainer_abilities_list_id_fkey FOREIGN KEY (list_id) REFERENCES trainer_ability_lists(list_id) ON UPDATE RESTRICT ON DELETE CASCADE;

