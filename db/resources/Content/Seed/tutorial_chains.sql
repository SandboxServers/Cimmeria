-- PROJECT_FINAL_S2C (Class Start v6, CS-03): global one-time tutorial chains.
-- docs/analysis/class-start-v6/README.md is the ledger.
--
-- Chain ID range: 7101-7119 is reserved for global tutorial chains. The
-- debug hub owns 7001-7099.
--
-- A tutorial is shown with `show_tutorial {"tutorial_id": <dialog id>}`: the
-- base records (player_id, tutorial_id) in sgw_player_tutorials and the
-- dialog is displayed only on the first record, so a replay, a relog or a
-- world change never shows it again. `tutorial_shown` (target_id = dialog id,
-- operator eq = seen, neq = not seen) gates on the same persisted set.
--
-- 5882 "Equipping a Weapon" is NOT seeded here: CS-04 (M622, Castle
-- CellBlock) and CS-05 (M1559 FirearmBody, SGC) add it inside the chains
-- that grant the first pistol, after the CORE_TUTORIAL grant_ability row.

SET search_path = resources, pg_catalog;

-- Chain 7101: the first hostile combat after "Equipping a Weapon" shows
-- 5883 "Combat" (DUIST_DefaultTutorial, 7 screens), once per character.
-- Gated on 5882 having been shown, which is what restricts it to the
-- Human and Loyalist Jaffa starts that get 5882.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (7101, 'Tutorial - first hostile combat after Equipping a Weapon: show 5883 Combat', 'global', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (7101, 'player_entered_combat', NULL, 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (7101, 'tutorial_shown', 5882, NULL, 'eq', NULL, 0),
       (7101, 'tutorial_shown', 5883, NULL, 'neq', NULL, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (7101, 'show_tutorial', NULL, NULL, '{"tutorial_id": 5883}', 0, 0);
