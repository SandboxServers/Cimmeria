--
-- Union Omega (worlds 18 `Omega_Site` and 80 `Omega_Site_CmdCenter`): hub
-- service chains (chain ids 63850-63869, hub services). Campaign:
-- docs/analysis/union-zone-planning/omega-work-packets.md (packets UO-01 to UO-42).
--
-- Omega's id block is 63800-63999; this file's chain ids are 63850-63869. Each
-- packet writes only below its own marker. The service chains key on
-- interact_tag; union_omega.rs in the interact_tag linter lists them.
-- Provenance: each block states its own tag (RESTORED_SOURCE,
-- CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING).
--

SET search_path = resources, pg_catalog;

-- [UO-32] begin: auctioneer chain 63850
-- [UO-32] end

-- [UO-33] begin: medic chain 63851
-- [UO-33] end

-- [UO-35] begin: guide chains 63852-63855
-- CIMMERIA_RECONSTRUCTION: crafting guides from shipped verbs (D-UO31); no reward (D-UO37)
-- Two interact_tag triggers per chain (plain spot, Adv. spot): spawnlist.tag is UNIQUE (UO-35 ruling)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (63852, 'Union Omega: the Biology guides name the Biomedical vendors', 'space', 18, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63852, 'interact_tag', 'UnionOmega_Guide_Biology', 'player', false, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63852, 'interact_tag', 'UnionOmega_Guide_AdvBiology', 'player', false, 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (63852, 'npc_bark', NULL, NULL, '{"screen_id": 63901, "speaker": "SGC Scientist", "channel": "say"}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (63853, 'Union Omega: the Electronics guides name the Electronics vendors', 'space', 18, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63853, 'interact_tag', 'UnionOmega_Guide_Electronics', 'player', false, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63853, 'interact_tag', 'UnionOmega_Guide_AdvElectronics', 'player', false, 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (63853, 'npc_bark', NULL, NULL, '{"screen_id": 63902, "speaker": "SGC Scientist", "channel": "say"}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (63854, 'Union Omega: the Engineering guides name the Power Systems vendors', 'space', 18, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63854, 'interact_tag', 'UnionOmega_Guide_Engineering', 'player', false, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63854, 'interact_tag', 'UnionOmega_Guide_AdvEngineering', 'player', false, 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (63854, 'npc_bark', NULL, NULL, '{"screen_id": 63903, "speaker": "SGC Scientist", "channel": "say"}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (63855, 'Union Omega: the Physics guides name the Materials vendors', 'space', 18, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63855, 'interact_tag', 'UnionOmega_Guide_Physics', 'player', false, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63855, 'interact_tag', 'UnionOmega_Guide_AdvPhysics', 'player', false, 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (63855, 'npc_bark', NULL, NULL, '{"screen_id": 63904, "speaker": "SGC Scientist", "channel": "say"}', 0, 0);
-- [UO-35] end

-- [UO-36] begin: inn and battlemaster chains 63856-63857
-- [UO-36] end
