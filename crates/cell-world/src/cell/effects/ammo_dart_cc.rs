//! Special-ammo effect scripts, packet AM-11a (ammo campaign, issue #1026):
//! crowd-control darts (Poison, Disease, Tranquilizer).
//!
//! Created empty by AM-F so each family packet adds its script here and one
//! `match` arm in `registry.rs`, and never edits `effects/mod.rs`.
//!
//! # What each dart does on a hit
//!
//! The rows live in `db/resources/Abilities/Seed/ammo_modifiers_dart_cc.sql`
//! (RECONSTRUCTION; the numbers are recorded in the seed and in
//! `docs/analysis/ammo/worknotes/AM-11a.md`):
//!
//! | Ammo | On-hit effect | Script |
//! |---|---|---|
//! | `Dart_Poison` | 9140, a short, strong damage over time | the existing `Suppression` (a per-pulse HEALTH chip) |
//! | `Dart_Disease` | 9141, a long, weak damage over time | `Suppression` |
//! | `Dart_Tranquilizer` | 9142, a timed slow | [`MovementSlow`], below |
//!
//! The Tranquilizer is a slow, not a stun. The existing `Stun` script sets
//! `BSF_MOVEMENT_LOCK`, which the NPC AI never reads, and it leaks the flag's
//! refcount when its effect pulses (issue #1049). The NPC movement tick and
//! the client both scale speed by `MOVEMENT_SPEED_MOD`, so lowering that stat
//! slows NPCs and players alike.

use cimmeria_entity::stats::MOVEMENT_SPEED_MOD;

use super::{EffectContext, EffectScript};

/// `effect_nvps` name for how many points [`MovementSlow`] takes off
/// `MOVEMENT_SPEED_MOD` (100 = normal speed, so 40 leaves 60%).
pub const SPEED_REDUCTION_NVP: &str = "SpeedReduction";

/// Lowers the target's `MOVEMENT_SPEED_MOD` by the effect's
/// [`SPEED_REDUCTION_NVP`] for the effect's duration, then puts it back.
///
/// Give the effect `pulse_count > 1` and a `pulse_duration`. That makes the
/// pulse layer register an active-effect instance whose expiry calls
/// [`MovementSlow::on_remove`].
///
/// **One slow per effect per target.** The pulse layer calls `on_apply` on
/// the hit, again on every pulse, and again when the same shooter re-hits
/// and refreshes the instance. So `on_apply` slows only when no instance of
/// this effect is on the target yet: on the hit, `damage_apply` runs the
/// script before it registers the instance. `on_remove` restores only when
/// no instance of this effect is left, because every removal path (expiry,
/// duel strip, channel cancel) drops the instance before calling it. Two
/// shooters therefore share one slow that ends with the last of their
/// instances, instead of stacking. Stacking two slows would need a record of
/// what each one actually took off; nothing asks for that.
pub struct MovementSlow;

/// Whether `target_id` carries an active instance of `effect_id`, from any
/// invoker.
fn has_instance(ctx: &EffectContext, effect_id: i32) -> bool {
    ctx.space_mgr
        .get_entity(ctx.target_id)
        .is_some_and(|e| e.active_effects.iter().any(|i| i.effect_id == effect_id))
}

impl EffectScript for MovementSlow {
    fn on_apply(&self, ctx: &mut EffectContext) {
        let reduction = ctx.effect.param_i32(SPEED_REDUCTION_NVP);
        if reduction <= 0 {
            tracing::warn!(
                target: "abilities",
                event = "movement_slow_no_reduction",
                source_id = ctx.source_id,
                target_id = ctx.target_id,
                effect_id = ctx.effect.effect_id,
                reduction,
                "MovementSlow effect has no positive SpeedReduction NVP; nothing slowed"
            );
            return;
        }
        let effect_id = ctx.effect.effect_id;
        if has_instance(ctx, effect_id) {
            // A pulse or a refresh of a slow that is already in force.
            tracing::trace!(
                target: "abilities",
                event = "movement_slow_already_active",
                source_id = ctx.source_id,
                target_id = ctx.target_id,
                effect_id,
                "MovementSlow already active on the target"
            );
            return;
        }
        let Some(stat) = ctx
            .space_mgr
            .get_entity_mut(ctx.target_id)
            .and_then(|t| t.stats.get_mut(MOVEMENT_SPEED_MOD))
        else {
            return;
        };
        let before = stat.cur;
        stat.change(-reduction);
        let after = stat.cur;
        tracing::info!(
            target: "abilities",
            event = "movement_slow_applied",
            source_id = ctx.source_id,
            target_id = ctx.target_id,
            effect_id,
            reduction,
            speed_mod_before = before,
            speed_mod_after = after,
            "MovementSlow applied"
        );
    }

    fn on_remove(&self, ctx: &mut EffectContext) {
        let reduction = ctx.effect.param_i32(SPEED_REDUCTION_NVP);
        if reduction <= 0 {
            return;
        }
        let effect_id = ctx.effect.effect_id;
        if has_instance(ctx, effect_id) {
            // Another shooter's instance still holds the slow.
            return;
        }
        let Some(stat) = ctx
            .space_mgr
            .get_entity_mut(ctx.target_id)
            .and_then(|t| t.stats.get_mut(MOVEMENT_SPEED_MOD))
        else {
            return;
        };
        let before = stat.cur;
        stat.change(reduction);
        let after = stat.cur;
        tracing::info!(
            target: "abilities",
            event = "movement_slow_expired",
            source_id = ctx.source_id,
            target_id = ctx.target_id,
            effect_id,
            reduction,
            speed_mod_before = before,
            speed_mod_after = after,
            "MovementSlow expired; speed restored"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::effects::test_fixtures::{effect_with_nvp, make_mgr_with_target};
    use crate::cell::effects::{dispatch_by_name, dispatch_on_remove, registry};
    use cimmeria_entity::abilities::EffectDef;
    use cimmeria_entity::cell_entity::ActiveEffectInstance;

    fn slow_effect(reduction: i32) -> EffectDef {
        let mut e = effect_with_nvp(SPEED_REDUCTION_NVP, &reduction.to_string());
        e.script_name = Some("MovementSlow".to_string());
        e.pulse_count = 4;
        e.pulse_duration = 2.0;
        e
    }

    fn speed(mgr: &crate::cell::space_manager::SpaceManager) -> i32 {
        mgr.get_entity(1)
            .unwrap()
            .stats
            .get(MOVEMENT_SPEED_MOD)
            .unwrap()
            .cur
    }

    fn add_instance(
        mgr: &mut crate::cell::space_manager::SpaceManager,
        effect_id: i32,
        invoker: u32,
    ) {
        mgr.get_entity_mut(1)
            .unwrap()
            .active_effects
            .push(ActiveEffectInstance {
                effect_id,
                ability_id: 597,
                invoker_id: invoker,
                remaining_pulses: 3,
                total_pulses: 4,
                next_pulse_at: std::time::Instant::now(),
                pulse_interval_secs: 2.0,
                invoker_position_at_register: None,
            });
    }

    fn remove_instance(mgr: &mut crate::cell::space_manager::SpaceManager, invoker: u32) {
        mgr.get_entity_mut(1)
            .unwrap()
            .active_effects
            .retain(|i| i.invoker_id != invoker);
    }

    fn apply(mgr: &mut crate::cell::space_manager::SpaceManager, effect: &EffectDef, source: u32) {
        let mut ctx = EffectContext {
            source_id: source,
            target_id: 1,
            effect,
            space_mgr: mgr,
        };
        assert!(dispatch_by_name("MovementSlow", &mut ctx));
    }

    fn remove(mgr: &mut crate::cell::space_manager::SpaceManager, effect: &EffectDef, source: u32) {
        let mut ctx = EffectContext {
            source_id: source,
            target_id: 1,
            effect,
            space_mgr: mgr,
        };
        assert!(dispatch_on_remove("MovementSlow", &mut ctx));
    }

    #[test]
    fn registry_resolves_movement_slow() {
        assert!(registry::lookup("MovementSlow").is_some());
    }

    /// The hit slows, the pulses and a same-shooter refresh do not slow
    /// again, and expiry restores exactly the starting speed.
    #[test]
    fn slow_applies_once_per_instance_and_restores_on_expiry() {
        let mut mgr = make_mgr_with_target();
        let effect = slow_effect(40);
        assert_eq!(speed(&mgr), 100);

        // The hit: no instance yet, so it slows.
        apply(&mut mgr, &effect, 7);
        assert_eq!(speed(&mgr), 60);
        // damage_apply then registers the instance.
        add_instance(&mut mgr, effect.effect_id, 7);
        // Three pulses and a refresh re-run on_apply: no further slow.
        for _ in 0..4 {
            apply(&mut mgr, &effect, 7);
        }
        assert_eq!(speed(&mgr), 60, "pulses and refreshes must not stack");

        // Expiry: the sweep drops the instance, then calls on_remove.
        remove_instance(&mut mgr, 7);
        remove(&mut mgr, &effect, 7);
        assert_eq!(speed(&mgr), 100, "expiry restores the original speed");
    }

    /// Two shooters share one slow; it ends with the last instance.
    #[test]
    fn two_shooters_share_one_slow_until_the_last_instance_goes() {
        let mut mgr = make_mgr_with_target();
        let effect = slow_effect(40);
        apply(&mut mgr, &effect, 7);
        add_instance(&mut mgr, effect.effect_id, 7);
        apply(&mut mgr, &effect, 8);
        add_instance(&mut mgr, effect.effect_id, 8);
        assert_eq!(speed(&mgr), 60);

        remove_instance(&mut mgr, 7);
        remove(&mut mgr, &effect, 7);
        assert_eq!(speed(&mgr), 60, "shooter 8's instance still holds the slow");

        remove_instance(&mut mgr, 8);
        remove(&mut mgr, &effect, 8);
        assert_eq!(speed(&mgr), 100);
    }

    #[test]
    fn missing_or_zero_reduction_changes_nothing() {
        let mut mgr = make_mgr_with_target();
        for reduction in [0, -30] {
            let effect = slow_effect(reduction);
            apply(&mut mgr, &effect, 7);
            assert_eq!(speed(&mgr), 100);
            remove(&mut mgr, &effect, 7);
            assert_eq!(speed(&mgr), 100);
        }
    }

    /// The seed of `ammo_modifiers_dart_cc.sql`, `ammo_dart_widening.sql`
    /// and `ammo_dart_loot.sql`, read through the startup loaders. The
    /// numbers are the ones `damage_apply::ammo_dart_cc_tests` (in
    /// `cimmeria-cell-combat`) fires with. Fails if any of the three files or
    /// its `\ir` line is dropped, or a number drifts.
    #[tokio::test]
    async fn live_db_dart_cc_seed_rows() {
        use crate::cell::spawner::{
            load_ammo_catalog, load_effect_defs, load_loot_tables, AmmoModifier,
        };
        use cimmeria_entity::abilities::DT_PHYSICAL;
        use cimmeria_entity::ammo_type::{DART_DISEASE, DART_POISON, DART_TRANQUILIZER};

        let pool = crate::test_support::require_db_or_skip!();

        // The modifier rows.
        let catalog = load_ammo_catalog(&pool).await.expect("ammo catalog loads");
        for (ammo_type, effect, toggle) in [
            (DART_POISON, 9140, 990),
            (DART_DISEASE, 9141, 991),
            (DART_TRANQUILIZER, 9142, 998),
        ] {
            assert_eq!(
                catalog.modifier(ammo_type),
                Some(&AmmoModifier {
                    ammo_type,
                    damage_mult: 1.0,
                    penetration_mult: 1.0,
                    damage_type: Some(i32::from(DT_PHYSICAL)),
                    on_hit_effect_id: Some(effect),
                    toggle_ability_id: toggle,
                    beneficial: false,
                }),
                "ammo type {ammo_type}"
            );
        }

        // The on-hit effects: script, duration and nvps.
        let effects = load_effect_defs(&pool).await.expect("effect defs load");
        let effect = |id: i32| effects.get(&id).unwrap_or_else(|| panic!("effect {id}"));
        let category = |id: i32| effect(id).params.get("EffectCategory").cloned();

        let poison = effect(9140);
        assert_eq!(poison.script_name.as_deref(), Some("Suppression"));
        assert_eq!((poison.pulse_count, poison.pulse_duration), (5, 2.0));
        assert_eq!(poison.param_i32("HealthDamage"), 4);
        // AM-11c's Antidote cleanses by this nvp; it is part of the contract.
        assert_eq!(category(9140).as_deref(), Some("Poison"));

        let disease = effect(9141);
        assert_eq!(disease.script_name.as_deref(), Some("Suppression"));
        assert_eq!((disease.pulse_count, disease.pulse_duration), (10, 2.0));
        assert_eq!(disease.param_i32("HealthDamage"), 2);
        assert_eq!(category(9141).as_deref(), Some("Disease"));

        let tranq = effect(9142);
        assert_eq!(tranq.script_name.as_deref(), Some("MovementSlow"));
        assert!(tranq.is_pulsing(), "the slow needs an instance to expire");
        assert_eq!((tranq.pulse_count, tranq.pulse_duration), (4, 2.0));
        assert_eq!(tranq.param_i32(SPEED_REDUCTION_NVP), 40);
        assert_eq!(category(9142), None);

        // The provenance abilities exist.
        let names: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM resources.abilities WHERE ability_id IN (990, 991, 998) \
             ORDER BY ability_id",
        )
        .fetch_all(&pool)
        .await
        .expect("abilities query");
        assert_eq!(
            names,
            [
                "Dart Type: Hazardous: Poison",
                "Dart Type: Hazardous: Disease",
                "Dart Type: Hazardous: Disorient",
            ]
        );

        // The widening: all 19 dart guns take all ten dart specials.
        let guns: Vec<(i32, Vec<String>)> = sqlx::query_as(
            "SELECT item_id, ammo_types::text[] FROM resources.items \
             WHERE 'Dart_Default' = ANY (ammo_types) ORDER BY item_id",
        )
        .fetch_all(&pool)
        .await
        .expect("dart guns query");
        assert_eq!(guns.len(), 19, "dart guns: {guns:?}");
        let specials = [
            "Dart_Poison",
            "Dart_Disease",
            "Dart_Tranquilizer",
            "Dart_EMP",
            "Dart_Radioactive",
            "Dart_Stim",
            "Dart_Coagulant",
            "Dart_Nanites",
            "Dart_Antidote",
            "Dart_Adrenaline",
        ];
        for (item_id, types) in &guns {
            for s in specials {
                assert!(
                    types.iter().any(|t| t == s),
                    "gun {item_id} lacks {s}: {types:?}"
                );
            }
            assert_eq!(
                types.len(),
                11,
                "gun {item_id} holds a duplicate: {types:?}"
            );
        }

        // The debug crate: the dart gun plus a stack of each dart special,
        // at probability 1 and never above the item's stack cap (#1045).
        let tables = load_loot_tables(&pool).await.expect("loot tables load");
        let crate3 = tables.get(&3).expect("loot table 3");
        let caps: Vec<(i32, i32)> = sqlx::query_as(
            "SELECT item_id, max_stack_size FROM resources.items \
             WHERE item_id = 3584 OR item_id BETWEEN 9005 AND 9014",
        )
        .fetch_all(&pool)
        .await
        .expect("stack caps query");
        assert_eq!(caps.len(), 11);
        for (item_id, cap) in caps {
            let row = crate3
                .iter()
                .find(|e| e.design_id == Some(item_id))
                .unwrap_or_else(|| panic!("table 3 lacks item {item_id}"));
            assert_eq!(row.probability, 1.0, "item {item_id}");
            assert!(
                row.min_quantity >= 1 && row.max_quantity <= cap,
                "item {item_id}"
            );
        }
    }

    #[test]
    fn missing_target_is_a_no_op() {
        let mut mgr = make_mgr_with_target();
        let effect = slow_effect(40);
        let mut ctx = EffectContext {
            source_id: 7,
            target_id: 4242,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        MovementSlow.on_apply(&mut ctx);
        MovementSlow.on_remove(&mut ctx);
        assert_eq!(speed(&mgr), 100);
    }
}
