-- Dakara_E1 rebuild: mission 1570, the Free Jaffa entry.
-- Campaign: docs/analysis/dakara-e1-rebuild/ (packet DK-10a).
-- Chain ID range: 8040-8044 (work-packets.md, C3). Evidence labels: README.md, "Evidence Labels".

SET search_path = resources, pg_catalog;

-- Chain 8040 PROJECT_FINAL_RECONSTRUCTION (D-DK1 option F, D-DK6): when a
-- Free Jaffa finishes the bridge mission 63100 "Home to Dakara" (talking to
-- Rak'nor, chain 8077), they accept 1570 and are moved past its two SGC
-- steps, 4641 (Harriman) and 4642 (enter the gate), to 4906 (speak to
-- Bra'tac). The character never played 4641 or 4642; the mission log shows
-- them completed, which Teal'c's letter (chain 8007) explains. Rak'nor's
-- optional 5936 is completed here: his 6110 has just played. Step 4906's
-- Bra'tac binds are DK-10b's world-62 restore chains (C5 rule 6).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8040, '1570 - Free Jaffa finished 63100 on Dakara_E1: accept, skip SGC steps 4641 and 4642, start at 4906, Rak''nor done', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8040, 'mission_completed', '63100', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8040, 'world', 61, NULL, 'eq', NULL, 0),
  (8040, 'archetype', NULL, NULL, 'eq', '7', 1),
  (8040, 'mission_status', 1570, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8040, 'accept_mission', 1570, NULL, '{}', 0, 0),
  (8040, 'advance_step', 1570, '4642', '{}', 0, 1),
  (8040, 'advance_step', 1570, '4906', '{}', 0, 2),
  (8040, 'complete_objective', 1570, '5936', '{}', 0, 3);
