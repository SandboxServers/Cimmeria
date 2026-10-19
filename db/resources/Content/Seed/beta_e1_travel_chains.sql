-- Beta Site E1 (world 23, Beta_Site_Evo_1) - travel, way back (UB-01b).
-- Plan: docs/analysis/union-zone-planning/beta-work-packets.md. Block 23000-23499.
-- Chains use only per-player binds (D-UB10) and no phasing (D-UB11).
SET search_path = resources, pg_catalog;

-- Provenance: CIMMERIA_RECONSTRUCTION. Precedent: DK-01 chain 8001
-- (dakara_e1_space_chains.sql), the Free Jaffa arrival on Dakara_E1 that
-- learns the Omega Site address. Omega Site is stargates.stargate_id 5
-- (world 18), so a Union player who gates to Beta can dial home.
-- Every Union race learns it; Goa'uld (6) and Jaffa (8) are Praxis and do
-- not. `grant_stargate_address` is a no-op for an address already held, so
-- a relog re-grants nothing.
-- Guards: `chain_replay_tests/beta_e1/travel.rs` and the way-back guard
-- `live_db_every_address_a_player_loaded_chain_grants_has_a_way_back`.

-- Chain 23001: player_loaded Beta_Site_Evo_1, archetype 1-4 (Human) -> grant Omega Site.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23001, 'Beta_Site_Evo_1 - Human arrival: learn the Omega Site address (UB-01b)', 'space', 23, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23001, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23001, 'archetype', NULL, NULL, 'gte', '1', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23001, 'archetype', NULL, NULL, 'lte', '4', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23001, 'grant_stargate_address', 5, NULL, '{}', 0, 0);

-- Chain 23002: player_loaded Beta_Site_Evo_1, archetype 5 (Asgard) -> grant Omega Site.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23002, 'Beta_Site_Evo_1 - Asgard arrival: learn the Omega Site address (UB-01b)', 'space', 23, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23002, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23002, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23002, 'grant_stargate_address', 5, NULL, '{}', 0, 0);

-- Chain 23003: player_loaded Beta_Site_Evo_1, archetype 7 (Free Jaffa) -> grant Omega Site.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23003, 'Beta_Site_Evo_1 - Free Jaffa arrival: learn the Omega Site address (UB-01b)', 'space', 23, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23003, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23003, 'archetype', NULL, NULL, 'eq', '7', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23003, 'grant_stargate_address', 5, NULL, '{}', 0, 0);
