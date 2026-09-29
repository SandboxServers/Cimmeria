--
-- resources.ammo_modifiers: Hollow Point and Armor Piercing (ammo campaign
-- AM-04, issue #1026, decision D-AM07).
--
-- RECONSTRUCTION. No numbers survive for either type. The cooked ability
-- text gives only directions, and these multipliers are chosen to match
-- them:
--
--   715 Hollow Point Ammunition:  "Damage Type: Physical / Penetration:
--       Decreased / Damage: Increased" (effect_ids empty)
--   719 Armor Piercing Ammunition: "Damage Type: Physical / Penetration:
--       Increased / Damage: Reduced", effect 747 "Penetration: Increased /
--       Damage: Decreased" (no NVPs)
--
-- damage_mult scales the shot's pre-armour damage; penetration_mult divides
-- the armour mitigation (2.0 lets half the armour stand, 0.5 twice as
-- much). See crates/cell-world/src/cell/effects/ammo_damage.rs. The toggle
-- abilities are never cast: toggle_ability_id is provenance only.
--
-- Wave-2 families (AM-08 .. AM-11c) copy this file's shape into their own
-- ammo_modifiers_<family>.sql with one \ir line in db/database.sql after
-- Effects/Seed/effects.sql, so the same file may also seed the family's
-- on-hit resources.effects / effect_nvps rows. Reserved on-hit effect and
-- nvp ids, 10 per family: 9110-9119 Incendiary, 9120-9129 EMP, 9130-9139
-- Explosive, 9140-9149 dart crowd-control, 9150-9159 dart tech, 9160-9169
-- dart support. (Seeded effects end at 5309, seeded nvps at 462.)
--

SET search_path = resources, pg_catalog;

INSERT INTO ammo_modifiers (ammo_type, damage_mult, penetration_mult, damage_type, on_hit_effect_id, toggle_ability_id) VALUES ('Bullet_Hollow_Point', 1.25, 0.5, 'DT_Physical', NULL, 715);
INSERT INTO ammo_modifiers (ammo_type, damage_mult, penetration_mult, damage_type, on_hit_effect_id, toggle_ability_id) VALUES ('Bullet_Armor_Piercing', 0.9, 2.0, 'DT_Physical', NULL, 719);
