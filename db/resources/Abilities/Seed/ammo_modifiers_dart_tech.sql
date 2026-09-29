--
-- resources.ammo_modifiers: tech-disable darts, Dart_EMP and
-- Dart_Radioactive (ammo campaign AM-11b, issue #1026, decision D-AM07).
--
-- RECONSTRUCTION. No numbers survive for either dart. What the client does
-- carry:
--
--   999 Dart Type: Hazardous: EMP: "Buff: / Toggle / Damage Type: Physical /
--       Penetration: Decreased / Damage: Increased" (effect_ids empty).
--   Dart_Radioactive has no toggle ability at all: no "Radioactive" or
--       "Radiation" dart ability exists in the cooked client, the abilities
--       seed or the handoff pack (Weapon Reconstruction Master section 07
--       lists 17 toggles, none of them Radioactive). The enum value is a
--       legacy-server name. toggle_ability_id is NOT NULL, so the row cites
--       1227 Dart Type: Hazardous: Contagion ("Buff: / Toggle", no text):
--       its name matches no other dart ammo type, and its status lingers,
--       since Antidote dart 1228's effect 5057 cures EFFECT_Contagion.
--
-- The dart auto attack (1086, effect 1237) is "-100F / -10H", so the
-- payloads below are sized against a 10-health, 100-focus dart hit.
--
--   Dart_EMP: the shot itself follows 999's text (damage up, penetration
--       down, physical), smaller than Hollow Point's 1.25 / 0.5 because
--       the dart's value is its payload. On a hit, effect 9150 drains 50
--       FOCUS through the existing RangedEnergyDamage script (FocusDamage
--       only, no HealthDamage): the EMP knocks down the target's shield,
--       the Focus-drain reading the campaign sets for EMP
--       (AM-09 does the same for the EMP bullet). No
--       interrupt: cancelling a target's channel is async combat code the
--       synchronous effect scripts cannot reach.
--   Dart_Radioactive: the shot itself is unmodified (1.0 / 1.0, the
--       ability's own damage type). On a hit, effect 9151 is a lingering
--       radiation dose: 3 HEALTH now and every 2 seconds, 5 pulses in all
--       (15 health over 8 seconds), through the RadiationDamage script in
--       crates/cell-world/src/cell/effects/ammo_dart_tech.rs. Another hit
--       from the same shooter refreshes the dose rather than stacking it
--       (pulsing's same-source rule).
--
-- Both payloads write the stat directly, like every effect script, so
-- armour and MITIGATION never reduce them; EMP's penetration_mult has no
-- effect in live play while MITIGATION is capped at 0.
--
-- Reserved ids (AM-04 worknote): effects and nvps 9150-9159 for this family.
-- The toggle abilities are never cast: toggle_ability_id is provenance only.
--

SET search_path = resources, pg_catalog;

INSERT INTO effects (effect_id, ability_id, delay, effect_desc, effect_sequence, flags, icon_location, pulse_count, pulse_duration, tcm_param1, tcm_param2, target_collection_method, use_ability_velocity, is_channeled, name, target_collection_id, event_set_id, script_name) VALUES (9150, 999, 0, 'EMP dart: Target -50 Focus', 0, 0, 'set:CoreWidgets image:IconMissing', 1, 0, NULL, NULL, 'TCM_Single', false, false, 'Dart EMP Focus Drain', 0, NULL, 'RangedEnergyDamage');
INSERT INTO effects (effect_id, ability_id, delay, effect_desc, effect_sequence, flags, icon_location, pulse_count, pulse_duration, tcm_param1, tcm_param2, target_collection_method, use_ability_velocity, is_channeled, name, target_collection_id, event_set_id, script_name) VALUES (9151, 1227, 0, 'Radioactive dart: Target -3 Health every 2 seconds, 5 pulses', 0, 0, 'set:CoreWidgets image:IconMissing', 5, 2, NULL, NULL, 'TCM_Single', false, false, 'Dart Radiation Dose', 0, NULL, 'RadiationDamage');

INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (9150, 9150, 'FocusDamage', '50');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (9151, 9151, 'HealthDamage', '3');

INSERT INTO ammo_modifiers (ammo_type, damage_mult, penetration_mult, damage_type, on_hit_effect_id, toggle_ability_id) VALUES ('Dart_EMP', 1.1, 0.75, 'DT_Physical', 9150, 999);
INSERT INTO ammo_modifiers (ammo_type, damage_mult, penetration_mult, damage_type, on_hit_effect_id, toggle_ability_id) VALUES ('Dart_Radioactive', 1.0, 1.0, NULL, 9151, 1227);
