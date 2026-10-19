-- Dakara_E1 rebuild: mission 63100 "Home to Dakara" (D-DK1 option F, packet DK-10fs).
-- Campaign: docs/analysis/dakara-e1-rebuild/. NEW CONTENT: written by this
-- project, no recovered counterpart. The client's copy is the MISSION_ADDITIONS
-- entry in crates/resources/src/base/mission_additions_dakara.rs; a resources
-- test fails if the two disagree. Ids 63100-63103. Level 1, no reward (D-DK7).

SET search_path = resources, pg_catalog;

INSERT INTO missions (mission_id, history_text, award_xp, can_abandon, can_fail, can_repeat_on_fail, difficulty, is_a_story, is_enabled, is_hidden, is_override_mission, is_shareable, level, mission_defn, mission_label, num_repeats, show_faction_change_icon, show_instance_icon, show_pvp_icon, script_name, reward_naq, reward_xp, script_spaces) VALUES (63100, 'Home to Dakara', true, false, false, true, 1, true, true, false, false, false, 1, 'Home to Dakara', 'General', 1, false, false, false, NULL, 0, 0, NULL);
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (63101, 63100, false, 1, false, 'Speak with Rak''nor at the Stargate.', 0);
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (63102, 63101, false, 1, false, false, false, 'Rak''nor keeps watch over the Stargate plaza. Tell him what you saw at Stargate Command.');
INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (63103, 63102, false, 1, true, 1);
