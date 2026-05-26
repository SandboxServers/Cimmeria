//! Effect script implementations.
//!
//! v1 (#331 Phase 1) — single-shot stat scripts:
//! - [`HealHealth`] / [`HealFocus`] — heal by `HealPercentage` × max
//! - [`MeleeDamage`] — `HealthDamage` raw damage
//!
//! v2 (#47 / #419 Phase F+G) — buff/debuff scripts:
//! - [`AbsorbShield`] — adds `ShieldAmount` to the matching ABSORB_*
//!   pool so subsequent damage of that type is consumed from the
//!   shield before HEALTH
//! - [`Stun`] — sets `BSF_MOVEMENT_LOCK` on the target; cleared by
//!   the active-effect expiry sweep
//! - [`Suppression`] — flat reduction to movement speed via the
//!   `MOVE_SPEED_MOD` stat
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
use crate::cell::combat::state::BSF_MOVEMENT_LOCK;
use cimmeria_entity::abilities::{DT_ENERGY, DT_HAZMAT, DT_PHYSICAL, DT_PSIONIC, DT_UNTYPED};
use cimmeria_entity::stats::{
    ABSORB_ENERGY, ABSORB_HAZMAT, ABSORB_PHYSICAL, ABSORB_PSIONIC, ABSORB_UNTYPED, FOCUS, HEALTH,
};

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

// ── AbsorbShield ─────────────────────────────────────────────────────────

/// Adds `ShieldAmount` to the target's matching ABSORB_* pool. Damage
/// of that type will then drain the pool before bleeding through to
/// HEALTH (see `cell::combat::damage::drain_absorption_pools`).
///
/// NVPs:
///   - `ShieldAmount` (i32, required) — capacity to grant
///   - `ShieldType`   (i32, optional) — damage type the shield blocks
///     (DT_PHYSICAL/ENERGY/HAZMAT/PSIONIC/UNTYPED). Defaults to UNTYPED.
///
/// The shield pool persists on the target's StatList until damage
/// drains it OR a pulsing instance carries the script and expires.
/// For "buff duration" semantics, register the effect as pulsing with
/// `pulse_count = 1` and `pulse_duration = <buff_seconds>` — the
/// pulse fires once on apply, then the instance ages out at expiry.
pub struct AbsorbShield;

/// Resolve which ABSORB_* pool a `ShieldType` NVP maps to. Centralises
/// the match so `on_apply` and `on_remove` agree on the target pool.
fn shield_pool_id(damage_type: i8) -> i32 {
    match damage_type {
        DT_PHYSICAL => ABSORB_PHYSICAL,
        DT_ENERGY => ABSORB_ENERGY,
        DT_HAZMAT => ABSORB_HAZMAT,
        DT_PSIONIC => ABSORB_PSIONIC,
        DT_UNTYPED => ABSORB_UNTYPED,
        _ => ABSORB_UNTYPED,
    }
}

impl EffectScript for AbsorbShield {
    fn on_apply(&self, ctx: &mut EffectContext) {
        let amount = ctx.effect.param_i32("ShieldAmount");
        if amount <= 0 {
            return;
        }
        let damage_type = ctx.effect.param_i32("ShieldType") as i8;
        let pool_id = shield_pool_id(damage_type);
        let Some(target) = ctx.space_mgr.get_entity_mut(ctx.target_id) else {
            return;
        };
        if let Some(pool) = target.stats.get_mut(pool_id) {
            let new_cur = (pool.cur + amount).min(pool.max);
            pool.update(pool.min, new_cur, pool.max);
            tracing::info!(
                target: "abilities",
                event = "shield_granted",
                source_id = ctx.source_id,
                target_id = ctx.target_id,
                effect_id = ctx.effect.effect_id,
                damage_type,
                amount,
                pool_after = new_cur,
                "AbsorbShield applied"
            );
        }
    }

    /// Drain any residual shield capacity granted by this effect when
    /// the active-effect instance expires. Uses `ShieldAmount` as the
    /// upper bound — we don't over-subtract if damage already chewed
    /// through most of the pool. The drain is capped at the current
    /// pool value via `stat.change()` clamping (min == 0 keeps it
    /// non-negative).
    fn on_remove(&self, ctx: &mut EffectContext) {
        let amount = ctx.effect.param_i32("ShieldAmount");
        if amount <= 0 {
            return;
        }
        let pool_id = shield_pool_id(ctx.effect.param_i32("ShieldType") as i8);
        let Some(target) = ctx.space_mgr.get_entity_mut(ctx.target_id) else {
            return;
        };
        if let Some(pool) = target.stats.get_mut(pool_id) {
            let to_drain = amount.min(pool.cur);
            if to_drain > 0 {
                pool.change(-to_drain);
            }
            tracing::info!(
                target: "abilities",
                event = "shield_expired",
                target_id = ctx.target_id,
                effect_id = ctx.effect.effect_id,
                drained = to_drain,
                pool_after = pool.cur,
                "AbsorbShield expired — residual capacity drained"
            );
        }
    }
}

// ── Stun ─────────────────────────────────────────────────────────────────

/// Locks the target's movement + actions for the effect's duration.
///
/// Sets `BSF_MOVEMENT_LOCK` on apply, clears it on `on_remove` when
/// the active-effect instance expires (Phase I). Pair with a pulsing
/// registration (`pulse_count` × `pulse_duration` = lockdown seconds).
///
/// No NVPs — duration comes from the owning effect's pulse_count ×
/// pulse_duration.
pub struct Stun;

impl EffectScript for Stun {
    fn on_apply(&self, ctx: &mut EffectContext) {
        let Some(target) = ctx.space_mgr.get_entity_mut(ctx.target_id) else {
            return;
        };
        let was_set = target.set_state_flag(BSF_MOVEMENT_LOCK);
        tracing::info!(
            target: "abilities",
            event = "stun_applied",
            source_id = ctx.source_id,
            target_id = ctx.target_id,
            effect_id = ctx.effect.effect_id,
            was_already_set = !was_set,
            "Stun applied — BSF_MOVEMENT_LOCK set"
        );
    }

    /// Clear `BSF_MOVEMENT_LOCK` when the stun instance expires.
    ///
    /// Multi-source stuns stack correctly because
    /// `set_state_flag` / `unset_state_flag` are refcounted via
    /// `state_flag_counts`: two stuns from different invokers bump
    /// the counter to 2, the first expiry decrements to 1 with the
    /// bit STILL set, the second clears it. See
    /// `cell_entity/state_flags.rs` for the counter implementation.
    fn on_remove(&self, ctx: &mut EffectContext) {
        let Some(target) = ctx.space_mgr.get_entity_mut(ctx.target_id) else {
            return;
        };
        let was_set = target.unset_state_flag(BSF_MOVEMENT_LOCK);
        tracing::info!(
            target: "abilities",
            event = "stun_expired",
            target_id = ctx.target_id,
            effect_id = ctx.effect.effect_id,
            cleared = was_set,
            "Stun expired — BSF_MOVEMENT_LOCK cleared"
        );
    }
}

// ── Suppression ──────────────────────────────────────────────────────────

/// Reduces the target's HEALTH by a small per-pulse amount (`HealthDamage`
/// NVP) AND surfaces the suppression event for observability. This is
/// the closest mechanical match to the original game's "suppression":
/// suppress effects do a small chip-damage tick over their duration to
/// discourage the target from staying in the line of fire.
///
/// NVPs:
///   - `HealthDamage` (i32, optional, default 5) — per-pulse chip
///
/// Full movement-speed reduction (the other half of suppression) waits
/// for a `MOVE_SPEED_MOD` stat the cell-entity layer doesn't expose yet.
/// Flagged for a Phase H follow-up — the script is in place so DB
/// content can opt in via `script_name = "Suppression"` without a
/// migration.
pub struct Suppression;

impl EffectScript for Suppression {
    fn on_apply(&self, ctx: &mut EffectContext) {
        let chip = ctx.effect.param_i32("HealthDamage").max(5);
        let Some(target) = ctx.space_mgr.get_entity_mut(ctx.target_id) else {
            return;
        };
        if let Some(stat) = target.stats.get_mut(HEALTH) {
            let cur = stat.cur;
            let new_cur = (cur - chip).max(0);
            stat.update(stat.min, new_cur, stat.max);
        }
        tracing::info!(
            target: "abilities",
            event = "suppression_pulse",
            source_id = ctx.source_id,
            target_id = ctx.target_id,
            effect_id = ctx.effect.effect_id,
            chip_damage = chip,
            "Suppression pulse"
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
            ..Default::default()
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
    fn absorb_shield_adds_to_matching_pool() {
        let mut mgr = make_mgr_with_target();
        let mut params = HashMap::new();
        params.insert("ShieldAmount".to_string(), "200".to_string());
        params.insert("ShieldType".to_string(), DT_PHYSICAL.to_string());
        let effect = EffectDef {
            effect_id: 555,
            ability_id: 1,
            params,
            ..Default::default()
        };
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        AbsorbShield.on_apply(&mut ctx);
        let pool = ctx
            .space_mgr
            .get_entity(1)
            .unwrap()
            .stats
            .get(ABSORB_PHYSICAL)
            .unwrap()
            .cur;
        assert_eq!(pool, 200, "shield grants 200 to ABSORB_PHYSICAL pool");
    }

    #[test]
    fn absorb_shield_defaults_to_untyped_when_no_shield_type_nvp() {
        let mut mgr = make_mgr_with_target();
        let mut params = HashMap::new();
        params.insert("ShieldAmount".to_string(), "100".to_string());
        let effect = EffectDef {
            effect_id: 556,
            ability_id: 1,
            params,
            ..Default::default()
        };
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        AbsorbShield.on_apply(&mut ctx);
        let untyped = ctx
            .space_mgr
            .get_entity(1)
            .unwrap()
            .stats
            .get(ABSORB_UNTYPED)
            .unwrap()
            .cur;
        assert_eq!(untyped, 100);
    }

    #[test]
    fn stun_multi_source_keeps_lock_until_all_release() {
        // Phase K: two concurrent stuns from different invokers share
        // one `BSF_MOVEMENT_LOCK` bit via the refcounted state-flag
        // helpers. First release must NOT drop the bit while the second
        // is still pulsing.
        let mut mgr = make_mgr_with_target();
        let effect_a = EffectDef {
            effect_id: 700,
            ability_id: 100,
            ..Default::default()
        };
        let effect_b = EffectDef {
            effect_id: 701,
            ability_id: 101,
            ..Default::default()
        };

        // Apply both stuns (different invokers) in scoped blocks so the
        // mutable borrows on `mgr` don't overlap between EffectContexts.
        {
            let mut ctx_a = EffectContext {
                source_id: 1,
                target_id: 1,
                effect: &effect_a,
                space_mgr: &mut mgr,
            };
            Stun.on_apply(&mut ctx_a);
        }
        {
            let mut ctx_b = EffectContext {
                source_id: 99,
                target_id: 1,
                effect: &effect_b,
                space_mgr: &mut mgr,
            };
            Stun.on_apply(&mut ctx_b);
        }
        assert!(
            mgr.get_entity(1).unwrap().has_state_flag(BSF_MOVEMENT_LOCK),
            "both stuns set the lock"
        );

        // First expiry — bit must STAY set (refcount drops 2 → 1)
        {
            let mut ctx_a = EffectContext {
                source_id: 1,
                target_id: 1,
                effect: &effect_a,
                space_mgr: &mut mgr,
            };
            Stun.on_remove(&mut ctx_a);
        }
        assert!(
            mgr.get_entity(1).unwrap().has_state_flag(BSF_MOVEMENT_LOCK),
            "lock must stay while second stun still active"
        );

        // Second expiry — now bit clears (refcount drops 1 → 0)
        {
            let mut ctx_b = EffectContext {
                source_id: 99,
                target_id: 1,
                effect: &effect_b,
                space_mgr: &mut mgr,
            };
            Stun.on_remove(&mut ctx_b);
        }
        assert!(
            !mgr.get_entity(1).unwrap().has_state_flag(BSF_MOVEMENT_LOCK),
            "lock clears when last reason expires"
        );
    }

    #[test]
    fn stun_on_remove_clears_movement_lock_state_flag() {
        // Phase I: stun apply → movement lock set; stun remove → cleared.
        let mut mgr = make_mgr_with_target();
        let effect = EffectDef {
            effect_id: 601,
            ability_id: 1,
            ..Default::default()
        };
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        Stun.on_apply(&mut ctx);
        assert!(
            ctx.space_mgr
                .get_entity(1)
                .unwrap()
                .has_state_flag(BSF_MOVEMENT_LOCK),
            "Stun apply must set lock"
        );
        Stun.on_remove(&mut ctx);
        assert!(
            !ctx.space_mgr
                .get_entity(1)
                .unwrap()
                .has_state_flag(BSF_MOVEMENT_LOCK),
            "Stun on_remove must clear lock"
        );
    }

    #[test]
    fn absorb_shield_on_remove_drains_residual_pool() {
        // Phase I: shield with 200 amount drains 200 from pool on remove,
        // capped by current pool value (no overdrain).
        let mut mgr = make_mgr_with_target();
        let mut params = HashMap::new();
        params.insert("ShieldAmount".to_string(), "200".to_string());
        params.insert("ShieldType".to_string(), DT_PHYSICAL.to_string());
        let effect = EffectDef {
            effect_id: 557,
            ability_id: 1,
            params,
            ..Default::default()
        };
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        AbsorbShield.on_apply(&mut ctx);
        // Pool grew by 200
        let before = ctx
            .space_mgr
            .get_entity(1)
            .unwrap()
            .stats
            .get(ABSORB_PHYSICAL)
            .unwrap()
            .cur;
        assert_eq!(before, 200);
        // Pretend damage drained 50 from the pool before expiry
        if let Some(t) = ctx.space_mgr.get_entity_mut(1) {
            if let Some(stat) = t.stats.get_mut(ABSORB_PHYSICAL) {
                stat.change(-50);
            }
        }
        AbsorbShield.on_remove(&mut ctx);
        // Pool drains by min(200, 150) = 150 → back to 0
        let after = ctx
            .space_mgr
            .get_entity(1)
            .unwrap()
            .stats
            .get(ABSORB_PHYSICAL)
            .unwrap()
            .cur;
        assert_eq!(after, 0, "on_remove drains residual without going negative");
    }

    #[test]
    fn stun_sets_movement_lock_state_flag() {
        let mut mgr = make_mgr_with_target();
        let effect = EffectDef {
            effect_id: 600,
            ability_id: 1,
            ..Default::default()
        };
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        Stun.on_apply(&mut ctx);
        let has_lock = ctx
            .space_mgr
            .get_entity(1)
            .unwrap()
            .has_state_flag(BSF_MOVEMENT_LOCK);
        assert!(has_lock, "Stun must set BSF_MOVEMENT_LOCK");
    }

    #[test]
    fn suppression_chips_health_by_nvp_amount() {
        let mut mgr = make_mgr_with_target();
        // Player starts at 50/100. Suppression with HealthDamage=8.
        let mut params = HashMap::new();
        params.insert("HealthDamage".to_string(), "8".to_string());
        let effect = EffectDef {
            effect_id: 700,
            ability_id: 1,
            params,
            ..Default::default()
        };
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        Suppression.on_apply(&mut ctx);
        let hp = ctx
            .space_mgr
            .get_entity(1)
            .unwrap()
            .stats
            .get(HEALTH)
            .unwrap()
            .cur;
        assert_eq!(hp, 42, "50 - 8 chip = 42");
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
