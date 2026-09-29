//! Special-ammo effect scripts, packet AM-08 (ammo campaign, issue #1026):
//! Incendiary rounds (toggle ability 723), the on-hit effect.
//!
//! Created empty by AM-F so each family packet adds its script here and one
//! `match` arm in `registry.rs`, and never edits `effects/mod.rs`.
//!
//! # No new script
//!
//! Incendiary needs no code of its own. Its `resources.ammo_modifiers` row
//! (`db/resources/Abilities/Seed/ammo_modifiers_incendiary.sql`) turns the
//! shot into energy damage at nominal damage and penetration, and names the
//! on-hit burn, effect 9110. The burn's `script_name` is the existing
//! [`RangedEnergyDamage`](super::scripts::RangedEnergyDamage), which takes
//! `FocusDamage` and `HealthDamage` from both pools with no Focus-first
//! gate, and its `pulse_count` of 4 makes it a DoT through the ordinary
//! pulsing machinery (`damage_apply` fires the first pulse on the hit and
//! registers the other three). So `registry.rs` has no arm for this family.
//!
//! Stacking is the effects ADR's decision 4, unchanged: a second hit from the
//! same shooter refreshes the burn (the remaining pulses never shrink, and
//! the next pulse is pushed a full interval out), and a hit from another
//! shooter adds a burn of their own. A shooter who keeps firing therefore
//! burns the target for one pulse per hit, and the three-pulse tail runs
//! when they stop.
//!
//! This file holds the reconstructed numbers as constants, so the tests of
//! both crates share one copy and the live-DB guard pins the seed to them.
//! Every number is RECONSTRUCTION except the damage type: ability 723's
//! cooked text is "Damage Type: Energy / Penetration: Nominal / Damage:
//! Nominal" and its `effect_ids` is empty.

use std::collections::HashMap;

use cimmeria_entity::abilities::{EffectDef, DT_ENERGY};
use cimmeria_entity::ammo_type::BULLET_INCENDIARY;

use crate::cell::spawner::AmmoModifier;

/// Toggle ability 723, "Incendiary Ammunition": provenance only (D-AM07).
pub const INCENDIARY_TOGGLE_ABILITY: i32 = 723;
/// The on-hit burn, the first id of AM-08's reserved block 9110-9119.
pub const INCENDIARY_BURN_EFFECT: i32 = 9110;
/// The existing script the burn runs each pulse.
pub const INCENDIARY_BURN_SCRIPT: &str = "RangedEnergyDamage";
/// Pulses per burn, the first on the hit. RECONSTRUCTION.
pub const INCENDIARY_BURN_PULSES: i32 = 4;
/// Seconds between pulses. RECONSTRUCTION.
pub const INCENDIARY_BURN_PULSE_SECS: f32 = 1.0;
/// Focus each pulse burns: Flame BC 2852's 5:1 Focus-to-Health ratio at a
/// tenth of its size. RECONSTRUCTION.
pub const INCENDIARY_BURN_FOCUS_DAMAGE: i32 = 15;
/// Health each pulse burns. RECONSTRUCTION.
pub const INCENDIARY_BURN_HEALTH_DAMAGE: i32 = 3;
/// The burn's `EffectCategory` NVP value, the client's `EFFECT_Burning`
/// category. Required: AM-11c's Antidote and Coagulant darts
/// (`RemoveEffects`) cleanse an effect by this NVP, so a burn without it
/// could not be cured. The damage script does not read it.
pub const INCENDIARY_BURN_CATEGORY: &str = "Burning";

/// The seeded `Bullet_Incendiary` modifier row: energy damage, nominal damage
/// and penetration, the burn on a hit.
pub fn incendiary_modifier() -> AmmoModifier {
    AmmoModifier {
        ammo_type: BULLET_INCENDIARY,
        damage_mult: 1.0,
        penetration_mult: 1.0,
        damage_type: Some(i32::from(DT_ENERGY)),
        on_hit_effect_id: Some(INCENDIARY_BURN_EFFECT),
        toggle_ability_id: INCENDIARY_TOGGLE_ABILITY,
        beneficial: false,
    }
}

/// The seeded burn, effect 9110, as `load_effect_defs` builds it.
pub fn incendiary_burn_effect() -> EffectDef {
    let params = HashMap::from([
        (
            "FocusDamage".to_string(),
            INCENDIARY_BURN_FOCUS_DAMAGE.to_string(),
        ),
        (
            "HealthDamage".to_string(),
            INCENDIARY_BURN_HEALTH_DAMAGE.to_string(),
        ),
        (
            "EffectCategory".to_string(),
            INCENDIARY_BURN_CATEGORY.to_string(),
        ),
    ]);
    EffectDef {
        effect_id: INCENDIARY_BURN_EFFECT,
        ability_id: INCENDIARY_TOGGLE_ABILITY,
        script_name: Some(INCENDIARY_BURN_SCRIPT.to_string()),
        pulse_count: INCENDIARY_BURN_PULSES,
        pulse_duration: INCENDIARY_BURN_PULSE_SECS,
        params,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::effects::ammo_damage::shot_ammo;
    use crate::cell::effects::test_fixtures::make_mgr_with_target;
    use crate::cell::effects::{dispatch_by_name, EffectContext};
    use crate::cell::space_manager::SpaceManager;
    use crate::cell::spawner::AmmoCatalog;
    use cimmeria_entity::abilities::{AbilityDef, DT_PHYSICAL};
    use cimmeria_entity::ammo_type::BULLET_DEFAULT;
    use cimmeria_entity::cell_entity::BandolierItem;
    use cimmeria_entity::stats::{FOCUS, HEALTH};

    fn shot_ability() -> AbilityDef {
        AbilityDef {
            ability_id: 592,
            name: "Pistol Shot".to_string(),
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

    /// Player 1 with a pistol loaded with Incendiary, the row and the burn
    /// loaded.
    fn mgr_incendiary() -> SpaceManager {
        let mut mgr = make_mgr_with_target();
        mgr.ammo_catalog =
            AmmoCatalog::from_rows([incendiary_modifier()], [(BULLET_INCENDIARY, 9002)]);
        mgr.effect_defs
            .insert(INCENDIARY_BURN_EFFECT, incendiary_burn_effect());
        let e = mgr.get_entity_mut(1).unwrap();
        e.active_bandolier_slot = 0;
        e.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 1,
                item_id: 3241,
                clip_size: 15,
                default_ammo_type: BULLET_DEFAULT,
                current_ammo: 15,
                cur_ammo_type: BULLET_INCENDIARY,
            },
        );
        mgr
    }

    /// The modifier math: an Incendiary shot keeps its damage and its
    /// penetration (both "Nominal"), lands as energy damage, and carries the
    /// burn. Fails if the row's numbers or its damage type drift.
    #[test]
    fn incendiary_shot_is_nominal_energy_damage_with_the_burn() {
        let mgr = mgr_incendiary();
        let shot = shot_ammo(&mgr, 1, Some(&shot_ability()), true).expect("modified");
        assert_eq!(shot.ammo_type, BULLET_INCENDIARY);
        assert!((shot.damage_scale() - 1.0).abs() < 1e-9);
        assert!((shot.penetration_mult() - 1.0).abs() < 1e-9);
        assert_eq!(shot.damage_type(DT_PHYSICAL), DT_ENERGY);
        assert_eq!(shot.on_hit_effect_id(&mgr), Some(INCENDIARY_BURN_EFFECT));
    }

    /// One burn pulse through the registry takes exactly the NVPs from both
    /// pools. Fails if the burn names a script that is not registered (the
    /// pulse would fall back to nothing) or one that gates Health behind
    /// Focus.
    #[test]
    fn a_burn_pulse_takes_both_pools_through_the_registry() {
        let mut mgr = mgr_incendiary();
        let burn = incendiary_burn_effect();
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &burn,
            space_mgr: &mut mgr,
        };
        assert!(
            dispatch_by_name(INCENDIARY_BURN_SCRIPT, &mut ctx),
            "the burn's script must be registered"
        );
        let e = mgr.get_entity(1).unwrap();
        // make_mgr_with_target: HEALTH 50, FOCUS 200. Focus is not empty, so
        // a Focus-first script would have left Health untouched.
        assert_eq!(
            e.stats.get(HEALTH).unwrap().cur,
            50 - INCENDIARY_BURN_HEALTH_DAMAGE
        );
        assert_eq!(
            e.stats.get(FOCUS).unwrap().cur,
            200 - INCENDIARY_BURN_FOCUS_DAMAGE
        );
    }

    /// The burn is a DoT: it pulses, for four seconds.
    #[test]
    fn the_burn_is_a_four_pulse_dot() {
        let burn = incendiary_burn_effect();
        assert!(burn.is_pulsing());
        assert!((burn.total_duration() - 4.0).abs() < 1e-6);
    }

    /// `ammo_modifiers_incendiary.sql` loads the row and the burn through the
    /// startup loaders with exactly the constants above, and ability 723
    /// exists. Fails if the seed or its `\ir` line is dropped or a number
    /// drifts.
    #[tokio::test]
    async fn live_db_incendiary_seed_rows() {
        let pool = crate::test_support::require_db_or_skip!();
        let catalog = crate::cell::spawner::load_ammo_catalog(&pool)
            .await
            .expect("ammo catalog loads");
        assert_eq!(
            catalog.modifier(BULLET_INCENDIARY),
            Some(&incendiary_modifier())
        );
        let defs = crate::cell::spawner::load_effect_defs(&pool)
            .await
            .expect("effect defs load");
        let burn = defs
            .get(&INCENDIARY_BURN_EFFECT)
            .expect("effect 9110 is seeded");
        let want = incendiary_burn_effect();
        assert_eq!(burn.ability_id, want.ability_id);
        assert_eq!(burn.script_name, want.script_name);
        assert_eq!(burn.pulse_count, want.pulse_count);
        assert!((burn.pulse_duration - want.pulse_duration).abs() < 1e-6);
        assert_eq!(burn.target_collection_method, want.target_collection_method);
        assert_eq!(burn.flags, 0);
        assert_eq!(
            burn.params.get("EffectCategory").map(String::as_str),
            Some(INCENDIARY_BURN_CATEGORY),
            "the burn must carry EffectCategory=Burning so Antidote can cleanse it"
        );
        assert_eq!(burn.params, want.params);
        let name: String =
            sqlx::query_scalar("SELECT name FROM resources.abilities WHERE ability_id = 723")
                .fetch_one(&pool)
                .await
                .expect("ability 723 exists");
        assert_eq!(name, "Incendiary Ammunition");
    }
}
