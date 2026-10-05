-- NEW CONTENT (Debug Area, DA-10): the Lineup attendants' chains, world
-- 1300. docs/content/debug-area.md#visual-npc-lineup describes the groups.
--
--   Show Humans                       (tag DebugArea_LineupAttendant_1301) chain 14090
--   Show Jaffa male                   (tag DebugArea_LineupAttendant_1302) chain 14091
--   Show Jaffa female                 (tag DebugArea_LineupAttendant_1303) chain 14092
--   Show Goa'uld, Asgard and children (tag DebugArea_LineupAttendant_1304) chain 14093
--   Show Creatures and machines       (tag DebugArea_LineupAttendant_1305) chain 14094
--   Clear lineup                      (tag DebugArea_LineupAttendant_Clear) chain 14095
--
-- Chain ids are the attendants' spawn ids, inside DA-10's block. The GM gate
-- is in the `spawn_set` action, so a player's click is answered with a
-- refusal line rather than dropped by a condition. No `set_interaction_type`:
-- each cursor bit is the template default (allowlisted in
-- crates/content-engine/tests/it/interact_tag_linter.rs).
--
-- One attendant per button rather than one attendant with a dialog: a custom
-- dialog's buttons need a cooked dialog override, and those crashed the
-- client (#943).

SET search_path = resources, pg_catalog;

-- Chain 14090: click "Show Humans (42)" -> show lineup group 1301 (Humans).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (14090, 'Debug Area - click the Lineup attendant "Show Humans (42)": show lineup group 1301 (Humans) (GM)', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (14090, 'interact_tag', 'DebugArea_LineupAttendant_1301', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (14090, 'spawn_set', NULL, NULL, '{"op": "show", "set_id": 1301}', 0, 0);

-- Chain 14091: click "Show Jaffa male (44)" -> show lineup group 1302 (Jaffa male).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (14091, 'Debug Area - click the Lineup attendant "Show Jaffa male (44)": show lineup group 1302 (Jaffa male) (GM)', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (14091, 'interact_tag', 'DebugArea_LineupAttendant_1302', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (14091, 'spawn_set', NULL, NULL, '{"op": "show", "set_id": 1302}', 0, 0);

-- Chain 14092: click "Show Jaffa female (30)" -> show lineup group 1303 (Jaffa female).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (14092, 'Debug Area - click the Lineup attendant "Show Jaffa female (30)": show lineup group 1303 (Jaffa female) (GM)', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (14092, 'interact_tag', 'DebugArea_LineupAttendant_1303', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (14092, 'spawn_set', NULL, NULL, '{"op": "show", "set_id": 1303}', 0, 0);

-- Chain 14093: click "Show Goa'uld, Asgard and children (28)" -> show lineup group 1304 (Goa'uld, Asgard and children).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (14093, 'Debug Area - click the Lineup attendant "Show Goa''uld, Asgard and children (28)": show lineup group 1304 (Goa''uld, Asgard and children) (GM)', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (14093, 'interact_tag', 'DebugArea_LineupAttendant_1304', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (14093, 'spawn_set', NULL, NULL, '{"op": "show", "set_id": 1304}', 0, 0);

-- Chain 14094: click "Show Creatures and machines (17)" -> show lineup group 1305 (Creatures and machines).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (14094, 'Debug Area - click the Lineup attendant "Show Creatures and machines (17)": show lineup group 1305 (Creatures and machines) (GM)', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (14094, 'interact_tag', 'DebugArea_LineupAttendant_1305', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (14094, 'spawn_set', NULL, NULL, '{"op": "show", "set_id": 1305}', 0, 0);

-- Chain 14095: click "Clear lineup" -> switch the lineup off.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (14095, 'Debug Area - click the Lineup attendant "Clear lineup": switch the lineup off (GM)', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (14095, 'interact_tag', 'DebugArea_LineupAttendant_Clear', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (14095, 'spawn_set', NULL, NULL, '{"op": "clear", "kind": "visual_lineup", "world_id": 1300}', 0, 0);
