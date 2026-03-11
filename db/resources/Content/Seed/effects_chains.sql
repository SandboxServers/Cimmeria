-- Phase 2: Effect script content chains
-- Exercises: effect_init, effect_pulse_begin, effect_pulse_end, effect_removed
--            qr_combat_damage, change_stat, apply_effect, remove_effect,
--            abandon_mission, fail_objective
--
-- Effect IDs used:
--   RangedEnergyDamage → resolved by Atrea framework; chain scope_id = effect DB ID
--   Reload             → scope_id = reload effect DB ID
--   TestEffect         → scope_id = test effect DB ID
--
-- Because we do not know the exact effect DB IDs at seed time (they vary by
-- server load order), we use scope_type='effect' with scope_id = NULL to
-- match ALL instances of the effect class.  In practice, each effect script
-- class maps to exactly one effect definition, so collisions are impossible.
-- When the engine loads chains with scope_id IS NULL and scope_type='effect',
-- it routes via the effect_id passed in on_effect_event().
--
-- Chain ID ranges:
--   RangedEnergyDamage:  2001-2010
--   Reload:              2011-2020
--   TestEffect:          2021-2050

SET search_path = resources, pg_catalog;

-- ============================================================
-- RangedEnergyDamage effect
-- ============================================================

-- Chain 2001: effect_pulse_begin → deal health damage (stat 7) + focus damage (stat 8)
--   Params are read from effect NVPs via action params; engine reads them from
--   the effect's params dict at action execution time.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (2001, 'RangedEnergyDamage - pulse begin: deal H+F damage', 'effect', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (2001, 'effect_pulse_begin', NULL, 'player', false, 0);

-- No conditions: always execute when this effect fires
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  -- stat_id 7=health, source_id 15=ranged energy, amount from NVP 'HealthDamage'
  (2001, 'qr_combat_damage', NULL, NULL, '{"stat_id": 7, "source_id": 15, "amount_nvp": "HealthDamage"}', 0, 0),
  -- stat_id 8=focus, source_id 15=ranged energy, amount from NVP 'FocusDamage'
  (2001, 'qr_combat_damage', NULL, NULL, '{"stat_id": 8, "source_id": 15, "amount_nvp": "FocusDamage"}', 0, 1);

-- ============================================================
-- Reload effect
-- ============================================================

-- Chain 2011: effect_init → reset ammo stat to max
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (2011, 'Reload - effect init: reset ammo to max', 'effect', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (2011, 'effect_init', NULL, 'player', false, 0);

-- The actual stat reset uses entity.getAmmoStat() dynamically; we model this
-- as a change_stat action with stat_id=-1 (engine-side meaning: use ammo stat).
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (2011, 'change_stat', NULL, NULL, '{"stat_id": -1, "use_ammo_stat": true, "set_to_max": true}', 0, 0);

-- ============================================================
-- TestEffect
-- ============================================================

-- Chain 2021: effect_pulse_end → abandon mission 1627 (tests abandon action)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (2021, 'TestEffect - pulse end: abandon mission 1627', 'effect', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (2021, 'effect_pulse_end', NULL, 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (2021, 'abandon_mission', 1627, NULL, '{}', 0, 0);

-- Chain 2022: effect_pulse_end → change stat (stat 8, 0→100) — tests change_stat action
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (2022, 'TestEffect - pulse end: change_stat(8, 0, 100)', 'effect', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (2022, 'effect_pulse_end', NULL, 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (2022, 'change_stat', NULL, NULL, '{"stat_id": 8, "min": 0, "max": 100}', 0, 0);

-- Chain 2023: effect_pulse_end → apply effect 1635 — tests apply_effect action
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (2023, 'TestEffect - pulse end: apply_effect(1635)', 'effect', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (2023, 'effect_pulse_end', NULL, 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (2023, 'apply_effect', 1635, NULL, '{}', 0, 0);

-- Chain 2024: effect_pulse_end → remove effect 1635 (10s delay) — tests remove_effect action
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (2024, 'TestEffect - pulse end: remove_effect(1635) after 10s', 'effect', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (2024, 'effect_pulse_end', NULL, 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (2024, 'remove_effect', 1635, NULL, '{}', 10000, 0);

-- Chain 2025: effect_pulse_end → fail objective 5541 in mission 1627 — tests fail_objective action
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (2025, 'TestEffect - pulse end: fail_objective(1627, 5541)', 'effect', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (2025, 'effect_pulse_end', NULL, 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (2025, 'fail_objective', 1627, '5541', '{}', 0, 0);
