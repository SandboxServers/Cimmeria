-- NEW CONTENT (Debug Area, DA-08): ring consoles in world 1300 open the
-- destination list of their station. Same shape as Harset's ring switches
-- (harset_space_chains.sql, chains 6001-6005): `interact_tag` on the console's
-- spawn tag -> `trigger_transporter` with the region id.
--
-- Chain ID range: 13800-13807 (DA-08).
--
-- No condition: every click works, for every character. No
-- `set_interaction_type`: template 3 already carries interaction_type 32
-- (INT_RingNetwork) as a template default, so the eight chains are allowlisted
-- in crates/content-engine/tests/it/interact_tag_linter.rs, as Harset's are.
--
-- `params` key is `regionId` (camelCase); the loader falls back to 0 on any
-- other spelling, which would be a silently dead console.
--
-- A right-click opens the destination list; the trip starts when the player
-- picks a destination and steps onto the pad.

SET search_path = resources, pg_catalog;

-- Chain 13800: DebugArea_Ring_Compound -> region 35.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (13800, 'Debug Area - Ring console Compound: open destination list for region 35', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (13800, 'interact_tag', 'DebugArea_Ring_Compound', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (13800, 'trigger_transporter', NULL, NULL, '{"regionId": 35}', 0, 0);

-- Chain 13801: DebugArea_Ring_FactionYard -> region 36.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (13801, 'Debug Area - Ring console Faction yard: open destination list for region 36', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (13801, 'interact_tag', 'DebugArea_Ring_FactionYard', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (13801, 'trigger_transporter', NULL, NULL, '{"regionId": 36}', 0, 0);

-- Chain 13802: DebugArea_Ring_AiSlope -> region 37.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (13802, 'Debug Area - Ring console AI slope: open destination list for region 37', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (13802, 'interact_tag', 'DebugArea_Ring_AiSlope', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (13802, 'trigger_transporter', NULL, NULL, '{"regionId": 37}', 0, 0);

-- Chain 13803: DebugArea_Ring_ArenaRim -> region 38.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (13803, 'Debug Area - Ring console Arena rim: open destination list for region 38', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (13803, 'interact_tag', 'DebugArea_Ring_ArenaRim', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (13803, 'trigger_transporter', NULL, NULL, '{"regionId": 38}', 0, 0);

-- Chain 13804: DebugArea_Ring_ArenaPit -> region 39.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (13804, 'Debug Area - Ring console Arena pit: open destination list for region 39', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (13804, 'interact_tag', 'DebugArea_Ring_ArenaPit', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (13804, 'trigger_transporter', NULL, NULL, '{"regionId": 39}', 0, 0);

-- Chain 13805: DebugArea_Ring_GalleryWest -> region 40.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (13805, 'Debug Area - Ring console Gallery west: open destination list for region 40', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (13805, 'interact_tag', 'DebugArea_Ring_GalleryWest', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (13805, 'trigger_transporter', NULL, NULL, '{"regionId": 40}', 0, 0);

-- Chain 13806: DebugArea_Ring_GalleryEast -> region 41.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (13806, 'Debug Area - Ring console Gallery east: open destination list for region 41', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (13806, 'interact_tag', 'DebugArea_Ring_GalleryEast', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (13806, 'trigger_transporter', NULL, NULL, '{"regionId": 41}', 0, 0);

-- Chain 13807: DebugArea_Ring_DeathYard -> region 42.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (13807, 'Debug Area - Ring console Death yard: open destination list for region 42', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (13807, 'interact_tag', 'DebugArea_Ring_DeathYard', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (13807, 'trigger_transporter', NULL, NULL, '{"regionId": 42}', 0, 0);
