
-- Dakara_E1 rebuild: mission 1651 "Renewed Attack", step 4930 (real gate travel to SGC_W1, crossing C2),
-- and the SGC address grant at Bra'tac's 4932 report (chain 8313).
-- Campaign: docs/analysis/dakara-e1-rebuild/ (packet DK-30b, DK-30b-r).
-- Chain ID range: 8310-8319 (work-packets.md, C3). Evidence labels: README.md, "Evidence Labels".

SET search_path = resources, pg_catalog;

-- Chain 8310 PROJECT_FINAL_RECONSTRUCTION: 1651 step 4930, the player dials SGC_W1 and steps through (crossing C2).
-- D-DK24 (owner 2026-10-19): the player dials the SGC themselves; there is no teleport and no recovery chain.
-- Advance to 4928 (frozen order); the SGC's US-30 receives the player at 4928. Triggered by the gate crossing,
-- not the DHD click: no chain may match the DHD at 4930, because a match would hide the dial window (F25).
-- Only the step condition: dialling from Omega Site's DHD also counts (F41, Castle 1360).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8310, '1651 - Step 4930: crossed to SGC_W1 by gate (C2), advance to 4928', 'mission', 1651, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8310, 'stargate_crossed', 'SGC_W1', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8310, 'step_status', 1651, '4930', 'eq', 'active', 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8310, 'advance_step', 1651, '4928', '{}', 0, 0);

-- Chain 8313 PROJECT_FINAL_RECONSTRUCTION: Bra'tac's status report at 4932 grants the SGC address (gate 27).
-- It co-fires with chain 8292 (dialog 5843, advance to 4930), which stays unchanged. A click grant, never
-- player_loaded (F44). The grant is idempotent: a player who already knows gate 27 (from 1654, or a GM skip)
-- is unaffected.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8313, '1651 - Status report to Bra''tac at 4932: grant the SGC address (gate 27)', 'mission', 1651, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8313, 'interact_tag', 'Dakara_E1_StoryRm_Bratac', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8313, 'world', 62, NULL, 'eq', NULL, 0),
  (8313, 'step_status', 1651, '4932', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8313, 'grant_stargate_address', 27, NULL, '{}', 0, 0);
