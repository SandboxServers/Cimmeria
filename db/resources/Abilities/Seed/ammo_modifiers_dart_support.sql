--
-- resources.ammo_modifiers: the beneficial darts Stim, Antidote, Coagulant
-- and Adrenaline, with their on-hit effects (ammo campaign AM-11c, issue
-- #1026, decision D-AM07). Effect and nvp ids come from the dart-support
-- block, 9160-9169.
--
-- A beneficial dart hits the target instead of hurting it. Its on-hit effect
-- heals or cleanses the target, and damage_mult is 0.0001 so the shot
-- itself deals no damage: any shot under 5000 points of pre-armour damage
-- rounds to 0. It is not 0 because ammo_modifiers_mults_positive_chk
-- requires damage_mult > 0. Relaxing that CHECK to >= 0 is the clean
-- follow-up.
--
-- All four rows are beneficial = true (AM-11d). With one loaded, a player's
-- shot may land on an ally or on themselves, and it is refused at a hostile
-- target (a hostile NPC or a duel opponent) with a feedback line, so a Stim
-- dart never heals an enemy. See docs/analysis/ammo/worknotes/AM-11d.md.
--
-- Sources:
--
--   Dart_Stim, 992 "Dart Type: Beneficial: Stim", effect 5008 "Direct
--       Heal": "Single Target / Target +10% Focus". SOURCE-BACKED: 10%.
--       992's own text ("Penetration: Decreased / Damage: Increased") is a
--       copy of Hollow Point 715's and is not used.
--   Dart_Antidote, 1228 (effects 5057/5005/5004: remove Contagion, Poison
--       (50%), Disease (33%)) and 2874 (5007/5006: remove Wound, Burning
--       (50%)). The row records 1228; 2874 is the second half. The
--       categories are SOURCE-BACKED. RECONSTRUCTION: the cleanse is
--       deterministic, so the 50% and 33% chances are not rolled.
--   Dart_Coagulant, no toggle ability. MS021 template 3427
--       "DartType:Beneficial:Coagulant", effect 5161 "Cure Wound Effect",
--       FX event set 1477. RECONSTRUCTION: removes one Wound effect.
--   Dart_Adrenaline, 1220 "Buff: Toggle", with no effects and no numbers.
--       RECONSTRUCTION: HealHealth, 10% of max Health (Stim's 10%, on the
--       other pool). Not a StatBuff: a StatBuff sends onTimerUpdate with its
--       effect id to the target's client or witnesses, and 9163 is not in the
--       client's cooked data (an unknown cooked id crashed the client before,
--       #938). Every on-hit effect in this file is server-only: it sends
--       onStatUpdate and no per-effect message.
--   Dart_Nanites, NO EVIDENCE: no ability, effect, moniker or FX sequence
--       anywhere in the seeds or the handoff pack. It has no row, so it fires
--       as a plain dart with no on-hit effect.
--
-- The cleanse (script RemoveEffects,
-- crates/cell-world/src/cell/effects/ammo_dart_support.rs) removes, for each
-- name in its RemoveCategories NVP, one active effect on the target whose
-- own EffectCategory NVP carries that name. The originals keyed this on
-- EFFECT_* monikers, which were never seeded.
--

SET search_path = resources, pg_catalog;

INSERT INTO effects (effect_id, ability_id, delay, effect_desc, effect_sequence, flags, icon_location, pulse_count, pulse_duration, tcm_param1, tcm_param2, target_collection_method, use_ability_velocity, is_channeled, name, target_collection_id, event_set_id, script_name) VALUES (9160, 992, 0, 'Single Target
Target +10% Focus', 0, 0, 'set:CoreWidgets image:IconMissing', 1, 0, NULL, NULL, 'TCM_Single', true, false, 'Dart Stim: Direct Heal', 0, NULL, 'HealFocus');
INSERT INTO effects (effect_id, ability_id, delay, effect_desc, effect_sequence, flags, icon_location, pulse_count, pulse_duration, tcm_param1, tcm_param2, target_collection_method, use_ability_velocity, is_channeled, name, target_collection_id, event_set_id, script_name) VALUES (9161, 1228, 0, 'Single Target
Target Remove 1 Effect each of Poison, Disease, Contagion, Wound, Burning', 0, 0, 'set:CoreWidgets image:IconMissing', 1, 0, NULL, NULL, 'TCM_Single', true, false, 'Dart Antidote: Cleanse', 0, NULL, 'RemoveEffects');
INSERT INTO effects (effect_id, ability_id, delay, effect_desc, effect_sequence, flags, icon_location, pulse_count, pulse_duration, tcm_param1, tcm_param2, target_collection_method, use_ability_velocity, is_channeled, name, target_collection_id, event_set_id, script_name) VALUES (9162, 3427, 0, 'Single Target
Target Remove 1 Effect of Wound', 0, 0, 'set:CoreWidgets image:IconMissing', 1, 0, NULL, NULL, 'TCM_Single', true, false, 'Dart Coagulant: Cure Wound', 0, NULL, 'RemoveEffects');
INSERT INTO effects (effect_id, ability_id, delay, effect_desc, effect_sequence, flags, icon_location, pulse_count, pulse_duration, tcm_param1, tcm_param2, target_collection_method, use_ability_velocity, is_channeled, name, target_collection_id, event_set_id, script_name) VALUES (9163, 1220, 0, 'Single Target
Target +10% Health', 0, 0, 'set:CoreWidgets image:IconMissing', 1, 0, NULL, NULL, 'TCM_Single', true, false, 'Dart Adrenaline: Direct Heal', 0, NULL, 'HealHealth');

INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (9160, 9160, 'HealPercentage', '10');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (9161, 9161, 'RemoveCategories', 'Poison,Disease,Contagion,Wound,Burning');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (9162, 9162, 'RemoveCategories', 'Wound');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (9163, 9163, 'HealPercentage', '10');

INSERT INTO ammo_modifiers (ammo_type, damage_mult, penetration_mult, damage_type, on_hit_effect_id, toggle_ability_id, beneficial) VALUES ('Dart_Stim', 0.0001, 1.0, NULL, 9160, 992, true);
INSERT INTO ammo_modifiers (ammo_type, damage_mult, penetration_mult, damage_type, on_hit_effect_id, toggle_ability_id, beneficial) VALUES ('Dart_Antidote', 0.0001, 1.0, NULL, 9161, 1228, true);
INSERT INTO ammo_modifiers (ammo_type, damage_mult, penetration_mult, damage_type, on_hit_effect_id, toggle_ability_id, beneficial) VALUES ('Dart_Coagulant', 0.0001, 1.0, NULL, 9162, 3427, true);
INSERT INTO ammo_modifiers (ammo_type, damage_mult, penetration_mult, damage_type, on_hit_effect_id, toggle_ability_id, beneficial) VALUES ('Dart_Adrenaline', 0.0001, 1.0, NULL, 9163, 1220, true);
