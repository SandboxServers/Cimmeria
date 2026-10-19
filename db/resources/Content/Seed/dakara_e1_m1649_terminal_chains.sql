-- Dakara_E1 rebuild: mission 1649 "Moh'katan", step 4922 (the Command Terminal and the drops).
-- Campaign: docs/analysis/dakara-e1-rebuild/ (packet DK-14b).
-- Chain ID range: 8220-8249 (work-packets.md, C3). Evidence labels: README.md, "Evidence Labels".

SET search_path = resources, pg_catalog;

-- Step 4922 "Investigate Moh'katan's behavior" has three required objectives:
-- 5638 (hack the terminal), 5639 (find the coded message in the drops) and
-- 5640 (decode the orders). 5640 is never completed by hand: the advance to
-- 4920 in chain 8224 completes it. So neither 5638 nor 5639 is ever the
-- step's last open objective (C5 rule 3).
--
-- The props are clickable only through the carrier dialog set 6760 (D-DK31):
-- 6760 on slot 453 is the Command Terminal (template 453), and 6760 on slots
-- 449, 458-461 are the five drops (Dakara_E1_Drop1 to Drop5). The carrier is
-- bound on entry (8220-8222) and unbound when its job is done. Drop 5 holds
-- the coded message (RECONSTRUCTION: the client names no drop); drops 1 to 4
-- say it was taken (8227-8230). The rule that the drops stay bound only until
-- the message is found is 8221 (bind) and 8226 (unbind).
--
-- Chains 8223, 8224 and 8225 are the terminal's three states, exclusive by
-- objectives 5638 and 5639 (8223: 5638 open; 8225: 5638 done, 5639 open;
-- 8224: both done). Mission 1649 offer and radio chains are DK-14a's
-- (8200-8219), and the 4921 to 4922 advance in 8205 and 8222 is DK-14a's
-- co-fire by design.

-- Chain 8220 PROJECT_FINAL_RECONSTRUCTION: 1649 step 4922, entering Dakara_E1: bind the terminal's carrier 6760 to slot 453 (on entry and after a relog).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8220, '1649 - 4922: bind terminal carrier on Dakara_E1', 'mission', 1649, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8220, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8220, 'world', 61, NULL, 'eq', NULL, 0),
  (8220, 'step_status', 1649, '4922', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8220, 'add_dialog_set', 6760, NULL, '{"slot": 453}', 0, 0);

-- Chain 8221 RECONSTRUCTION: 1649 step 4922, entering Dakara_E1 with the message not yet found: bind carrier 6760 to the five drop slots (449, 458-461).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8221, '1649 - 4922: bind drop carriers on Dakara_E1 until the message is found', 'mission', 1649, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8221, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8221, 'world', 61, NULL, 'eq', NULL, 0),
  (8221, 'step_status', 1649, '4922', 'eq', 'active', 1),
  (8221, 'objective_status', 1649, '5639', 'neq', 'completed', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8221, 'add_dialog_set', 6760, NULL, '{"slot": 449}', 0, 0),
  (8221, 'add_dialog_set', 6760, NULL, '{"slot": 458}', 0, 1),
  (8221, 'add_dialog_set', 6760, NULL, '{"slot": 459}', 0, 2),
  (8221, 'add_dialog_set', 6760, NULL, '{"slot": 460}', 0, 3),
  (8221, 'add_dialog_set', 6760, NULL, '{"slot": 461}', 0, 4);

-- Chain 8222 PROJECT_FINAL_RECONSTRUCTION: 1649 step 4921 on entering Dakara_E1 (the player left before closing 5832): bind the terminal and the five drops. Co-fires with DK-14a's 8205, which advances 4921 to 4922 in this event; the conditions are read before it runs.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8222, '1649 - 4921: bind terminal and drop carriers on Dakara_E1', 'mission', 1649, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8222, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8222, 'world', 61, NULL, 'eq', NULL, 0),
  (8222, 'step_status', 1649, '4921', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8222, 'add_dialog_set', 6760, NULL, '{"slot": 453}', 0, 0),
  (8222, 'add_dialog_set', 6760, NULL, '{"slot": 449}', 0, 1),
  (8222, 'add_dialog_set', 6760, NULL, '{"slot": 458}', 0, 2),
  (8222, 'add_dialog_set', 6760, NULL, '{"slot": 459}', 0, 3),
  (8222, 'add_dialog_set', 6760, NULL, '{"slot": 460}', 0, 4),
  (8222, 'add_dialog_set', 6760, NULL, '{"slot": 461}', 0, 5);

-- Chain 8223 PROJECT_FINAL_RECONSTRUCTION: 1649 step 4922, terminal, objective 5638 open: the hack. Display 5834, give the copy of the orders 6782, complete 5638.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8223, '1649 - 4922: terminal hack, download the orders', 'mission', 1649, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8223, 'interact_tag', 'Dakara_E1_CommandTerminal', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8223, 'world', 61, NULL, 'eq', NULL, 0),
  (8223, 'step_status', 1649, '4922', 'eq', 'active', 1),
  (8223, 'objective_status', 1649, '5638', 'neq', 'completed', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8223, 'display_dialog', 5834, NULL, '{}', 0, 0),
  (8223, 'add_item', 6782, NULL, '{"container": 0, "qty": 1}', 0, 1),
  (8223, 'complete_objective', 1649, '5638', '{}', 0, 2);

-- Chain 8224 PROJECT_FINAL_RECONSTRUCTION: 1649 step 4922, terminal, both 5638 and 5639 done: decode the message. Display 5837, take the encrypted message 6780, give the decoded message 6783, advance 4922 to 4920 (completes 5640), unbind the terminal carrier.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8224, '1649 - 4922: terminal decode, leave for 4920', 'mission', 1649, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8224, 'interact_tag', 'Dakara_E1_CommandTerminal', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8224, 'world', 61, NULL, 'eq', NULL, 0),
  (8224, 'step_status', 1649, '4922', 'eq', 'active', 1),
  (8224, 'objective_status', 1649, '5638', 'eq', 'completed', 2),
  (8224, 'objective_status', 1649, '5639', 'eq', 'completed', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8224, 'display_dialog', 5837, NULL, '{}', 0, 0),
  (8224, 'remove_item', 6780, NULL, '{"qty": 1}', 0, 1),
  (8224, 'add_item', 6783, NULL, '{"container": 0, "qty": 1}', 0, 2),
  (8224, 'advance_step', 1649, '4920', '{}', 0, 3),
  (8224, 'remove_dialog_set', 6760, NULL, '{"slot": 453}', 0, 4);

-- Chain 8225 PROJECT_FINAL_RECONSTRUCTION: 1649 step 4922, terminal, 5638 done and 5639 open: feedback only, the orders again, nothing to decode yet.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8225, '1649 - 4922: terminal, orders again before the message is found', 'mission', 1649, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8225, 'interact_tag', 'Dakara_E1_CommandTerminal', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8225, 'world', 61, NULL, 'eq', NULL, 0),
  (8225, 'step_status', 1649, '4922', 'eq', 'active', 1),
  (8225, 'objective_status', 1649, '5638', 'eq', 'completed', 2),
  (8225, 'objective_status', 1649, '5639', 'neq', 'completed', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8225, 'display_dialog', 5834, NULL, '{}', 0, 0);

-- Chain 8226 RECONSTRUCTION: 1649 step 4922, drop 5 (holds the coded message), 5639 open: display 5835, take the message 6780, complete 5639, unbind the five drop carriers.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8226, '1649 - 4922: drop 5, the coded message', 'mission', 1649, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8226, 'interact_tag', 'Dakara_E1_Drop5', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8226, 'world', 61, NULL, 'eq', NULL, 0),
  (8226, 'step_status', 1649, '4922', 'eq', 'active', 1),
  (8226, 'objective_status', 1649, '5639', 'neq', 'completed', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8226, 'display_dialog', 5835, NULL, '{}', 0, 0),
  (8226, 'add_item', 6780, NULL, '{"container": 0, "qty": 1}', 0, 1),
  (8226, 'complete_objective', 1649, '5639', '{}', 0, 2),
  (8226, 'remove_dialog_set', 6760, NULL, '{"slot": 449}', 0, 3),
  (8226, 'remove_dialog_set', 6760, NULL, '{"slot": 458}', 0, 4),
  (8226, 'remove_dialog_set', 6760, NULL, '{"slot": 459}', 0, 5),
  (8226, 'remove_dialog_set', 6760, NULL, '{"slot": 460}', 0, 6),
  (8226, 'remove_dialog_set', 6760, NULL, '{"slot": 461}', 0, 7);

-- Chain 8227 PROJECT_FINAL_RECONSTRUCTION: 1649 step 4922, drop 1, message already taken: display 5836.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8227, '1649 - 4922: drop 1, already taken', 'mission', 1649, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8227, 'interact_tag', 'Dakara_E1_Drop1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8227, 'world', 61, NULL, 'eq', NULL, 0),
  (8227, 'step_status', 1649, '4922', 'eq', 'active', 1),
  (8227, 'objective_status', 1649, '5639', 'neq', 'completed', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8227, 'display_dialog', 5836, NULL, '{}', 0, 0);

-- Chain 8228 PROJECT_FINAL_RECONSTRUCTION: 1649 step 4922, drop 2, message already taken: display 5836.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8228, '1649 - 4922: drop 2, already taken', 'mission', 1649, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8228, 'interact_tag', 'Dakara_E1_Drop2', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8228, 'world', 61, NULL, 'eq', NULL, 0),
  (8228, 'step_status', 1649, '4922', 'eq', 'active', 1),
  (8228, 'objective_status', 1649, '5639', 'neq', 'completed', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8228, 'display_dialog', 5836, NULL, '{}', 0, 0);

-- Chain 8229 PROJECT_FINAL_RECONSTRUCTION: 1649 step 4922, drop 3, message already taken: display 5836.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8229, '1649 - 4922: drop 3, already taken', 'mission', 1649, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8229, 'interact_tag', 'Dakara_E1_Drop3', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8229, 'world', 61, NULL, 'eq', NULL, 0),
  (8229, 'step_status', 1649, '4922', 'eq', 'active', 1),
  (8229, 'objective_status', 1649, '5639', 'neq', 'completed', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8229, 'display_dialog', 5836, NULL, '{}', 0, 0);

-- Chain 8230 PROJECT_FINAL_RECONSTRUCTION: 1649 step 4922, drop 4, message already taken: display 5836.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8230, '1649 - 4922: drop 4, already taken', 'mission', 1649, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8230, 'interact_tag', 'Dakara_E1_Drop4', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8230, 'world', 61, NULL, 'eq', NULL, 0),
  (8230, 'step_status', 1649, '4922', 'eq', 'active', 1),
  (8230, 'objective_status', 1649, '5639', 'neq', 'completed', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8230, 'display_dialog', 5836, NULL, '{}', 0, 0);
