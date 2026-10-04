--
-- resources.ammo_modifiers: EMP rounds (ammo campaign AM-09, issue #1026,
-- decision D-AM07), plus the on-hit effect 9120 and its NVPs.
--
-- RECONSTRUCTION. No numbers survive for EMP rounds:
--
--   1445 EMP Ammunition: "Buff: / Toggle / Damage Type: Physical /
--       Penetration: Decreased / Damage: Increased" (effect_ids empty).
--       The text is word-for-word Hollow Point's (715), so it gives only
--       directions: damage up, penetration down, physical damage type.
--
-- The on-hit behaviour follows the EMP Grenade (ability 2864), the one EMP
-- the cooked data describes in full: effect 4202 "Non-Mechanical Target
-- Damage -100F / -0H" drains Focus from living targets, effect 4200
-- "Mechanical Target Damage -0F / -225H" damages machines' Health instead,
-- split by the "Mechanical Type Check" effects 4203 / 4204. (4201 also
-- disorients a mechanical target for 20 s; the rounds do not, see
-- worknotes/AM-09.md.) Effect 9120's script, EmpDisrupt, makes the same
-- split per shot at about a tenth of the grenade:
--
--   FocusDamage 10            Focus drained from a non-mechanical target.
--                             A tenth of the grenade's 100, and a tenth
--                             of a pistol auto attack's (effect 641) 100.
--   MechanicalHealthDamage 5  Health taken from a mechanical target, half
--                             a pistol auto attack's base HealthDamage 10.
--   InterruptChance 25        Chance in percent that a hit breaks the
--                             target's warmup and channels (ability
--                             mechanics AB-09c), before its interruptRes.
--                             DESIGN, not recovered: an EMP disrupts, and
--                             a quarter of hits breaks a cast without an
--                             automatic weapon locking out every warmup.
--
-- "Mechanical" is the target's body set: drones, the Prisoner Retrieval
-- Unit, the BattleWalker and deployables. The list and its reasons are in
-- crates/cell-world/src/cell/effects/ammo_emp.rs (MECHANICAL_BODY_SETS).
--
-- The modifier: damage_mult 1.1 ("Damage: Increased", below Hollow Point's
-- 1.25 because EMP also has an on-hit effect), penetration_mult 0.75
-- ("Penetration: Decreased", milder than Hollow Point's 0.5). MITIGATION is
-- capped at 0 in the default stats, so the penetration has no effect in
-- live play yet.
--
-- Reserved ids for this family: effects and nvps 9120-9129.
--

SET search_path = resources, pg_catalog;

INSERT INTO effects (effect_id, ability_id, delay, effect_desc, effect_sequence, flags, icon_location, pulse_count, pulse_duration, tcm_param1, tcm_param2, target_collection_method, use_ability_velocity, is_channeled, name, target_collection_id, event_set_id, script_name) VALUES (9120, 1445, 0, 'EMP rounds on hit: -10F on a living target, -5H on a mechanical one', 0, 0, 'set:CoreWidgets image:IconMissing', 1, 0, NULL, NULL, 'TCM_Single', false, false, 'EMP Rounds Disruption', 0, NULL, 'EmpDisrupt');

INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (9120, 9120, 'FocusDamage', '10');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (9121, 9120, 'MechanicalHealthDamage', '5');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (9122, 9120, 'InterruptChance', '25');

INSERT INTO ammo_modifiers (ammo_type, damage_mult, penetration_mult, damage_type, on_hit_effect_id, toggle_ability_id) VALUES ('Bullet_EMP', 1.1, 0.75, 'DT_Physical', 9120, 1445);
