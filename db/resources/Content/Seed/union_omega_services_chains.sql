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
-- CIMMERIA_RECONSTRUCTION: copy of chain 13008 on Omega's tag (D-UO36)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (63850, 'Union Omega: the Auction House auctioneer opens the Black Market', 'space', 18, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63850, 'interact_tag', 'UnionOmega_Auctioneer', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (63850, 'open_black_market', NULL, NULL, '{}', 0, 0);
-- [UO-32] end

-- [UO-33] begin: medic chain 63851
-- [UO-33] end

-- [UO-35] begin: guide chains 63852-63855
-- [UO-35] end

-- [UO-36] begin: inn and battlemaster chains 63856-63857
-- [UO-36] end
