-- NEW CONTENT (Debug Area, DA-08): ring consoles in world 1300 open the
-- destination list of their station. Same shape as Harset's ring switches
-- (harset_space_chains.sql, chains 6001-6005): `interact_tag` on the console's
-- spawn tag -> `trigger_transporter` with the region id.
--
-- Chain ID range: 13810-13817 (DA-08), 13818 (DA-11).
--
-- No condition: every click works, for every character. No
-- `set_interaction_type`: template 3 already carries interaction_type 32
-- (INT_RingNetwork) as a template default, so the nine chains are allowlisted
-- in crates/content-engine/tests/it/interact_tag_linter.rs, as Harset's are.
--
-- `params` key is `regionId` (camelCase); the loader falls back to 0 on any
-- other spelling, which would be a silently dead console.
--
-- A right-click opens the destination list; the trip starts when the player
-- picks a destination and steps onto the pad.

SET search_path = resources, pg_catalog;

-- Chain 13810: DebugArea_Ring_Compound -> region 35.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (13810, 'Debug Area - Ring console Compound: open destination list for region 35', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (13810, 'interact_tag', 'DebugArea_Ring_Compound', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (13810, 'trigger_transporter', NULL, NULL, '{"regionId": 35}', 0, 0);

-- Chain 13811: DebugArea_Ring_FactionYard -> region 36.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (13811, 'Debug Area - Ring console Faction yard: open destination list for region 36', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (13811, 'interact_tag', 'DebugArea_Ring_FactionYard', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (13811, 'trigger_transporter', NULL, NULL, '{"regionId": 36}', 0, 0);

-- Chain 13812: DebugArea_Ring_AiSlope -> region 37.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (13812, 'Debug Area - Ring console AI slope: open destination list for region 37', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (13812, 'interact_tag', 'DebugArea_Ring_AiSlope', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (13812, 'trigger_transporter', NULL, NULL, '{"regionId": 37}', 0, 0);

-- Chain 13813: DebugArea_Ring_ArenaRim -> region 38.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (13813, 'Debug Area - Ring console Arena rim: open destination list for region 38', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (13813, 'interact_tag', 'DebugArea_Ring_ArenaRim', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (13813, 'trigger_transporter', NULL, NULL, '{"regionId": 38}', 0, 0);

-- Chain 13814: DebugArea_Ring_ArenaPit -> region 39.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (13814, 'Debug Area - Ring console Arena pit: open destination list for region 39', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (13814, 'interact_tag', 'DebugArea_Ring_ArenaPit', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (13814, 'trigger_transporter', NULL, NULL, '{"regionId": 39}', 0, 0);

-- Chain 13815: DebugArea_Ring_GalleryWest -> region 40.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (13815, 'Debug Area - Ring console Gallery west: open destination list for region 40', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (13815, 'interact_tag', 'DebugArea_Ring_GalleryWest', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (13815, 'trigger_transporter', NULL, NULL, '{"regionId": 40}', 0, 0);

-- Chain 13816: DebugArea_Ring_GalleryEast -> region 41.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (13816, 'Debug Area - Ring console Gallery east: open destination list for region 41', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (13816, 'interact_tag', 'DebugArea_Ring_GalleryEast', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (13816, 'trigger_transporter', NULL, NULL, '{"regionId": 41}', 0, 0);

-- Chain 13817: DebugArea_Ring_DeathYard -> region 42.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (13817, 'Debug Area - Ring console Death yard: open destination list for region 42', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (13817, 'interact_tag', 'DebugArea_Ring_DeathYard', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (13817, 'trigger_transporter', NULL, NULL, '{"regionId": 42}', 0, 0);

-- Chain 13818: DebugArea_Ring_Lineup -> region 43 (DA-11).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (13818, 'Debug Area - Ring console Lineup: open destination list for region 43', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (13818, 'interact_tag', 'DebugArea_Ring_Lineup', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (13818, 'trigger_transporter', NULL, NULL, '{"regionId": 43}', 0, 0);
