--
-- resources.ammo_modifiers: Incendiary rounds, with their on-hit burn
-- (ammo campaign AM-08, issue #1026, decision D-AM07).
--
-- Source: toggle ability 723 "Incendiary Ammunition", cooked text
-- "Buff: / Toggle / Damage Type: Energy / Penetration: Nominal /
-- Damage: Nominal". Its effect_ids is empty, so nothing survives for the
-- burn itself.
--
--   damage_type      DT_Energy   from the cooked text.
--   damage_mult      1.0         "Damage: Nominal".
--   penetration_mult 1.0         "Penetration: Nominal".
--   on_hit_effect_id 9110        RECONSTRUCTION: the burn below. The
--       client knows a Burning category (effects 2407, 3475, 3477, 3484,
--       4797 and 5000 "Remove 1 Effect of Moniker EFFECT_Burning"), but no
--       seeded effect is a working burn: Flame BC 2852 ("-150F -30H
--       8 Ticks") is seeded with pulse_count 1 and no script.
--
-- Effect 9110, Incendiary Burn. RECONSTRUCTION, every number:
--   script_name  RangedEnergyDamage, the existing energy-damage script: it
--       takes FocusDamage and HealthDamage from both pools each pulse, with
--       no Focus-first gate. No new script is needed.
--   pulse_count 4, pulse_duration 1.0: one burn pulse on the hit, three
--       more over three seconds.
--   FocusDamage 15, HealthDamage 3 per pulse: Flame BC's 5:1 Focus to
--       Health ratio at a tenth of its size, so a full burn (60F / 12H) is
--       about one extra pistol shot (641: -100F / -10H) spread over time.
--   EffectCategory Burning (nvp 9112): the EFFECT_Burning category that
--       AM-11c's Antidote / Coagulant RemoveEffects script matches to
--       cleanse the burn. Not read by RangedEnergyDamage.
--   Stacking is the ADR's rule (abilities-and-effects-system.md
--       decision 4): the same shooter refreshes the burn, a second shooter
--       adds a burn of their own.
--
-- Ids come from AM-08's reserved block, 9110-9119 (effects and nvps).
-- The seed guard is effects::ammo_incendiary::tests::live_db_incendiary_seed_rows
-- in cimmeria-cell-world, which pins these numbers to the constants in
-- crates/cell-world/src/cell/effects/ammo_incendiary.rs.
--

SET search_path = resources, pg_catalog;

INSERT INTO effects (effect_id, ability_id, delay, effect_desc, effect_sequence, flags, icon_location, pulse_count, pulse_duration, tcm_param1, tcm_param2, target_collection_method, use_ability_velocity, is_channeled, name, target_collection_id, event_set_id, script_name) VALUES (9110, 723, 0, 'Burning: -15F -3H, 4 Ticks', 0, 0, 'set:CoreWidgets image:IconMissing', 4, 1, NULL, NULL, 'TCM_Single', true, false, 'Incendiary Burn', 0, NULL, 'RangedEnergyDamage');

INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (9110, 9110, 'FocusDamage', '15');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (9111, 9110, 'HealthDamage', '3');
-- Required: AM-11c's Antidote and Coagulant darts (RemoveEffects) cleanse
-- an effect by this row, the client's EFFECT_Burning category.
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (9112, 9110, 'EffectCategory', 'Burning');

INSERT INTO ammo_modifiers (ammo_type, damage_mult, penetration_mult, damage_type, on_hit_effect_id, toggle_ability_id) VALUES ('Bullet_Incendiary', 1.0, 1.0, 'DT_Energy', 9110, 723);
