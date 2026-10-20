-- NEW CONTENT (minigames campaign, MG-74, D-MG23): the Debug Area minigame
-- terminals of world 1300 `DebugArea` (spawnlist_debug_area_minigames.sql,
-- templates 1600-1605). Chains 24050-24057 (block 24050-24069).
--
--   CrystalGame            (tag DebugArea_Minigame_CrystalGame)            chain 24050
--   Alignment              (tag DebugArea_Minigame_Alignment)              chain 24051
--   GoauldCrystals         (tag DebugArea_Minigame_GoauldCrystals)         chain 24052
--   ConverseBasicHumanoid  (tag DebugArea_Minigame_ConverseBasicHumanoid)  chain 24053
--   Livewire               (tag DebugArea_Minigame_Livewire)               chain 24054
--   Hack (shell)           (tag DebugArea_Minigame_Hack)                   chain 24055
--
-- Result lines: chain 24056 is the win line, 24057 the loss line, both spoken
-- by `npc_bark` through the terminals' bark holder (dialog 60120).
--
-- Reusable GM test fixtures; no mission, no step gate. Every cursor bit is a
-- template default (1600-1605), so there is no `set_interaction_type`. No
-- launcher carries a condition: every click starts its game.

SET search_path = resources, pg_catalog;

-- ============================================================
-- Launchers: click -> start the game; its result fires 24056 or 24057.
-- ============================================================

-- Chain 24050: click the CrystalGame terminal -> start CrystalGame.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (24050, 'Debug Area - minigame terminal: start CrystalGame', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (24050, 'interact_tag', 'DebugArea_Minigame_CrystalGame', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (24050, 'start_minigame', NULL, 'CrystalGame', '{"difficulty": 1, "on_victory_chains": [24056], "on_failure_chains": [24057]}', 0, 0);

-- Chain 24051: click the Alignment terminal -> start Alignment.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (24051, 'Debug Area - minigame terminal: start Alignment', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (24051, 'interact_tag', 'DebugArea_Minigame_Alignment', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (24051, 'start_minigame', NULL, 'Alignment', '{"difficulty": 1, "on_victory_chains": [24056], "on_failure_chains": [24057]}', 0, 0);

-- Chain 24052: click the GoauldCrystals terminal -> start GoauldCrystals.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (24052, 'Debug Area - minigame terminal: start GoauldCrystals', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (24052, 'interact_tag', 'DebugArea_Minigame_GoauldCrystals', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (24052, 'start_minigame', NULL, 'GoauldCrystals', '{"difficulty": 1, "on_victory_chains": [24056], "on_failure_chains": [24057]}', 0, 0);

-- Chain 24053: click the ConverseBasicHumanoid terminal -> start ConverseBasicHumanoid.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (24053, 'Debug Area - minigame terminal: start ConverseBasicHumanoid', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (24053, 'interact_tag', 'DebugArea_Minigame_ConverseBasicHumanoid', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (24053, 'start_minigame', NULL, 'ConverseBasicHumanoid', '{"difficulty": 1, "on_victory_chains": [24056], "on_failure_chains": [24057]}', 0, 0);

-- Chain 24054: click the Livewire terminal -> start Livewire.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (24054, 'Debug Area - minigame terminal: start Livewire', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (24054, 'interact_tag', 'DebugArea_Minigame_Livewire', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (24054, 'start_minigame', NULL, 'Livewire', '{"difficulty": 1, "on_victory_chains": [24056], "on_failure_chains": [24057]}', 0, 0);

-- Chain 24055: click the Hack (shell) terminal -> start Hack.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (24055, 'Debug Area - minigame terminal: start Hack (shell)', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (24055, 'interact_tag', 'DebugArea_Minigame_Hack', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (24055, 'start_minigame', NULL, 'Hack', '{"difficulty": 1, "on_victory_chains": [24056], "on_failure_chains": [24057]}', 0, 0);

-- ============================================================
-- Result lines: the minigame callback invokes these by id (no trigger rows).
-- ============================================================

-- Chain 24056: win -> confirmation line in chat.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (24056, 'Debug Area - minigame terminal: win line', 'space', 1300, true, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (24056, 'npc_bark', NULL, NULL, '{"screen_id": 200020, "speaker": "Terminal", "channel": "say"}', 0, 0);

-- Chain 24057: loss -> confirmation line in chat.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (24057, 'Debug Area - minigame terminal: loss line', 'space', 1300, true, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (24057, 'npc_bark', NULL, NULL, '{"screen_id": 200021, "speaker": "Terminal", "channel": "say"}', 0, 0);
