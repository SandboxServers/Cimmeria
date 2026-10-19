-- Beta Site E1 (world 23, Beta_Site_Evo_1) - 381 Closing Shop (UB-11).
-- Plan: docs/analysis/union-zone-planning/beta-work-packets.md. Block 23000-23499.
-- Chains use only per-player binds (D-UB10) and no phasing (D-UB11).
SET search_path = resources, pg_catalog;

-- Provenance: RESTORED_SOURCE for the step ids (S1408, S1409, S1410), the
-- dialogs (1882 offer, 1884 cages, 1883 turn-in, 1886 blurb is never shown),
-- the dsm ids (2305 blurb, 2306 welcome back, 2304 cages, 2302 turn-in), the
-- items (2636 checklist, 3635 cage) and the cage tags. Cage template 2320 is
-- RESTORED_SOURCE for the placed objects (UB-02a).
-- CIMMERIA_RECONSTRUCTION for the chain shapes and the counter quota (P9).
-- CIMMERIA_AUTHORING for the Human order after 1423 (HELD-B1, mission-seal-held.md).
-- Guards: `chain_replay_tests/beta_e1/m381.rs`.
-- The seal (docs/analysis/union-beta-e1-continuation/mission-seal.md, 381 table)
-- applies C1 (no `pick:1886` row on chain 23042) and C2 (S1410 is never entered,
-- so the turn-in completes 381 from S1409; labelled CIMMERIA_RECONSTRUCTION).

-- Chain 23040: login or completing 1423, Human, 1423 completed, 381 not_active
-- -> bind Carter's blurb dsm 2305 (the offer bind, P2). Two trigger rows: a
-- player who finished 1423 this session gets no second `player_loaded`.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23040, 'Beta_Site_Evo_1 - Human after 1423: bind Carter''s 381 offer (UB-11)', 'mission', 381, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23040, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23040, 'mission_completed', '1423', 'player', false, 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23040, 'archetype', NULL, NULL, 'gte', '1', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23040, 'archetype', NULL, NULL, 'lte', '4', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23040, 'mission_status', 1423, NULL, 'eq', 'completed', 2);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23040, 'mission_status', 381, NULL, 'eq', 'not_active', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23040, 'add_dialog_set', 2305, NULL, '{"slot": 2300, "mission_id": 381}', 0, 0);

-- Chain 23041: interact_tag BetaE1_Carter, Human, 1423 completed, 381 not_active
-- -> show the offer dialog 1882 (P2 offer show). Dialog 1882 is the offer
-- screen with Accept (8) and More Info; its accept is chain 23042.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23041, 'Beta_Site_Evo_1 - Carter offers 381: show 1882 (UB-11)', 'mission', 381, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23041, 'interact_tag', 'BetaE1_Carter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23041, 'archetype', NULL, NULL, 'gte', '1', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23041, 'archetype', NULL, NULL, 'lte', '4', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23041, 'mission_status', 1423, NULL, 'eq', 'completed', 2);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23041, 'mission_status', 381, NULL, 'eq', 'not_active', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23041, 'display_dialog', 1882, NULL, '{}', 0, 0);

-- Chain 23042: dialog_choice 1882 (the Accept on the offer), Human, 1423 completed,
-- 381 not_active -> accept 381, unbind the blurb, give Carter's checklist 2636,
-- bind Felger's cage talk 2304. Only `pick:1882`: dialog 1886 is never displayed
-- (the tag chain 23041 shows 1882 and suppresses the bind's auto-display), so a
-- `pick:1886` row here would be dead (seal C1).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23042, 'Beta_Site_Evo_1 - accept 381 on dialog 1882 (UB-11)', 'mission', 381, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23042, 'dialog_choice', '1882', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23042, 'archetype', NULL, NULL, 'gte', '1', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23042, 'archetype', NULL, NULL, 'lte', '4', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23042, 'mission_status', 1423, NULL, 'eq', 'completed', 2);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23042, 'mission_status', 381, NULL, 'eq', 'not_active', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23042, 'accept_mission', 381, NULL, '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23042, 'remove_dialog_set', 2305, NULL, '{"slot": 2300}', 0, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23042, 'add_item', 2636, NULL, '{"container": 0, "qty": 1}', 0, 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23042, 'add_dialog_set', 2304, NULL, '{"slot": 2301, "mission_id": 381}', 0, 3);

-- Chain 23043: player_loaded Beta_Site_Evo_1, Human, S1408 active -> re-bind
-- Felger's cage talk (restore; binds are not persisted, P1).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23043, 'Beta_Site_Evo_1 - Human relog on S1408: re-bind Felger (UB-11)', 'mission', 381, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23043, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23043, 'archetype', NULL, NULL, 'gte', '1', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23043, 'archetype', NULL, NULL, 'lte', '4', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23043, 'step_status', 381, '1408', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23043, 'add_dialog_set', 2304, NULL, '{"slot": 2301, "mission_id": 381}', 0, 0);

-- Chain 23044: interact_tag BetaE1_Felger, Human, S1408 active -> show the cage
-- dialog 1884, unbind Felger's S1408 talk, take the checklist 2636 back, advance
-- to S1409, bind the turn-in talk 2302 (P4 mid-step talk).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23044, 'Beta_Site_Evo_1 - Felger takes the checklist: S1408 to S1409 (UB-11)', 'mission', 381, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23044, 'interact_tag', 'BetaE1_Felger', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23044, 'archetype', NULL, NULL, 'gte', '1', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23044, 'archetype', NULL, NULL, 'lte', '4', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23044, 'step_status', 381, '1408', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23044, 'display_dialog', 1884, NULL, '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23044, 'remove_dialog_set', 2304, NULL, '{"slot": 2301}', 0, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23044, 'remove_item', 2636, NULL, '{"qty": 1}', 0, 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23044, 'advance_step', 381, '1409', '{}', 0, 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23044, 'add_dialog_set', 2302, NULL, '{"slot": 2301, "mission_id": 381}', 0, 4);

-- Chains 23045-23049: interact_tag on each cage (BetaE1_XenoCage_1 .. _5), Human,
-- S1409 active -> one cage into the inventory and one count on the quota (P9).
-- `once` is true, so a cage gives one cage per player. Object tags carry the
-- static INT_MissionWorldObject from template 2320 (UB-02a). Cage n uses
-- chain 23044 + n.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23045, 'Beta_Site_Evo_1 - cage 1 collected (UB-11)', 'mission', 381, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23045, 'interact_tag', 'BetaE1_XenoCage_1', 'player', true, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23045, 'archetype', NULL, NULL, 'gte', '1', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23045, 'archetype', NULL, NULL, 'lte', '4', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23045, 'step_status', 381, '1409', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23045, 'add_item', 3635, NULL, '{"container": 0, "qty": 1}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23045, 'increment_counter', NULL, 'beta_381_cages', '{"amount": 1}', 0, 1);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23046, 'Beta_Site_Evo_1 - cage 2 collected (UB-11)', 'mission', 381, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23046, 'interact_tag', 'BetaE1_XenoCage_2', 'player', true, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23046, 'archetype', NULL, NULL, 'gte', '1', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23046, 'archetype', NULL, NULL, 'lte', '4', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23046, 'step_status', 381, '1409', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23046, 'add_item', 3635, NULL, '{"container": 0, "qty": 1}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23046, 'increment_counter', NULL, 'beta_381_cages', '{"amount": 1}', 0, 1);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23047, 'Beta_Site_Evo_1 - cage 3 collected (UB-11)', 'mission', 381, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23047, 'interact_tag', 'BetaE1_XenoCage_3', 'player', true, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23047, 'archetype', NULL, NULL, 'gte', '1', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23047, 'archetype', NULL, NULL, 'lte', '4', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23047, 'step_status', 381, '1409', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23047, 'add_item', 3635, NULL, '{"container": 0, "qty": 1}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23047, 'increment_counter', NULL, 'beta_381_cages', '{"amount": 1}', 0, 1);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23048, 'Beta_Site_Evo_1 - cage 4 collected (UB-11)', 'mission', 381, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23048, 'interact_tag', 'BetaE1_XenoCage_4', 'player', true, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23048, 'archetype', NULL, NULL, 'gte', '1', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23048, 'archetype', NULL, NULL, 'lte', '4', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23048, 'step_status', 381, '1409', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23048, 'add_item', 3635, NULL, '{"container": 0, "qty": 1}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23048, 'increment_counter', NULL, 'beta_381_cages', '{"amount": 1}', 0, 1);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23049, 'Beta_Site_Evo_1 - cage 5 collected (UB-11)', 'mission', 381, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23049, 'interact_tag', 'BetaE1_XenoCage_5', 'player', true, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23049, 'archetype', NULL, NULL, 'gte', '1', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23049, 'archetype', NULL, NULL, 'lte', '4', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23049, 'step_status', 381, '1409', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23049, 'add_item', 3635, NULL, '{"container": 0, "qty": 1}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23049, 'increment_counter', NULL, 'beta_381_cages', '{"amount": 1}', 0, 1);

-- Chain 23050: player_loaded Beta_Site_Evo_1, Human, S1409 active -> re-bind
-- Felger's turn-in talk (restore, P1 style; binds are not persisted).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23050, 'Beta_Site_Evo_1 - Human relog on S1409: re-bind Felger turn-in (UB-11)', 'mission', 381, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23050, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23050, 'archetype', NULL, NULL, 'gte', '1', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23050, 'archetype', NULL, NULL, 'lte', '4', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23050, 'step_status', 381, '1409', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23050, 'add_dialog_set', 2302, NULL, '{"slot": 2301, "mission_id": 381}', 0, 0);

-- Chain 23051: interact_tag BetaE1_Felger, Human, S1409 active, counter
-- beta_381_cages gte 5 -> the turn-in 1883, unbind it, hand in the five cages,
-- complete 381 (P3 + P9). Seal C2: S1410 is never entered, so the turn-in
-- completes 381 from S1409 (CIMMERIA_RECONSTRUCTION). Use `complete_mission`,
-- not a last `complete_objective` (F19).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23051, 'Beta_Site_Evo_1 - Felger takes five cages: complete 381 (UB-11)', 'mission', 381, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23051, 'interact_tag', 'BetaE1_Felger', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23051, 'archetype', NULL, NULL, 'gte', '1', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23051, 'archetype', NULL, NULL, 'lte', '4', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23051, 'step_status', 381, '1409', 'eq', 'active', 2);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23051, 'counter', NULL, 'beta_381_cages', 'gte', '5', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23051, 'display_dialog', 1883, NULL, '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23051, 'remove_dialog_set', 2302, NULL, '{"slot": 2301}', 0, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23051, 'remove_item', 3635, NULL, '{"qty": 5}', 0, 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23051, 'complete_mission', 381, NULL, '{}', 0, 3);

-- Chain 23052: interact_tag BetaE1_Felger, Human, S1409 active, counter lt 5 ->
-- show the cage dialog 1884 again. The "not yet" sibling of 23051 (F21), so the
-- first press always answers.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23052, 'Beta_Site_Evo_1 - Felger, fewer than five cages: show 1884 (UB-11)', 'mission', 381, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23052, 'interact_tag', 'BetaE1_Felger', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23052, 'archetype', NULL, NULL, 'gte', '1', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23052, 'archetype', NULL, NULL, 'lte', '4', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23052, 'step_status', 381, '1409', 'eq', 'active', 2);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23052, 'counter', NULL, 'beta_381_cages', 'lt', '5', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23052, 'display_dialog', 1884, NULL, '{}', 0, 0);

-- Chain 23053: player_loaded Beta_Site_Evo_1, Human, S1409 active -> resync the
-- cages after a relog (P9 login resync, UB-00a1 Q5). Counters reset on relog, so
-- the cages in the inventory are removed with them. Keeps the first stack only
-- (accepted gap, UB-U7b).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23053, 'Beta_Site_Evo_1 - Human relog on S1409: resync the cages (UB-11)', 'mission', 381, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23053, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23053, 'archetype', NULL, NULL, 'gte', '1', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23053, 'archetype', NULL, NULL, 'lte', '4', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23053, 'step_status', 381, '1409', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23053, 'remove_item', 3635, NULL, '{"qty": 5}', 0, 0);
