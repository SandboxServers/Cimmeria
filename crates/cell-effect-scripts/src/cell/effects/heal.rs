//! The two pool heals, [`HealHealth`] and [`HealFocus`].
//!
//! Each reads one of two NVPs:
//!
//! - **`HealAmount`**, a flat number of points (the consumable heals: effect
//!   712 "Heals 500 health.", 3062 "Heals 384 Focus", ...). The 2009 rows
//!   shipped no NVPs for these, so each `HealAmount` row in
//!   `db/resources/Effects/Seed/effect_nvps.sql` is the number in its
//!   effect's own `effect_desc`.
//! - **`HealPercentage`**, a percentage of the pool's max (597 Heal Focus,
//!   the Kelnorim and pet heals, python's `HealHealth.py`).
//!
//! `HealAmount` wins when it is positive. A zero, negative or unparseable
//! `HealAmount` is ignored, so it can never damage, and the effect falls
//! back to `HealPercentage` exactly as before. The result is clamped at the
//! pool's max. One script per pool keeps every heal on one code path.
//!
//! Split out of `scripts.rs`, which is over the file cap; `scripts`
//! re-exports both so `scripts::HealHealth` still resolves.

use cimmeria_entity::stats::{FOCUS, HEALTH};

use super::{EffectContext, EffectScript};

/// Heals the target's HEALTH by `HealAmount` points, or else by
/// `HealPercentage`% of its max.
///
/// Reference: deprecated/python (fan-server) `HealHealth.py`.
pub struct HealHealth;

impl EffectScript for HealHealth {
    fn on_apply(&self, ctx: &mut EffectContext) {
        heal_pool(ctx, HEALTH, "HealHealth", "heal_health");
    }
}

/// Heals the target's FOCUS by `HealAmount` points, or else by
/// `HealPercentage`% of its max.
///
/// The canonical percentage user is Heal Focus (ability 597), which the
/// starter loadout grants to every archetype.
pub struct HealFocus;

impl EffectScript for HealFocus {
    fn on_apply(&self, ctx: &mut EffectContext) {
        heal_pool(ctx, FOCUS, "HealFocus", "heal_focus");
    }
}

/// How much a heal effect restores, before the clamp at max.
#[derive(Debug, Clone, Copy, PartialEq)]
enum HealSize {
    /// `HealAmount` points.
    Flat(i32),
    /// `HealPercentage`% of the pool's max.
    Percent(f32),
}

/// The heal the effect's NVPs ask for, or `None` when neither is positive.
fn heal_size(ctx: &EffectContext) -> Option<HealSize> {
    let amount = ctx.effect.param_i32("HealAmount");
    if amount > 0 {
        return Some(HealSize::Flat(amount));
    }
    let percent = ctx.effect.param_f32("HealPercentage");
    (percent > 0.0).then_some(HealSize::Percent(percent))
}

fn heal_pool(ctx: &mut EffectContext, stat_id: i32, script: &'static str, event: &'static str) {
    let Some(size) = heal_size(ctx) else {
        tracing::debug!(
            target: "abilities",
            event = "heal_skipped_zero_percent",
            cast_id = ctx.row_ids().cast_id,
            account_id = ctx.row_ids().account_id,
            player_id = ctx.row_ids().player_id,
            target_player_id = ctx.row_ids().target_player_id,
            script,
            effect_id = ctx.effect.effect_id,
            source_id = ctx.source_id,
            target_id = ctx.target_id,
            "{script}: neither HealAmount nor HealPercentage is positive, no-op"
        );
        return;
    };
    let Some(target) = ctx.space_mgr.get_entity_mut(ctx.target_id) else {
        tracing::debug!(
            target: "abilities",
            event = "heal_skipped_no_target",
            cast_id = ctx.row_ids().cast_id,
            account_id = ctx.row_ids().account_id,
            player_id = ctx.row_ids().player_id,
            target_player_id = ctx.row_ids().target_player_id,
            script,
            effect_id = ctx.effect.effect_id,
            target_id = ctx.target_id,
            "{script}: target entity missing, no-op"
        );
        return;
    };
    let Some(stat) = target.stats.get_mut(stat_id) else {
        return;
    };
    let (cur, max) = (stat.cur, stat.max);
    let delta = match size {
        HealSize::Flat(amount) => amount,
        HealSize::Percent(percent) => ((max as f32) * (percent / 100.0)).round() as i32,
    };
    let new_cur = cur.saturating_add(delta).min(max);
    stat.update(stat.min, new_cur, stat.max);
    let (mode, amount, percent) = match size {
        HealSize::Flat(amount) => ("flat", amount, 0.0),
        HealSize::Percent(percent) => ("percent", 0, percent),
    };
    tracing::info!(
        target: "abilities",
        event,
        cast_id = ctx.row_ids().cast_id,
        account_id = ctx.row_ids().account_id,
        player_id = ctx.row_ids().player_id,
        target_player_id = ctx.row_ids().target_player_id,
        source_id = ctx.source_id,
        target_id = ctx.target_id,
        effect_id = ctx.effect.effect_id,
        ability_id = ctx.effect.ability_id,
        mode,
        amount,
        percent,
        healed = (new_cur - cur).max(0),
        stat_before = cur,
        new_cur,
        max,
        "{script} applied"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::effects::test_fixtures::{effect_with_nvp, make_mgr_with_target};

    /// `(cur, max)` of `stat` on entity 1.
    fn pool(mgr: &crate::cell::space_manager::SpaceManager, stat: i32) -> (i32, i32) {
        let s = mgr.get_entity(1).unwrap().stats.get(stat).unwrap();
        (s.cur, s.max)
    }

    fn apply(script: &dyn EffectScript, name: &str, value: &str, target_id: u32) -> (i32, i32) {
        let mut mgr = make_mgr_with_target();
        let effect = effect_with_nvp(name, value);
        let mut ctx = EffectContext {
            source_id: 1,
            target_id,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        script.on_apply(&mut ctx);
        let health = pool(&mgr, HEALTH).0;
        let focus = pool(&mgr, FOCUS).0;
        (health, focus)
    }

    // The fixture starts at HEALTH 50/100 and FOCUS 200/1000.

    #[test]
    fn heal_health_35_percent_of_max() {
        // 50 + (100 * 0.35 = 35) = 85
        assert_eq!(apply(&HealHealth, "HealPercentage", "35.00", 1).0, 85);
    }

    #[test]
    fn heal_health_percent_caps_at_max() {
        let mut mgr = make_mgr_with_target();
        mgr.get_entity_mut(1)
            .unwrap()
            .stats
            .get_mut(HEALTH)
            .unwrap()
            .update(0, 90, 100);
        let effect = effect_with_nvp("HealPercentage", "50.00");
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        HealHealth.on_apply(&mut ctx);
        // 90 + 50 = 140 but capped at max=100
        assert_eq!(pool(&mgr, HEALTH), (100, 100));
    }

    #[test]
    fn heal_focus_35_percent_of_max() {
        // 200 + (1000 * 0.35 = 350) = 550
        assert_eq!(apply(&HealFocus, "HealPercentage", "35.00", 1).1, 550);
    }

    #[test]
    fn zero_percent_heal_is_noop() {
        assert_eq!(apply(&HealFocus, "HealPercentage", "0.00", 1).1, 200);
    }

    #[test]
    fn heal_health_flat_amount_adds_that_many_points() {
        assert_eq!(apply(&HealHealth, "HealAmount", "30", 1), (80, 200));
    }

    #[test]
    fn heal_focus_flat_amount_adds_that_many_points() {
        assert_eq!(apply(&HealFocus, "HealAmount", "384", 1), (50, 584));
    }

    #[test]
    fn flat_amount_that_would_overheal_clamps_at_max() {
        // Health Slappack TC1: 500 on a 50/100 pool.
        assert_eq!(apply(&HealHealth, "HealAmount", "500", 1).0, 100);
    }

    #[test]
    fn flat_amount_at_max_is_a_no_op() {
        let mut mgr = make_mgr_with_target();
        mgr.get_entity_mut(1)
            .unwrap()
            .stats
            .get_mut(HEALTH)
            .unwrap()
            .update(0, 100, 100);
        let effect = effect_with_nvp("HealAmount", "500");
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        HealHealth.on_apply(&mut ctx);
        assert_eq!(pool(&mgr, HEALTH), (100, 100), "never raises max");
    }

    #[test]
    fn zero_or_negative_flat_amount_never_damages() {
        for value in ["0", "-5", "abc"] {
            assert_eq!(
                apply(&HealHealth, "HealAmount", value, 1),
                (50, 200),
                "HealAmount {value}"
            );
        }
    }

    #[test]
    fn flat_amount_wins_over_percentage() {
        let mut mgr = make_mgr_with_target();
        let mut effect = effect_with_nvp("HealAmount", "10");
        effect
            .params
            .insert("HealPercentage".to_string(), "50.00".to_string());
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        HealHealth.on_apply(&mut ctx);
        assert_eq!(pool(&mgr, HEALTH).0, 60, "10 flat, not 50%");
    }

    #[test]
    fn a_non_positive_flat_amount_falls_back_to_percentage() {
        let mut mgr = make_mgr_with_target();
        let mut effect = effect_with_nvp("HealAmount", "0");
        effect
            .params
            .insert("HealPercentage".to_string(), "35.00".to_string());
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        HealHealth.on_apply(&mut ctx);
        assert_eq!(pool(&mgr, HEALTH).0, 85);
    }

    #[test]
    fn flat_amount_on_a_missing_target_is_a_no_op() {
        assert_eq!(apply(&HealHealth, "HealAmount", "500", 99_999), (50, 200));
        assert_eq!(apply(&HealFocus, "HealAmount", "500", 99_999), (50, 200));
    }
}
