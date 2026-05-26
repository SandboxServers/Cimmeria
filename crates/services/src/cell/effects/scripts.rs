//! Effect script implementations.
//!
//! Three scripts in v1 (#331 Phase 1):
//! - [`HealHealth`] — heals target's health by `HealPercentage` × max
//! - [`HealFocus`] — heals target's focus by `HealPercentage` × max
//! - [`MeleeDamage`] — does `HealthDamage` health damage via the damage pipeline
//!
//! ## Adding a new script
//!
//! 1. Add a zero-sized struct here with an `impl EffectScript`.
//! 2. Wire it into [`super::registry::lookup`].
//! 3. Add a unit test in this file covering the happy path + edge cases
//!    (missing target, zero/negative NVP, missing stat).
//! 4. Seed an effect row with `script_name = "YourScriptName"` if you
//!    want existing content to dispatch through it.

use super::{EffectContext, EffectScript};
use cimmeria_entity::stats::{FOCUS, HEALTH};

// ── HealHealth ───────────────────────────────────────────────────────────

/// Heals the target's HEALTH stat by `HealPercentage`% of its max.
///
/// Reads `HealPercentage` NVP (e.g., `"35.00"` = 35% of max). Negative
/// or missing percentage = no-op. Caps at `cur + delta <= max` because
/// the underlying stat clamps.
///
/// Reference: deprecated/python (fan-server) `HealHealth.py`.
pub struct HealHealth;

impl EffectScript for HealHealth {
    fn on_apply(&self, ctx: &mut EffectContext) {
        let percent = ctx.effect.param_f32("HealPercentage");
        if percent <= 0.0 {
            tracing::debug!(
                target: "abilities",
                event = "heal_skipped_zero_percent",
                effect_id = ctx.effect.effect_id,
                source_id = ctx.source_id,
                target_id = ctx.target_id,
                "HealHealth: HealPercentage <= 0, no-op"
            );
            return;
        }
        let target = match ctx.space_mgr.get_entity_mut(ctx.target_id) {
            Some(t) => t,
            None => {
                tracing::debug!(
                    target_id = ctx.target_id,
                    "HealHealth: target entity missing — no-op"
                );
                return;
            }
        };
        let (cur, max) = match target.stats.get(HEALTH) {
            Some(s) => (s.cur, s.max),
            None => return,
        };
        let delta = ((max as f32) * (percent / 100.0)).round() as i32;
        let new_cur = (cur + delta).min(max);
        if let Some(stat) = target.stats.get_mut(HEALTH) {
            stat.update(stat.min, new_cur, stat.max);
        }
        tracing::info!(
            target: "abilities",
            event = "heal_health",
            source_id = ctx.source_id,
            target_id = ctx.target_id,
            effect_id = ctx.effect.effect_id,
            percent,
            healed = delta.min(max - cur),
            new_cur,
            "HealHealth applied"
        );
    }
}

// ── HealFocus ────────────────────────────────────────────────────────────

/// Heals the target's FOCUS stat by `HealPercentage`% of its max.
///
/// Same pattern as [`HealHealth`] but targets `FOCUS` (stat id 8). The
/// canonical use case is Heal Focus (ability 597), which the existing
/// starter loadout grants to every archetype.
pub struct HealFocus;

impl EffectScript for HealFocus {
    fn on_apply(&self, ctx: &mut EffectContext) {
        let percent = ctx.effect.param_f32("HealPercentage");
        if percent <= 0.0 {
            tracing::debug!(
                target: "abilities",
                event = "heal_skipped_zero_percent",
                effect_id = ctx.effect.effect_id,
                source_id = ctx.source_id,
                target_id = ctx.target_id,
                "HealFocus: HealPercentage <= 0, no-op"
            );
            return;
        }
        let target = match ctx.space_mgr.get_entity_mut(ctx.target_id) {
            Some(t) => t,
            None => return,
        };
        let (cur, max) = match target.stats.get(FOCUS) {
            Some(s) => (s.cur, s.max),
            None => return,
        };
        let delta = ((max as f32) * (percent / 100.0)).round() as i32;
        let new_cur = (cur + delta).min(max);
        if let Some(stat) = target.stats.get_mut(FOCUS) {
            stat.update(stat.min, new_cur, stat.max);
        }
        tracing::info!(
            target: "abilities",
            event = "heal_focus",
            source_id = ctx.source_id,
            target_id = ctx.target_id,
            effect_id = ctx.effect.effect_id,
            percent,
            healed = delta.min(max - cur),
            new_cur,
            "HealFocus applied"
        );
    }
}

// ── MeleeDamage ──────────────────────────────────────────────────────────

/// Applies `HealthDamage` health damage to the target via direct stat
/// mutation.
///
/// V1 keeps this script intentionally simple — it doesn't route through
/// `damage_apply::apply_damage_to_target` (which does QR roll + threat +
/// death detection + wire packets) because that pipeline is already
/// triggered by the `use_ability` flow. The script exists for future
/// content (effect-driven secondary damage, channelled damage pulses)
/// that wants effect-NVP-driven damage without going through ability
/// resolution.
///
/// For the v1 hook point in `damage_apply`, the legacy `HealthDamage`
/// NVP path already covers melee abilities; effects with
/// `script_name = "MeleeDamage"` will run this in addition, applying
/// extra raw damage. Operators should set `script_name` only when the
/// script-driven behavior is the intended path.
pub struct MeleeDamage;

impl EffectScript for MeleeDamage {
    fn on_apply(&self, ctx: &mut EffectContext) {
        let damage = ctx.effect.param_i32("HealthDamage");
        if damage <= 0 {
            return;
        }
        let target = match ctx.space_mgr.get_entity_mut(ctx.target_id) {
            Some(t) => t,
            None => return,
        };
        let cur = match target.stats.get(HEALTH).map(|s| s.cur) {
            Some(v) => v,
            None => return,
        };
        let new_cur = (cur - damage).max(0);
        if let Some(stat) = target.stats.get_mut(HEALTH) {
            stat.update(stat.min, new_cur, stat.max);
        }
        tracing::info!(
            target: "abilities",
            event = "melee_damage",
            source_id = ctx.source_id,
            target_id = ctx.target_id,
            effect_id = ctx.effect.effect_id,
            damage,
            new_cur,
            "MeleeDamage applied"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::space_manager::SpaceManager;
    use cimmeria_entity::abilities::EffectDef;
    use std::collections::HashMap;

    fn make_mgr_with_target() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "W", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            // Seed health 50/100 (half damaged) so heal can show a delta;
            // focus 200/1000 so HealFocus has room.
            if let Some(s) = e.stats.get_mut(HEALTH) {
                s.update(0, 50, 100);
            }
            if let Some(s) = e.stats.get_mut(FOCUS) {
                s.update(0, 200, 1000);
            }
        }
        mgr
    }

    fn effect_with_nvp(name: &str, value: &str) -> EffectDef {
        let mut params = HashMap::new();
        params.insert(name.to_string(), value.to_string());
        EffectDef {
            effect_id: 999,
            ability_id: 597,
            delay: 0,
            effect_sequence: 0,
            event_set_id: None,
            script_name: None,
            params,
        }
    }

    #[test]
    fn heal_health_35_percent_of_max_caps_at_max() {
        let mut mgr = make_mgr_with_target();
        let effect = effect_with_nvp("HealPercentage", "35.00");
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        HealHealth.on_apply(&mut ctx);
        // 50 + (100 * 0.35 = 35) = 85
        let hp = ctx
            .space_mgr
            .get_entity(1)
            .unwrap()
            .stats
            .get(HEALTH)
            .unwrap()
            .cur;
        assert_eq!(hp, 85);
    }

    #[test]
    fn heal_health_caps_at_max() {
        let mut mgr = make_mgr_with_target();
        // Player already near full
        if let Some(e) = mgr.get_entity_mut(1) {
            if let Some(s) = e.stats.get_mut(HEALTH) {
                s.update(0, 90, 100);
            }
        }
        let effect = effect_with_nvp("HealPercentage", "50.00");
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        HealHealth.on_apply(&mut ctx);
        // 90 + 50 = 140 but capped at max=100
        let hp = ctx
            .space_mgr
            .get_entity(1)
            .unwrap()
            .stats
            .get(HEALTH)
            .unwrap()
            .cur;
        assert_eq!(hp, 100);
    }

    #[test]
    fn heal_focus_35_percent_of_max() {
        let mut mgr = make_mgr_with_target();
        let effect = effect_with_nvp("HealPercentage", "35.00");
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        HealFocus.on_apply(&mut ctx);
        // 200 + (1000 * 0.35 = 350) = 550
        let focus = ctx
            .space_mgr
            .get_entity(1)
            .unwrap()
            .stats
            .get(FOCUS)
            .unwrap()
            .cur;
        assert_eq!(focus, 550);
    }

    #[test]
    fn melee_damage_applies_health_damage() {
        let mut mgr = make_mgr_with_target();
        let effect = effect_with_nvp("HealthDamage", "20");
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        MeleeDamage.on_apply(&mut ctx);
        // 50 - 20 = 30
        let hp = ctx
            .space_mgr
            .get_entity(1)
            .unwrap()
            .stats
            .get(HEALTH)
            .unwrap()
            .cur;
        assert_eq!(hp, 30);
    }

    #[test]
    fn melee_damage_clamps_at_zero() {
        let mut mgr = make_mgr_with_target();
        // Player at low health
        if let Some(e) = mgr.get_entity_mut(1) {
            if let Some(s) = e.stats.get_mut(HEALTH) {
                s.update(0, 5, 100);
            }
        }
        let effect = effect_with_nvp("HealthDamage", "999");
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        MeleeDamage.on_apply(&mut ctx);
        let hp = ctx
            .space_mgr
            .get_entity(1)
            .unwrap()
            .stats
            .get(HEALTH)
            .unwrap()
            .cur;
        assert_eq!(hp, 0, "must clamp at zero, not go negative");
    }

    #[test]
    fn missing_target_is_noop() {
        let mut mgr = make_mgr_with_target();
        let effect = effect_with_nvp("HealPercentage", "35.00");
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 99999, // missing
            effect: &effect,
            space_mgr: &mut mgr,
        };
        HealHealth.on_apply(&mut ctx); // must not panic
        HealFocus.on_apply(&mut ctx);
        MeleeDamage.on_apply(&mut ctx);
        // Original target unchanged
        let hp = mgr.get_entity(1).unwrap().stats.get(HEALTH).unwrap().cur;
        assert_eq!(hp, 50);
    }

    #[test]
    fn zero_percent_heal_is_noop() {
        let mut mgr = make_mgr_with_target();
        let effect = effect_with_nvp("HealPercentage", "0.00");
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        HealFocus.on_apply(&mut ctx);
        let focus = ctx
            .space_mgr
            .get_entity(1)
            .unwrap()
            .stats
            .get(FOCUS)
            .unwrap()
            .cur;
        assert_eq!(focus, 200, "zero percent must not change stat");
    }
}
