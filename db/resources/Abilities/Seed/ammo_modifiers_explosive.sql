--
-- resources.ammo_modifiers: Explosive rounds (ammo campaign AM-10, issue
-- #1026, decision D-AM07), plus the on-hit splash effect 9130 and its NVP
-- 9130, from AM-10's reserved block (9130-9139).
--
-- RECONSTRUCTION. No numbers survive. The cooked text of the toggle gives
-- directions only, and they are the same as Hollow Point's:
--
--   1446 Explosive Ammunition: "Buff: / Toggle / Damage Type: Physical /
--       Penetration: Decreased / Damage: Increased" (effect_ids empty)
--
-- The splash is not in the cooked text either; it is this packet's design
-- for what makes an explosive round different from a hollow point:
--
--   damage_mult 1.1       smaller direct bonus than Hollow Point's 1.25,
--                         because the round also splashes
--   penetration_mult 0.5  "Penetration: Decreased", as Hollow Point
--   effect 9130           TCM_AERadius, tcm_param1 'Short' (5 m, the tier
--                         the seeded grenade "Ground Blast Damage" effects
--                         use): every other hostile within 5 m of the target
--                         takes SplashDamageFraction 0.5 of the shot
--
-- The splash runs in the combat pipeline, not as an effect script, so 9130
-- has no script_name: see crates/cell-world/src/cell/effects/ammo_explosive.rs
-- and crates/cell-combat/src/cell/abilities/damage_apply/ammo_splash.rs.
-- The toggle ability is never cast: toggle_ability_id is provenance only.
--

SET search_path = resources, pg_catalog;

INSERT INTO effects (effect_id, ability_id, delay, effect_desc, effect_sequence, flags, icon_location, pulse_count, pulse_duration, tcm_param1, tcm_param2, target_collection_method, use_ability_velocity, is_channeled, name, target_collection_id, event_set_id, script_name) VALUES (9130, 1446, 0, 'Secondary Targets
Splash: 50% of the shot', 0, 0, 'set:CoreWidgets image:IconMissing', 1, 0, 'Short', NULL, 'TCM_AERadius', false, false, 'Explosive Round Splash', 0, NULL, NULL);
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (9130, 9130, 'SplashDamageFraction', '0.5');

INSERT INTO ammo_modifiers (ammo_type, damage_mult, penetration_mult, damage_type, on_hit_effect_id, toggle_ability_id) VALUES ('Bullet_Explosive', 1.1, 0.5, 'DT_Physical', 9130, 1446);
