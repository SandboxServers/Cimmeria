--
-- resources.ammo_modifiers: crowd-control darts, Poison, Disease and
-- Tranquilizer (ammo campaign AM-11a, issue #1026, decision D-AM07), and
-- their on-hit effects from the AM-11a id block 9140-9149 (effects and nvps).
--
-- RECONSTRUCTION, every number. The toggle abilities give only directions:
--
--   990 Dart Type: Hazardous: Poison   "Damage Type: Physical / Penetration:
--       Decreased / Damage: Increased", effect_ids empty
--   991 Dart Type: Hazardous: Disease  the same text, effect_ids empty
--       (2876 is a duplicate row with the same name and text)
--   998 Dart Type: Hazardous: Disorient the same text, effect_ids empty.
--       No ability is named Tranquilizer; Disorient is the closest hazardous
--       dart toggle to a sedative, so it is the provenance for
--       Dart_Tranquilizer.
--
-- The "Penetration: Decreased / Damage: Increased" line is boilerplate, not
-- data: 992 Beneficial: Stim, a heal dart, carries it too. So these rows
-- leave damage_mult and penetration_mult at 1.0, and the dart's value is
-- its on-hit effect:
--
--   9140 Poison: Suppression chip of 4 HEALTH on the hit and on each of 4
--        pulses 2 s apart (20 HEALTH over 8 s).
--   9141 Disease: Suppression chip of 2 HEALTH on the hit and on each of 9
--        pulses 2 s apart (20 HEALTH over 18 s): same total, slower.
--   9142 Tranquilizer: MovementSlow, MOVEMENT_SPEED_MOD down 40 (to 60% speed)
--        for 3 pulses 2 s apart, restored at expiry 6 s after the hit.
--
-- The Poison and Disease effects each carry an EffectCategory nvp (Poison,
-- Disease). AM-11c's Antidote dart matches on it to cleanse them, the way
-- Antidote 1228's effects 5005 / 5004 remove EFFECT_Poison / EFFECT_Disease.
--
-- Suppression writes HEALTH directly, so the DoT ignores armour; MITIGATION
-- is capped at 0 today anyway. Death from a pulse is resolved by the pulse
-- tick's dot_kill_credit.
--

SET search_path = resources, pg_catalog;

INSERT INTO effects (effect_id, ability_id, delay, effect_desc, effect_sequence, flags, icon_location, pulse_count, pulse_duration, tcm_param1, tcm_param2, target_collection_method, use_ability_velocity, is_channeled, name, target_collection_id, event_set_id, script_name) VALUES (9140, 990, 0, 'Poison dart: 4 Health every 2 s, 5 times', 0, 0, 'set:CoreWidgets image:IconMissing', 5, 2, NULL, NULL, 'TCM_Single', false, false, 'Poison Dart Toxin', 0, NULL, 'Suppression');
INSERT INTO effects (effect_id, ability_id, delay, effect_desc, effect_sequence, flags, icon_location, pulse_count, pulse_duration, tcm_param1, tcm_param2, target_collection_method, use_ability_velocity, is_channeled, name, target_collection_id, event_set_id, script_name) VALUES (9141, 991, 0, 'Disease dart: 2 Health every 2 s, 10 times', 0, 0, 'set:CoreWidgets image:IconMissing', 10, 2, NULL, NULL, 'TCM_Single', false, false, 'Disease Dart Infection', 0, NULL, 'Suppression');
INSERT INTO effects (effect_id, ability_id, delay, effect_desc, effect_sequence, flags, icon_location, pulse_count, pulse_duration, tcm_param1, tcm_param2, target_collection_method, use_ability_velocity, is_channeled, name, target_collection_id, event_set_id, script_name) VALUES (9142, 998, 0, 'Tranquilizer dart: movement speed -40 for 6 s', 0, 0, 'set:CoreWidgets image:IconMissing', 4, 2, NULL, NULL, 'TCM_Single', false, false, 'Tranquilizer Dart Sedation', 0, NULL, 'MovementSlow');

INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (9140, 9140, 'HealthDamage', '4');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (9141, 9140, 'EffectCategory', 'Poison');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (9142, 9141, 'HealthDamage', '2');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (9143, 9141, 'EffectCategory', 'Disease');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (9144, 9142, 'SpeedReduction', '40');

INSERT INTO ammo_modifiers (ammo_type, damage_mult, penetration_mult, damage_type, on_hit_effect_id, toggle_ability_id) VALUES ('Dart_Poison', 1.0, 1.0, 'DT_Physical', 9140, 990);
INSERT INTO ammo_modifiers (ammo_type, damage_mult, penetration_mult, damage_type, on_hit_effect_id, toggle_ability_id) VALUES ('Dart_Disease', 1.0, 1.0, 'DT_Physical', 9141, 991);
INSERT INTO ammo_modifiers (ammo_type, damage_mult, penetration_mult, damage_type, on_hit_effect_id, toggle_ability_id) VALUES ('Dart_Tranquilizer', 1.0, 1.0, 'DT_Physical', 9142, 998);
