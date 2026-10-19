-- Dakara_E1 rebuild: mission 1651 "Renewed Attack", step 4930 (the DHD departure to SGC_W1, crossing C2).
-- Campaign: docs/analysis/dakara-e1-rebuild/ (packet DK-30b).
-- Chain ID range: 8310-8319 (work-packets.md, C3). Evidence labels: README.md, "Evidence Labels".

SET search_path = resources, pg_catalog;
-- Chain 8310 PROJECT_FINAL_RECONSTRUCTION: 1651 step 4930, the DHD dials SGC_W1 (crossing C2).
-- Advance to 4928 first (frozen order), then teleport to the SGC H3 (172.79, -10.9, 12.0). The SGC's
-- US-30 receives the player at 4928; no stargate address is granted (the SGC has no DHD to dial back).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8310, '1651 - Step 4930: DHD to SGC_W1 (C2), advance to 4928, teleport H3', 'mission', 1651, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8310, 'interact_tag', 'Dakara_E1_DHD', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8310, 'world', 61, NULL, 'eq', NULL, 0),
  (8310, 'step_status', 1651, '4930', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8310, 'advance_step', 1651, '4928', '{}', 0, 0),
  (8310, 'cross_world_teleport', NULL, 'SGC_W1', '{"x": 172.79, "y": -10.9, "z": 12.0}', 0, 1);

-- Chain 8311 PROJECT_FINAL_RECONSTRUCTION: 1651 step 4928, the DHD teleport to SGC_W1 (crossing C2 recovery).
-- A failed teleport or a GM move left the player on Dakara at 4928; the dial still reaches the SGC.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8311, '1651 - Step 4928: DHD to SGC_W1 (C2 recovery), teleport H3', 'mission', 1651, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8311, 'interact_tag', 'Dakara_E1_DHD', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8311, 'world', 61, NULL, 'eq', NULL, 0),
  (8311, 'step_status', 1651, '4928', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8311, 'cross_world_teleport', NULL, 'SGC_W1', '{"x": 172.79, "y": -10.9, "z": 12.0}', 0, 0);

-- Chain 8312 PROJECT_FINAL_RECONSTRUCTION: 1651 step 4931, the DHD teleport to SGC_W1 (crossing C2 recovery).
-- The same recovery as 8311, for a player who is already at 4931 on Dakara.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8312, '1651 - Step 4931: DHD to SGC_W1 (C2 recovery), teleport H3', 'mission', 1651, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8312, 'interact_tag', 'Dakara_E1_DHD', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8312, 'world', 61, NULL, 'eq', NULL, 0),
  (8312, 'step_status', 1651, '4931', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8312, 'cross_world_teleport', NULL, 'SGC_W1', '{"x": 172.79, "y": -10.9, "z": 12.0}', 0, 0);
