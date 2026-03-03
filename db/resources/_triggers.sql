--
-- TOC entry 3141 (class 2620 OID 63340)
-- Name: abilities_update_trig; Type: TRIGGER; Schema: resources; Owner: -
--

CREATE TRIGGER abilities_update_trig AFTER INSERT OR DELETE OR UPDATE ON abilities FOR EACH ROW EXECUTE PROCEDURE resource_update_trigger();

--
-- TOC entry 3142 (class 2620 OID 63341)
-- Name: applied_science_update_trig; Type: TRIGGER; Schema: resources; Owner: -
--

CREATE TRIGGER applied_science_update_trig AFTER INSERT OR DELETE OR UPDATE ON applied_science FOR EACH ROW EXECUTE PROCEDURE resource_update_trigger();

--
-- TOC entry 3143 (class 2620 OID 63342)
-- Name: blueprints_update_trig; Type: TRIGGER; Schema: resources; Owner: -
--

CREATE TRIGGER blueprints_update_trig AFTER INSERT OR DELETE OR UPDATE ON blueprints FOR EACH ROW EXECUTE PROCEDURE resource_update_trigger();

--
-- TOC entry 3144 (class 2620 OID 63343)
-- Name: containers_update_trig; Type: TRIGGER; Schema: resources; Owner: -
--

CREATE TRIGGER containers_update_trig AFTER INSERT OR DELETE OR UPDATE ON containers FOR EACH ROW EXECUTE PROCEDURE resource_update_trigger();

--
-- TOC entry 3145 (class 2620 OID 63344)
-- Name: dialog_set_maps_update_trig; Type: TRIGGER; Schema: resources; Owner: -
--

CREATE TRIGGER dialog_set_maps_update_trig AFTER INSERT OR DELETE OR UPDATE ON dialog_set_maps FOR EACH ROW EXECUTE PROCEDURE resource_update_trigger();

--
-- TOC entry 3146 (class 2620 OID 63345)
-- Name: dialog_update_trig; Type: TRIGGER; Schema: resources; Owner: -
--

CREATE TRIGGER dialog_update_trig AFTER INSERT OR DELETE OR UPDATE ON dialogs FOR EACH ROW EXECUTE PROCEDURE resource_update_trigger();

--
-- TOC entry 3147 (class 2620 OID 63346)
-- Name: disciplines_update_trig; Type: TRIGGER; Schema: resources; Owner: -
--

CREATE TRIGGER disciplines_update_trig AFTER INSERT OR DELETE OR UPDATE ON disciplines FOR EACH ROW EXECUTE PROCEDURE resource_update_trigger();

--
-- TOC entry 3148 (class 2620 OID 63347)
-- Name: effects_update_trig; Type: TRIGGER; Schema: resources; Owner: -
--

CREATE TRIGGER effects_update_trig AFTER INSERT OR DELETE OR UPDATE ON effects FOR EACH ROW EXECUTE PROCEDURE resource_update_trigger();

--
-- TOC entry 3149 (class 2620 OID 63348)
-- Name: error_texts_update_trig; Type: TRIGGER; Schema: resources; Owner: -
--

CREATE TRIGGER error_texts_update_trig AFTER INSERT OR DELETE OR UPDATE ON error_texts FOR EACH ROW EXECUTE PROCEDURE resource_update_trigger();

--
-- TOC entry 3151 (class 2620 OID 63349)
-- Name: event_sets_sequences_update_trig; Type: TRIGGER; Schema: resources; Owner: -
--

CREATE TRIGGER event_sets_sequences_update_trig AFTER INSERT OR DELETE OR UPDATE ON event_sets_sequences FOR EACH ROW EXECUTE PROCEDURE resource_update_trigger();

--
-- TOC entry 3150 (class 2620 OID 63350)
-- Name: event_sets_update_trig; Type: TRIGGER; Schema: resources; Owner: -
--

CREATE TRIGGER event_sets_update_trig AFTER INSERT OR DELETE OR UPDATE ON event_sets FOR EACH ROW EXECUTE PROCEDURE resource_update_trigger();

--
-- TOC entry 3152 (class 2620 OID 63351)
-- Name: interactions_update_trig; Type: TRIGGER; Schema: resources; Owner: -
--

CREATE TRIGGER interactions_update_trig AFTER INSERT OR DELETE OR UPDATE ON interactions FOR EACH ROW EXECUTE PROCEDURE resource_update_trigger();

--
-- TOC entry 3162 (class 2620 OID 64307)
-- Name: items_event_sets_update_trig; Type: TRIGGER; Schema: resources; Owner: -
--

CREATE TRIGGER items_event_sets_update_trig AFTER INSERT OR DELETE OR UPDATE ON items_event_sets FOR EACH ROW EXECUTE PROCEDURE resource_update_trigger();

--
-- TOC entry 3153 (class 2620 OID 63353)
-- Name: items_update_trig; Type: TRIGGER; Schema: resources; Owner: -
--

CREATE TRIGGER items_update_trig AFTER INSERT OR DELETE OR UPDATE OF item_id, applied_science_id, description, icon_location, name, quality_id, tech_comp, tier, container_sets, max_stack_size, moniker_ids, max_ranged_range, min_ranged_range, max_melee_range, min_melee_range, discipline_ids, flags ON items FOR EACH ROW EXECUTE PROCEDURE resource_update_trigger();

--
-- TOC entry 3154 (class 2620 OID 63354)
-- Name: mission_steps_update_trig; Type: TRIGGER; Schema: resources; Owner: -
--

CREATE TRIGGER mission_steps_update_trig AFTER INSERT OR DELETE OR UPDATE ON mission_steps FOR EACH ROW EXECUTE PROCEDURE resource_update_trigger();

--
-- TOC entry 3155 (class 2620 OID 63355)
-- Name: missions_update_trig; Type: TRIGGER; Schema: resources; Owner: -
--

CREATE TRIGGER missions_update_trig AFTER INSERT OR DELETE OR UPDATE OF mission_id, history_text, award_xp, can_abandon, can_fail, can_repeat_on_fail, difficulty, is_a_story, is_enabled, is_hidden, is_override_mission, is_shareable, level, mission_defn, mission_label, num_repeats, show_faction_change_icon, show_instance_icon, show_pvp_icon ON missions FOR EACH ROW EXECUTE PROCEDURE resource_update_trigger();

--
-- TOC entry 3156 (class 2620 OID 63356)
-- Name: racial_paradigm_update_trig; Type: TRIGGER; Schema: resources; Owner: -
--

CREATE TRIGGER racial_paradigm_update_trig AFTER INSERT OR DELETE OR UPDATE ON racial_paradigm FOR EACH ROW EXECUTE PROCEDURE resource_update_trigger();

--
-- TOC entry 3158 (class 2620 OID 63357)
-- Name: sequences_nvp_update_trig; Type: TRIGGER; Schema: resources; Owner: -
--

CREATE TRIGGER sequences_nvp_update_trig AFTER INSERT OR DELETE OR UPDATE ON sequences_nvp FOR EACH ROW EXECUTE PROCEDURE resource_update_trigger();

--
-- TOC entry 3157 (class 2620 OID 63358)
-- Name: sequences_update_trig; Type: TRIGGER; Schema: resources; Owner: -
--

CREATE TRIGGER sequences_update_trig AFTER INSERT OR DELETE OR UPDATE ON sequences FOR EACH ROW EXECUTE PROCEDURE resource_update_trigger();

--
-- TOC entry 3159 (class 2620 OID 63359)
-- Name: stargates_update_trig; Type: TRIGGER; Schema: resources; Owner: -
--

CREATE TRIGGER stargates_update_trig AFTER INSERT OR DELETE OR UPDATE ON stargates FOR EACH ROW EXECUTE PROCEDURE resource_update_trigger();

--
-- TOC entry 3160 (class 2620 OID 63360)
-- Name: texts_update_trig; Type: TRIGGER; Schema: resources; Owner: -
--

CREATE TRIGGER texts_update_trig AFTER INSERT OR DELETE OR UPDATE ON texts FOR EACH ROW EXECUTE PROCEDURE resource_update_trigger();

--
-- TOC entry 3161 (class 2620 OID 63361)
-- Name: worlds_update_trig; Type: TRIGGER; Schema: resources; Owner: -
--

CREATE TRIGGER worlds_update_trig AFTER INSERT OR DELETE OR UPDATE OF world_id, flags, min_per_day, min_to_real_min, world, client_map ON worlds FOR EACH ROW EXECUTE PROCEDURE resource_update_trigger();

