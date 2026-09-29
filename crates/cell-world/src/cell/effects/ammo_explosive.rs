//! Special-ammo effect scripts, packet AM-10 (ammo campaign, issue #1026):
//! Explosive rounds (toggle ability 1446), the on-hit effect.
//!
//! Created empty by AM-F so each family packet adds its script here and one
//! `EFFECT_SCRIPTS` row in `registry.rs`, and never edits `effects/mod.rs`.
//!
//! # Explosive rounds splash; they need no script
//!
//! An Explosive shot hits its target like any shot (the AM-04 pipeline
//! applies the row's `damage_mult` and `penetration_mult`), then deals a
//! reduced share of the shot to the other hostiles within a few metres of
//! the target. That is not an [`super::EffectScript`]: a script holds a
//! synchronous [`super::EffectContext`] and cannot send the secondaries'
//! `onEffectResults` and `onStatUpdate`, resolve their deaths or give them
//! threat. The splash therefore runs in the combat pipeline
//! (`cimmeria-cell-combat`, `abilities::damage_apply::ammo_splash`), which
//! applies each secondary through the same per-target function as a cone or
//! ground-AoE secondary.
//!
//! This file owns the data side: [`splash_of`] reads the on-hit effect
//! (effect 9130, `ammo_modifiers_explosive.sql`) into a [`Splash`]. The
//! effect is recognised by its target collection method, `TCM_AERadius`,
//! the method every seeded blast effect uses; its radius is the
//! `tcm_param1` range tier (`EffectDef::tcm_range_meters`) and its share of
//! the shot is the [`SPLASH_FRACTION_NVP`] NVP. So no `EFFECT_SCRIPTS` row.

use cimmeria_entity::abilities::{EffectDef, TCM_AE_RADIUS};

/// The NVP naming the share of the shot's damage each splash target takes.
pub const SPLASH_FRACTION_NVP: &str = "SplashDamageFraction";

/// The splash an on-hit effect asks for, from [`splash_of`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Splash {
    /// The on-hit effect this came from (a telemetry correlator).
    pub effect_id: i32,
    /// Metres from the primary target within which a hostile is splashed.
    pub radius: f32,
    /// The share of the shot's damage each splash target takes, in `(0, 1]`.
    pub fraction: f64,
}

/// The splash `effect` describes, or `None` when it is not a splash.
///
/// A splash is a `TCM_AERadius` effect with a [`SPLASH_FRACTION_NVP`] in
/// `(0, 1]`. A radius effect with a missing or out-of-range fraction warns
/// and splashes nothing: the splash is "reduced damage", so a fraction above
/// 1 is a bad row, not a bigger blast.
pub fn splash_of(effect: &EffectDef) -> Option<Splash> {
    if effect.target_collection_method != TCM_AE_RADIUS {
        return None;
    }
    let raw = effect.params.get(SPLASH_FRACTION_NVP);
    let fraction = raw.and_then(|v| v.parse::<f64>().ok());
    let Some(fraction) = fraction.filter(|f| f.is_finite() && *f > 0.0 && *f <= 1.0) else {
        tracing::warn!(
            target: "ammo",
            event = "ammo_splash_bad_fraction",
            effect_id = effect.effect_id,
            value = raw.map(String::as_str),
            "on-hit radius effect has no SplashDamageFraction in (0, 1]; the shot does not splash"
        );
        return None;
    };
    Some(Splash {
        effect_id: effect.effect_id,
        radius: EffectDef::tcm_range_meters(&effect.tcm_param1),
        fraction,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::spawner::AmmoModifier;
    use cimmeria_entity::abilities::{DT_PHYSICAL, TCM_AE_CONE, TCM_SINGLE};
    use cimmeria_entity::ammo_type::BULLET_EXPLOSIVE;

    fn radius_effect(tier: &str, fraction: Option<&str>) -> EffectDef {
        let mut params = std::collections::HashMap::new();
        if let Some(f) = fraction {
            params.insert(SPLASH_FRACTION_NVP.to_string(), f.to_string());
        }
        EffectDef {
            effect_id: 9130,
            target_collection_method: TCM_AE_RADIUS.to_string(),
            tcm_param1: tier.to_string(),
            params,
            ..Default::default()
        }
    }

    /// The seeded shape: Short radius (5 m), half the shot.
    #[test]
    fn a_radius_effect_with_a_fraction_splashes() {
        assert_eq!(
            splash_of(&radius_effect("Short", Some("0.5"))),
            Some(Splash {
                effect_id: 9130,
                radius: 5.0,
                fraction: 0.5,
            })
        );
        assert_eq!(
            splash_of(&radius_effect("Melee", Some("1"))).map(|s| s.radius),
            Some(2.5)
        );
    }

    /// Only `TCM_AERadius` splashes: a single-target or cone on-hit effect
    /// (HP, AP, a DoT) never fans out, whatever its NVPs say.
    #[test]
    fn only_a_radius_effect_splashes() {
        for tcm in [TCM_SINGLE, TCM_AE_CONE, ""] {
            let mut e = radius_effect("Short", Some("0.5"));
            e.target_collection_method = tcm.to_string();
            assert_eq!(splash_of(&e), None, "{tcm}");
        }
    }

    /// A missing, unparseable, zero, negative, non-finite or above-1
    /// fraction splashes nothing: splash damage is always a reduced share.
    /// Each one warns `ammo_splash_bad_fraction`, so a bad seed row shows in
    /// SigNoz instead of silently losing the splash.
    #[test]
    fn a_bad_fraction_does_not_splash() {
        let logs = crate::test_support::LogCapture::install();
        let bad = [
            None,
            Some("x"),
            Some("0"),
            Some("-0.5"),
            Some("NaN"),
            Some("1.5"),
        ];
        for f in bad {
            assert_eq!(splash_of(&radius_effect("Short", f)), None, "{f:?}");
        }
        let warned = logs
            .all()
            .into_iter()
            .filter(|c| {
                c.target == "ammo"
                    && c.level == tracing::Level::WARN
                    && c.has_field("event", "ammo_splash_bad_fraction")
                    && c.has_field("effect_id", "9130")
            })
            .count();
        assert_eq!(warned, bad.len());
    }

    /// The Explosive seed (`ammo_modifiers_explosive.sql`) loads through the
    /// startup loaders with the numbers `damage_apply::ammo_splash_tests` (in
    /// `cimmeria-cell-combat`) assumes: the row names effect 9130, and 9130
    /// reads as a 5 m splash of half the shot. Fails if the seed or its `\ir`
    /// line is dropped, or a number drifts.
    #[tokio::test]
    async fn live_db_explosive_seed_rows() {
        let pool = crate::test_support::require_db_or_skip!();
        let catalog = crate::cell::spawner::load_ammo_catalog(&pool)
            .await
            .expect("ammo catalog loads");
        assert_eq!(
            catalog.modifier(BULLET_EXPLOSIVE),
            Some(&AmmoModifier {
                ammo_type: BULLET_EXPLOSIVE,
                damage_mult: 1.1,
                penetration_mult: 0.5,
                damage_type: Some(i32::from(DT_PHYSICAL)),
                on_hit_effect_id: Some(9130),
                toggle_ability_id: 1446,
                beneficial: false,
            })
        );
        let effects = crate::cell::spawner::load_effect_defs(&pool)
            .await
            .expect("effect defs load");
        let effect = effects.get(&9130).expect("effect 9130 is seeded");
        assert_eq!(effect.script_name, None, "the splash runs in the pipeline");
        assert!(!effect.is_pulsing(), "the splash is one burst");
        assert_eq!(
            splash_of(effect),
            Some(Splash {
                effect_id: 9130,
                radius: 5.0,
                fraction: 0.5,
            })
        );
        let name: String =
            sqlx::query_scalar("SELECT name FROM resources.abilities WHERE ability_id = 1446")
                .fetch_one(&pool)
                .await
                .expect("ability 1446");
        assert_eq!(name, "Explosive Ammunition");
    }
}
