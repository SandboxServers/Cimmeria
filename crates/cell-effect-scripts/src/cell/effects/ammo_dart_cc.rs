//! Special-ammo effect scripts, packet AM-11a (ammo campaign, issue #1026):
//! crowd-control darts (Poison, Disease, Tranquilizer).
//!
//! Created empty by AM-F so each family packet adds its script here and one
//! `EFFECT_SCRIPTS` row in `registry.rs`, and never edits `effects/mod.rs`.
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
//! The Tranquilizer is a slow, not a stun. Since ability mechanics AB-09b
//! the slow is a timed-ledger entry on `MOVEMENT_SPEED_MOD` (the stat the
//! NPC movement tick and the client both scale speed by), so it reverts by
//! exactly what it took even when another slow or a speed buff moved the
//! stat meanwhile. Before, it changed the stat directly and its restore
//! could overshoot.

use std::time::Instant;

use cimmeria_entity::cell_entity::{TimedEffectSpec, TimedStacking};
use cimmeria_entity::stats::MOVEMENT_SPEED_MOD;

use super::stat_buff::StatBuffRemoval;
use super::{EffectContext, EffectScript};

/// `effect_nvps` name for how many points [`MovementSlow`] takes off
/// `MOVEMENT_SPEED_MOD` (100 = normal speed, so 40 leaves 60%).
pub const SPEED_REDUCTION_NVP: &str = "SpeedReduction";

/// Lowers the target's `MOVEMENT_SPEED_MOD` by the effect's
/// [`SPEED_REDUCTION_NVP`] for the effect's duration, as one timed-ledger
/// entry, then puts back exactly what it took.
///
/// **One slow per effect per target** ([`TimedStacking::PerEffect`]): a
/// second shooter's hit, or a re-hit, refreshes the entry instead of
/// stacking, as before the ledger. A different slow (a Snare Shot) is a
/// different effect and stacks; each reverts its own delta.
///
/// Duration: one `pulse_duration` for a single-pulse row (the seeded 9142
/// is 6 s), `pulse_count x pulse_duration` for a pulsing one, whose
/// instance's `on_remove` ends it.
pub struct MovementSlow;

impl EffectScript for MovementSlow {
    fn on_apply(&self, ctx: &mut EffectContext) {
        let reduction = ctx.effect.param_i32(SPEED_REDUCTION_NVP);
        if reduction <= 0 {
            tracing::warn!(
                target: "abilities",
                event = "movement_slow_no_reduction",
                cast_id = ctx.row_ids().cast_id,
                account_id = ctx.row_ids().account_id,
                player_id = ctx.row_ids().player_id,
                target_player_id = ctx.row_ids().target_player_id,
                source_id = ctx.source_id,
                target_id = ctx.target_id,
                effect_id = ctx.effect.effect_id,
                reduction,
                "MovementSlow effect has no positive SpeedReduction NVP; nothing slowed"
            );
            return;
        }
        let effect = ctx.effect;
        let duration = match effect.pulse_count {
            0 => None,
            n => Some(n.max(1) as f32 * effect.pulse_duration.max(0.0)),
        };
        let spec = TimedEffectSpec {
            effect_id: effect.effect_id,
            ability_id: effect.ability_id,
            invoker_id: ctx.source_id,
            effect_flags: effect.flags,
            moniker_ids: ctx.space_mgr.ability_moniker_ids(effect.ability_id),
            stats: vec![(MOVEMENT_SPEED_MOD, -reduction)],
            duration_secs: duration,
            stacking: TimedStacking::PerEffect,
            ..Default::default()
        };
        let before = speed_mod(ctx);
        if ctx
            .space_mgr
            .apply_timed_effect(ctx.target_id, spec, Instant::now())
            .is_none()
        {
            return;
        }
        tracing::info!(
            target: "abilities",
            event = "movement_slow_applied",
            cast_id = ctx.row_ids().cast_id,
            account_id = ctx.row_ids().account_id,
            player_id = ctx.row_ids().player_id,
            target_player_id = ctx.row_ids().target_player_id,
            source_id = ctx.source_id,
            target_id = ctx.target_id,
            effect_id = effect.effect_id,
            reduction,
            duration_secs = duration.unwrap_or(0.0),
            speed_mod_before = before,
            speed_mod_after = speed_mod(ctx),
            "MovementSlow applied"
        );
    }

    /// A pulsing row's instance ended: its entry comes off (expiry of a
    /// single-pulse row is the ledger tick's, logged `stat_buff_removed`).
    fn on_remove(&self, ctx: &mut EffectContext) {
        let effect_id = ctx.effect.effect_id;
        let before = speed_mod(ctx);
        let removed =
            ctx.space_mgr
                .remove_timed_effects(ctx.target_id, StatBuffRemoval::Removed, |e| {
                    e.effect_id == effect_id
                });
        if removed.is_empty() {
            return;
        }
        tracing::info!(
            target: "abilities",
            event = "movement_slow_expired",
            cast_id = ctx.row_ids().cast_id,
            account_id = ctx.row_ids().account_id,
            player_id = ctx.row_ids().player_id,
            target_player_id = ctx.row_ids().target_player_id,
            source_id = ctx.source_id,
            target_id = ctx.target_id,
            effect_id,
            speed_mod_before = before,
            speed_mod_after = speed_mod(ctx),
            "MovementSlow expired; speed restored"
        );
    }
}

fn speed_mod(ctx: &EffectContext) -> Option<i32> {
    ctx.space_mgr
        .get_entity(ctx.target_id)
        .and_then(|t| t.stats.get(MOVEMENT_SPEED_MOD))
        .map(|s| s.cur)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::effects::test_fixtures::{effect_with_nvp, make_mgr_with_target};
    use crate::cell::effects::{dispatch_by_name, dispatch_on_remove, registry};
    use crate::cell::space_manager::SpaceManager;
    use cimmeria_entity::abilities::EffectDef;

    /// Effect 9142 as seeded: 40 points for one 6 s pulse.
    fn slow_effect(reduction: i32) -> EffectDef {
        let mut e = effect_with_nvp(SPEED_REDUCTION_NVP, &reduction.to_string());
        e.effect_id = 9142;
        e.script_name = Some("MovementSlow".to_string());
        e.pulse_count = 1;
        e.pulse_duration = 6.0;
        e
    }

    fn speed(mgr: &SpaceManager) -> i32 {
        mgr.get_entity(1)
            .unwrap()
            .stats
            .get(MOVEMENT_SPEED_MOD)
            .unwrap()
            .cur
    }

    fn apply(mgr: &mut SpaceManager, effect: &EffectDef, source: u32) {
        let mut ctx = EffectContext {
            source_id: source,
            target_id: 1,
            effect,
            space_mgr: mgr,
        };
        assert!(dispatch_by_name("MovementSlow", &mut ctx));
    }

    fn remove(mgr: &mut SpaceManager, effect: &EffectDef, source: u32) {
        let mut ctx = EffectContext {
            source_id: source,
            target_id: 1,
            effect,
            space_mgr: mgr,
        };
        assert!(dispatch_on_remove("MovementSlow", &mut ctx));
    }

    /// A snare (`TimedStat` on `MovementSpeedMod`) from another caster.
    fn snare(mgr: &mut SpaceManager, delta: i32, source: u32) {
        let mut effect = effect_with_nvp("MovementSpeedMod", &delta.to_string());
        effect.effect_id = 1462;
        effect.ability_id = 717;
        effect.script_name = Some("TimedStat".to_string());
        effect.pulse_count = 1;
        effect.pulse_duration = 15.0;
        let mut ctx = EffectContext {
            source_id: source,
            target_id: 1,
            effect: &effect,
            space_mgr: mgr,
        };
        assert!(dispatch_by_name("TimedStat", &mut ctx));
    }

    #[test]
    fn registry_resolves_movement_slow() {
        assert!(registry::lookup("MovementSlow").is_some());
    }

    /// The hit slows once, a re-hit (same or other shooter) refreshes
    /// without slowing again, and the removal restores exactly the starting
    /// speed.
    #[test]
    fn slow_applies_once_per_effect_and_restores_on_removal() {
        let mut mgr = make_mgr_with_target();
        let effect = slow_effect(40);
        assert_eq!(speed(&mgr), 100);
        apply(&mut mgr, &effect, 7);
        assert_eq!(speed(&mgr), 60);
        apply(&mut mgr, &effect, 7);
        apply(&mut mgr, &effect, 8);
        assert_eq!(speed(&mgr), 60, "re-hits must not stack");
        let entries = &mgr.get_entity(1).unwrap().stat_buffs.entries;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].duration_secs, 6.0);
        remove(&mut mgr, &effect, 8);
        assert_eq!(speed(&mgr), 100, "removal restores the original speed");
    }

    /// **Regression guard (the MovementSlow overshoot).** A -70 snare and
    /// the -40 dart stacked on 100 drive the stat below its 0 floor; each
    /// reverts exactly what it took, in either order, back to 100. The old
    /// script moved the stat directly: its -40 clamped to -30 at the floor
    /// but its restore added the full 40, ending at 110 (`speed after both`
    /// fails).
    #[test]
    fn stacked_slows_revert_exactly_in_either_order() {
        for dart_first_off in [true, false] {
            let mut mgr = make_mgr_with_target();
            let dart = slow_effect(40);
            snare(&mut mgr, -70, 9);
            apply(&mut mgr, &dart, 7);
            assert!(speed(&mgr) <= 0, "both slows in force: {}", speed(&mgr));
            if dart_first_off {
                remove(&mut mgr, &dart, 7);
                assert_eq!(speed(&mgr), 30, "the snare alone");
                let _ = mgr.remove_timed_effects(1, StatBuffRemoval::Expired, |_| true);
            } else {
                let _ =
                    mgr.remove_timed_effects(1, StatBuffRemoval::Expired, |e| e.effect_id == 1462);
                assert_eq!(speed(&mgr), 60, "the dart alone");
                remove(&mut mgr, &dart, 7);
            }
            assert_eq!(
                speed(&mgr),
                100,
                "speed after both (dart first: {dart_first_off})"
            );
        }
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
        // A single 6 s ledger entry since AB-09b (was 4 pulses 2 s apart).
        assert_eq!((tranq.pulse_count, tranq.pulse_duration), (1, 6.0));
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
