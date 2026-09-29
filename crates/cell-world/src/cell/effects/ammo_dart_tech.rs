//! Special-ammo effect scripts, packet AM-11b (ammo campaign, issue #1026):
//! tech-disable darts (EMP, Radioactive).
//!
//! Created empty by AM-F so each family packet adds its script here and one
//! `match` arm in `registry.rs`, and never edits `effects/mod.rs`.
//!
//! # What the two darts do (RECONSTRUCTION)
//!
//! The rows live in `db/resources/Abilities/Seed/ammo_modifiers_dart_tech.sql`,
//! whose header carries the evidence. No numbers survive for either dart.
//!
//! - **`Dart_EMP`** (provenance: toggle 999 "Dart Type: Hazardous: EMP",
//!   "Damage Type: Physical / Penetration: Decreased / Damage: Increased").
//!   The shot is scaled by [`DART_EMP_DAMAGE_MULT`] with armour divided by
//!   [`DART_EMP_PENETRATION_MULT`], as physical damage. On a hit, effect
//!   [`DART_EMP_EFFECT_ID`] drains [`DART_EMP_FOCUS_DRAIN`] FOCUS through the
//!   existing `RangedEnergyDamage` script (a `FocusDamage` NVP and no
//!   `HealthDamage`), so it needs no code here. Focus is the shield, and an
//!   EMP knocking it down is the same reading the campaign gives the EMP
//!   bullet (AM-09). There is no interrupt: cancelling the target's channel
//!   is async combat code a synchronous effect script cannot reach.
//! - **`Dart_Radioactive`** has no toggle ability anywhere in the client; the
//!   row cites 1227 "Dart Type: Hazardous: Contagion" as the nearest
//!   lingering hazard. The shot is unmodified. On a hit, effect
//!   [`DART_RADIOACTIVE_EFFECT_ID`] is a radiation dose: [`RadiationDamage`]
//!   takes [`DART_RADIOACTIVE_PULSE_DAMAGE`] HEALTH at once and again on each
//!   of the remaining pulses ([`DART_RADIOACTIVE_PULSES`] in all, every
//!   [`DART_RADIOACTIVE_PULSE_SECS`] seconds), through the ordinary pulse
//!   tick. A second hit from the same shooter refreshes the dose.
//!
//! # Why a new script for the dose
//!
//! An on-hit effect with no `script_name` never fires its first pulse: the
//! combat pipeline reads only the ability's own effects for NVP damage, so an
//! NVP-only DoT would lose a pulse. `MeleeDamage` does the right arithmetic
//! but logs every tick as `melee_damage`, which reads as a melee hit in
//! SigNoz. [`RadiationDamage`] logs `radiation_pulse` instead.
//!
//! Like every effect script, both payloads write the stat directly, so armour
//! and MITIGATION never reduce them.

use super::{EffectContext, EffectScript};
use cimmeria_entity::stats::HEALTH;

/// `Dart_EMP` shot damage multiplier (RECONSTRUCTION, seeded).
pub const DART_EMP_DAMAGE_MULT: f32 = 1.1;
/// `Dart_EMP` armour divisor (RECONSTRUCTION, seeded).
pub const DART_EMP_PENETRATION_MULT: f32 = 0.75;
/// `Dart_EMP` on-hit effect: `RangedEnergyDamage` with `FocusDamage` only.
pub const DART_EMP_EFFECT_ID: i32 = 9150;
/// FOCUS the EMP dart drains on a hit (RECONSTRUCTION, seeded NVP).
pub const DART_EMP_FOCUS_DRAIN: i32 = 50;
/// The toggle ability the EMP row cites.
pub const DART_EMP_TOGGLE_ABILITY: i32 = 999;

/// `Dart_Radioactive` on-hit effect: [`RadiationDamage`], pulsing.
pub const DART_RADIOACTIVE_EFFECT_ID: i32 = 9151;
/// HEALTH per radiation pulse (RECONSTRUCTION, seeded NVP).
pub const DART_RADIOACTIVE_PULSE_DAMAGE: i32 = 3;
/// Pulses per dose, the first one on the hit (RECONSTRUCTION, seeded).
pub const DART_RADIOACTIVE_PULSES: i32 = 5;
/// Seconds between radiation pulses (RECONSTRUCTION, seeded).
pub const DART_RADIOACTIVE_PULSE_SECS: f32 = 2.0;
/// The toggle ability the Radioactive row cites (Contagion; see above).
pub const DART_RADIOACTIVE_TOGGLE_ABILITY: i32 = 1227;

// ── RadiationDamage ──────────────────────────────────────────────────────

/// One pulse of a lingering radiation dose: takes the `HealthDamage` NVP off
/// the target's HEALTH, floored at 0.
///
/// NVPs:
///   - `HealthDamage` (i32) — HEALTH per pulse; 0, negative or missing is a
///     no-op.
///
/// A target already at 0 HEALTH is left alone: the pulse tick skips dead
/// targets itself, but the on-hit apply runs after the shot's own damage,
/// which may have killed it. Death from a lethal pulse is resolved by the
/// callers (the pipeline's post-script death sweep, the pulse tick's DoT
/// kill credit), as for every other script.
pub struct RadiationDamage;

impl EffectScript for RadiationDamage {
    fn on_apply(&self, ctx: &mut EffectContext) {
        let damage = ctx.effect.param_i32("HealthDamage");
        if damage <= 0 {
            return;
        }
        let Some(target) = ctx.space_mgr.get_entity_mut(ctx.target_id) else {
            tracing::debug!(
                target: "abilities",
                event = "radiation_pulse_skipped",
                reason = "target_missing",
                source_id = ctx.source_id,
                target_id = ctx.target_id,
                effect_id = ctx.effect.effect_id,
                "RadiationDamage: target missing, pulse skipped"
            );
            return;
        };
        let Some(stat) = target.stats.get_mut(HEALTH) else {
            return;
        };
        let before = stat.cur;
        if before <= 0 {
            tracing::debug!(
                target: "abilities",
                event = "radiation_pulse_skipped",
                reason = "target_dead",
                source_id = ctx.source_id,
                target_id = ctx.target_id,
                effect_id = ctx.effect.effect_id,
                "RadiationDamage: target already dead, pulse skipped"
            );
            return;
        }
        let after = (before - damage).max(0);
        stat.update(stat.min, after, stat.max);
        tracing::info!(
            target: "abilities",
            event = "radiation_pulse",
            source_id = ctx.source_id,
            target_id = ctx.target_id,
            effect_id = ctx.effect.effect_id,
            damage,
            health_before = before,
            health_after = after,
            "RadiationDamage pulse applied"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::effects::ammo_damage::shot_ammo;
    use crate::cell::effects::test_fixtures::make_mgr_with_target;
    use crate::cell::effects::{dispatch_by_name, registry};
    use crate::cell::space_manager::SpaceManager;
    use crate::cell::spawner::{AmmoCatalog, AmmoModifier};
    use cimmeria_entity::abilities::{AbilityDef, EffectDef, DT_ENERGY, DT_PHYSICAL};
    use cimmeria_entity::ammo_type::{DART_DEFAULT, DART_EMP, DART_RADIOACTIVE};
    use cimmeria_entity::cell_entity::BandolierItem;
    use cimmeria_entity::stats::FOCUS;
    use std::collections::HashMap;

    /// The two reserve items `ammo_item_types.sql` maps the darts to.
    const EMP_ITEM: i32 = 9008;
    const RADIOACTIVE_ITEM: i32 = 9009;

    /// The rows `ammo_modifiers_dart_tech.sql` seeds.
    fn seeded_rows() -> Vec<AmmoModifier> {
        vec![
            AmmoModifier {
                ammo_type: DART_EMP,
                damage_mult: DART_EMP_DAMAGE_MULT,
                penetration_mult: DART_EMP_PENETRATION_MULT,
                damage_type: Some(i32::from(DT_PHYSICAL)),
                on_hit_effect_id: Some(DART_EMP_EFFECT_ID),
                toggle_ability_id: DART_EMP_TOGGLE_ABILITY,
                beneficial: false,
            },
            AmmoModifier {
                ammo_type: DART_RADIOACTIVE,
                damage_mult: 1.0,
                penetration_mult: 1.0,
                damage_type: None,
                on_hit_effect_id: Some(DART_RADIOACTIVE_EFFECT_ID),
                toggle_ability_id: DART_RADIOACTIVE_TOGGLE_ABILITY,
                beneficial: false,
            },
        ]
    }

    /// The two on-hit effects the seed defines.
    fn seeded_effects() -> [EffectDef; 2] {
        let nvp = |k: &str, v: i32| HashMap::from([(k.to_string(), v.to_string())]);
        [
            EffectDef {
                effect_id: DART_EMP_EFFECT_ID,
                ability_id: DART_EMP_TOGGLE_ABILITY,
                script_name: Some("RangedEnergyDamage".to_string()),
                params: nvp("FocusDamage", DART_EMP_FOCUS_DRAIN),
                ..Default::default()
            },
            EffectDef {
                effect_id: DART_RADIOACTIVE_EFFECT_ID,
                ability_id: DART_RADIOACTIVE_TOGGLE_ABILITY,
                script_name: Some("RadiationDamage".to_string()),
                pulse_count: DART_RADIOACTIVE_PULSES,
                pulse_duration: DART_RADIOACTIVE_PULSE_SECS,
                params: nvp("HealthDamage", DART_RADIOACTIVE_PULSE_DAMAGE),
                ..Default::default()
            },
        ]
    }

    fn dart_shot() -> AbilityDef {
        AbilityDef {
            ability_id: 1086,
            name: "Dart Pistol Auto Attack".to_string(),
            cooldown: 0.0,
            warmup: 0.0,
            flags: 0,
            is_ranged: true,
            min_range: 0.0,
            max_range: 30.0,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 1,
            event_set_id: None,
            velocity: 0.0,
        }
    }

    /// Player 1 holding a dart pistol loaded with `ammo_type` (the weapon's
    /// `ammo_types` widening is AM-11a's; the bandolier slot is stubbed
    /// directly), with the seeded rows and effects loaded. Entity 1 is also
    /// the target, which is all the scripts need.
    fn mgr_loaded(ammo_type: i32) -> SpaceManager {
        let mut mgr = make_mgr_with_target();
        mgr.ammo_catalog = AmmoCatalog::from_rows(
            seeded_rows(),
            [(DART_EMP, EMP_ITEM), (DART_RADIOACTIVE, RADIOACTIVE_ITEM)],
        );
        for e in seeded_effects() {
            mgr.effect_defs.insert(e.effect_id, e);
        }
        let e = mgr.get_entity_mut(1).unwrap();
        e.active_bandolier_slot = 0;
        e.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 1,
                item_id: 3584,
                clip_size: 10,
                default_ammo_type: DART_DEFAULT,
                current_ammo: 10,
                cur_ammo_type: ammo_type,
            },
        );
        mgr
    }

    fn stat(mgr: &SpaceManager, id: i32) -> i32 {
        mgr.get_entity(1).unwrap().stats.get(id).unwrap().cur
    }

    /// Fire the shot's on-hit effect on entity 1, the way `damage_apply`
    /// does after the damage: resolve the row, then dispatch its script.
    fn fire_on_hit(mgr: &mut SpaceManager) -> Option<i32> {
        let shot = shot_ammo(mgr, 1, Some(&dart_shot()), true)?;
        let eid = shot.on_hit_effect_id(mgr)?;
        let effect = mgr.effect_defs.get(&eid).cloned().unwrap();
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: mgr,
        };
        assert!(dispatch_by_name(
            effect.script_name.as_deref().unwrap(),
            &mut ctx
        ));
        Some(eid)
    }

    // ── Modifier math ──

    #[test]
    fn emp_dart_scales_up_lowers_penetration_and_lands_physical() {
        let mgr = mgr_loaded(DART_EMP);
        let shot = shot_ammo(&mgr, 1, Some(&dart_shot()), true).expect("EMP modifies");
        assert!((shot.damage_scale() - 1.1).abs() < 1e-6);
        assert!((shot.penetration_mult() - 0.75).abs() < 1e-6);
        // An energy weapon's shot lands physical, per 999's text.
        assert_eq!(shot.damage_type(DT_ENERGY), DT_PHYSICAL);
        assert_eq!(shot.ammo_item_id, Some(EMP_ITEM));
    }

    #[test]
    fn radioactive_dart_leaves_the_shot_itself_alone() {
        let mgr = mgr_loaded(DART_RADIOACTIVE);
        let shot = shot_ammo(&mgr, 1, Some(&dart_shot()), true).expect("a row exists");
        assert!((shot.damage_scale() - 1.0).abs() < 1e-9);
        assert!((shot.penetration_mult() - 1.0).abs() < 1e-9);
        assert_eq!(
            shot.damage_type(DT_ENERGY),
            DT_ENERGY,
            "keeps the ability's type"
        );
    }

    // ── On-hit payloads through the shot ──

    #[test]
    fn emp_dart_hit_drains_focus_only() {
        let mut mgr = mgr_loaded(DART_EMP);
        let (hp, focus) = (stat(&mgr, HEALTH), stat(&mgr, FOCUS));
        assert_eq!(fire_on_hit(&mut mgr), Some(DART_EMP_EFFECT_ID));
        assert_eq!(stat(&mgr, FOCUS), focus - DART_EMP_FOCUS_DRAIN);
        assert_eq!(stat(&mgr, HEALTH), hp, "EMP payload takes no health");
    }

    #[test]
    fn radioactive_dart_hit_starts_a_pulsing_dose() {
        let mut mgr = mgr_loaded(DART_RADIOACTIVE);
        let hp = stat(&mgr, HEALTH);
        assert_eq!(fire_on_hit(&mut mgr), Some(DART_RADIOACTIVE_EFFECT_ID));
        assert_eq!(stat(&mgr, HEALTH), hp - DART_RADIOACTIVE_PULSE_DAMAGE);
        let effect = &mgr.effect_defs[&DART_RADIOACTIVE_EFFECT_ID];
        assert!(
            effect.is_pulsing(),
            "the pipeline registers it for the tick"
        );
        assert!((effect.total_duration() - 10.0).abs() < 1e-6);
    }

    /// Default darts fire with no payload, whatever the catalog holds.
    #[test]
    fn default_dart_has_no_payload() {
        let mut mgr = mgr_loaded(DART_DEFAULT);
        let focus = stat(&mgr, FOCUS);
        assert_eq!(fire_on_hit(&mut mgr), None);
        assert_eq!(stat(&mgr, FOCUS), focus);
    }

    // ── RadiationDamage ──

    fn dose(damage: &str) -> EffectDef {
        EffectDef {
            effect_id: DART_RADIOACTIVE_EFFECT_ID,
            script_name: Some("RadiationDamage".to_string()),
            params: HashMap::from([("HealthDamage".to_string(), damage.to_string())]),
            ..Default::default()
        }
    }

    fn apply(mgr: &mut SpaceManager, effect: &EffectDef) {
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect,
            space_mgr: mgr,
        };
        RadiationDamage.on_apply(&mut ctx);
    }

    #[test]
    fn radiation_is_registered() {
        assert!(registry::lookup("RadiationDamage").is_some());
    }

    #[test]
    fn each_pulse_takes_the_nvp_and_logs_it() {
        let mut mgr = make_mgr_with_target(); // HEALTH 50/100
        let logs = crate::test_support::LogCapture::install();
        apply(&mut mgr, &dose("3"));
        apply(&mut mgr, &dose("3"));
        assert_eq!(stat(&mgr, HEALTH), 44);
        let pulses: Vec<_> = logs
            .all()
            .into_iter()
            .filter(|c| c.target == "abilities" && c.has_field("event", "radiation_pulse"))
            .collect();
        assert_eq!(pulses.len(), 2);
        assert!(
            pulses[1].has_field("health_before", "47"),
            "{:?}",
            pulses[1]
        );
        assert!(pulses[1].has_field("health_after", "44"), "{:?}", pulses[1]);
        assert!(pulses[1].has_field("effect_id", "9151"), "{:?}", pulses[1]);
    }

    #[test]
    fn a_pulse_floors_health_at_zero_and_skips_the_dead() {
        let mut mgr = make_mgr_with_target();
        apply(&mut mgr, &dose("80"));
        assert_eq!(stat(&mgr, HEALTH), 0);
        let logs = crate::test_support::LogCapture::install();
        apply(&mut mgr, &dose("80"));
        assert_eq!(stat(&mgr, HEALTH), 0);
        assert!(logs
            .all()
            .iter()
            .any(|c| c.has_field("event", "radiation_pulse_skipped")
                && c.has_field("reason", "target_dead")));
    }

    #[test]
    fn zero_negative_or_missing_damage_is_a_no_op() {
        let mut mgr = make_mgr_with_target();
        for bad in ["0", "-5", "abc"] {
            apply(&mut mgr, &dose(bad));
        }
        apply(
            &mut mgr,
            &EffectDef {
                effect_id: DART_RADIOACTIVE_EFFECT_ID,
                ..Default::default()
            },
        );
        assert_eq!(stat(&mgr, HEALTH), 50);
    }

    #[test]
    fn a_missing_target_is_a_no_op() {
        let mut mgr = make_mgr_with_target();
        let effect = dose("3");
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 4242,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        RadiationDamage.on_apply(&mut ctx);
        assert_eq!(stat(&mgr, HEALTH), 50);
    }

    // ── Seed ──

    /// `ammo_modifiers_dart_tech.sql` loads both rows through the startup
    /// loader with the constants above, and both on-hit effects load through
    /// `load_effect_defs` with their script, pulses and NVP. Fails if the
    /// seed or its `\ir` line is dropped or a number drifts.
    #[tokio::test]
    async fn live_db_dart_tech_seed_rows() {
        let pool = crate::test_support::require_db_or_skip!();
        let catalog = crate::cell::spawner::load_ammo_catalog(&pool)
            .await
            .expect("ammo catalog loads");
        let [emp, radioactive]: [AmmoModifier; 2] = seeded_rows().try_into().unwrap();
        assert_eq!(catalog.modifier(DART_EMP), Some(&emp));
        assert_eq!(catalog.modifier(DART_RADIOACTIVE), Some(&radioactive));
        assert_eq!(catalog.item_id_for(DART_EMP), Some(EMP_ITEM));
        assert_eq!(
            catalog.item_id_for(DART_RADIOACTIVE),
            Some(RADIOACTIVE_ITEM)
        );

        let defs = crate::cell::spawner::load_effect_defs(&pool)
            .await
            .expect("effect defs load");
        for want in seeded_effects() {
            let got = defs
                .get(&want.effect_id)
                .unwrap_or_else(|| panic!("effect {} is seeded", want.effect_id));
            assert_eq!(got.script_name, want.script_name, "{}", want.effect_id);
            assert_eq!(got.pulse_count, want.pulse_count, "{}", want.effect_id);
            assert!((got.pulse_duration - want.pulse_duration).abs() < 1e-6);
            assert_eq!(got.params, want.params, "{}", want.effect_id);
            assert!(
                registry::lookup(got.script_name.as_deref().unwrap()).is_some(),
                "effect {} names a registered script",
                want.effect_id
            );
        }

        let names: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM resources.abilities WHERE ability_id IN (999, 1227) \
             ORDER BY ability_id",
        )
        .fetch_all(&pool)
        .await
        .expect("abilities query");
        assert_eq!(
            names,
            [
                "Dart Type: Hazardous: EMP",
                "Dart Type: Hazardous: Contagion"
            ]
        );
    }
}
