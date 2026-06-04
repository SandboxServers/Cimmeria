//! Effect script implementations.
//!
//! v1 — single-shot stat scripts:
//! - [`HealHealth`] / [`HealFocus`] — heal by `HealPercentage` × max
//! - [`MeleeDamage`] — `HealthDamage` raw damage
//!
//! v2 — buff/debuff scripts:
//! - [`AbsorbShield`] — adds `ShieldAmount` to the matching ABSORB_*
//!   pool so subsequent damage of that type is consumed from the
//!   shield before HEALTH. Drains residual capacity on `on_remove`.
//! - [`Stun`] — sets `BSF_MOVEMENT_LOCK` on the target; cleared on
//!   `on_remove` when the active-effect instance expires.
//! - [`Suppression`] — per-pulse HEALTH chip via the `HealthDamage`
//!   NVP. Full movement-speed reduction (the original game's other
//!   half of "suppression") waits for a `MOVE_SPEED_MOD` stat the
//!   cell-entity layer doesn't expose yet.
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
        // Default chip = 5 when the effect doesn't specify a `HealthDamage`
        // NVP. `param_i32` already returns 0 for missing keys, so we test
        // for presence and supply the default explicitly — `.max(5)` would
        // floor every Suppression hit at 5 even when content authored
        // `HealthDamage = 1`, which silently breaks chip-damage tuning.
        let chip = if ctx.effect.params.contains_key("HealthDamage") {
            ctx.effect.param_i32("HealthDamage").max(0)
        } else {
            5
        };
        if chip == 0 {
            return;
        }
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

// ── RangedPhysicalDamage ─────────────────────────────────────────────────

/// Focus-gated physical damage. Models "shields absorb the bullet first."
///
/// NVPs:
///   - `FocusDamage` (i32) — amount to subtract from target FOCUS pool
///   - `HealthDamage` (i32) — base HEALTH damage added to spillover
///
/// Behavior, mirroring `deprecated/python/cell/effects/RangedPhysicalDamage.py`:
///
/// 1. Subtract `FocusDamage` from the target's FOCUS pool. The pool
///    clamps at 0 — any unused damage is the "overflow."
/// 2. If FOCUS absorbed *everything* (no overflow) → done. No HEALTH
///    damage. This is the "shields held" case.
/// 3. Otherwise, compute spillover via the legacy two-step integer
///    formula `(overflow * 100 / FocusDamage) * FocusDamage / 300`,
///    add `HealthDamage`, apply to HEALTH.
///
/// **Why the two-step integer formula matters** — algebraically the
/// `FocusDamage` factor cancels to `overflow / 3`, but the legacy
/// truncates after each integer step. With `FocusDamage = 80` and
/// `overflow = 3`: legacy computes `3 * 100 / 80 = 3` (truncated from
/// 3.75) then `3 * 80 / 300 = 0` (truncated from 0.8) — zero
/// spillover. The algebraic shortcut `overflow / 3` gives 1 here,
/// over-damaging on small overflows. We match the legacy truncation
/// step-for-step so combat tuning is parity-correct.
///
/// Without this script, the legacy NVP fallback applies both
/// `FocusDamage` and `HealthDamage` independently every shot, so the
/// player takes HEALTH damage even at full Focus.
///
/// Reference: `deprecated/python/cell/effects/RangedPhysicalDamage.py`
/// and `deprecated/data-scripts/scripts/effects/RangedPhysicalDamage.script`.
pub struct RangedPhysicalDamage;

impl EffectScript for RangedPhysicalDamage {
    fn on_apply(&self, ctx: &mut EffectContext) {
        let focus_damage = ctx.effect.param_i32("FocusDamage").max(0);
        let health_damage = ctx.effect.param_i32("HealthDamage").max(0);
        if focus_damage == 0 && health_damage == 0 {
            return;
        }

        let Some(target) = ctx.space_mgr.get_entity_mut(ctx.target_id) else {
            return;
        };

        // Apply Focus damage; capture how much overflowed the pool.
        let focus_overflow = if focus_damage > 0 {
            let cur = target.stats.get(FOCUS).map(|s| s.cur).unwrap_or(0);
            let applied = focus_damage.min(cur.max(0));
            let overflow = (focus_damage - applied).max(0);
            if let Some(stat) = target.stats.get_mut(FOCUS) {
                let new_cur = (cur - applied).max(0);
                stat.update(stat.min, new_cur, stat.max);
            }
            overflow
        } else {
            // If there's no Focus damage at all, the spillover gate
            // (`overflow > 0`) suppresses Health damage too — matches
            // the legacy script's conditional branch off the QR result.
            0
        };

        // Legacy gate: if Focus absorbed everything (overflow == 0),
        // NO Health damage is applied at all. The shield held.
        if focus_overflow == 0 {
            tracing::debug!(
                target: "abilities",
                event = "ranged_physical_damage",
                source_id = ctx.source_id,
                target_id = ctx.target_id,
                effect_id = ctx.effect.effect_id,
                focus_damage,
                focus_overflow,
                health_damage_applied = 0,
                "RangedPhysicalDamage: Focus absorbed all damage, no HEALTH bleed",
            );
            return;
        }

        // Two-step integer truncation matches the legacy Atrea script
        // graph (Node 6 → 9 → 10 → 12). DO NOT collapse to
        // `focus_overflow / 3` — see fn docs for the small-overflow
        // divergence.
        let remaining_pct = focus_overflow.saturating_mul(100) / focus_damage;
        let spillover = remaining_pct.saturating_mul(focus_damage) / 300;
        let final_health_damage = spillover + health_damage;
        if final_health_damage <= 0 {
            return;
        }
        if let Some(stat) = target.stats.get_mut(HEALTH) {
            let new_cur = (stat.cur - final_health_damage).max(0);
            stat.update(stat.min, new_cur, stat.max);
        }
        tracing::info!(
            target: "abilities",
            event = "ranged_physical_damage",
            source_id = ctx.source_id,
            target_id = ctx.target_id,
            effect_id = ctx.effect.effect_id,
            focus_damage,
            focus_overflow,
            remaining_pct,
            spillover,
            health_damage_applied = final_health_damage,
            base_health_damage = health_damage,
            "RangedPhysicalDamage: Focus pierced, applied HEALTH bleed",
        );
    }
}

// ── RangedEnergyDamage ───────────────────────────────────────────────────

/// Parallel HEALTH + FOCUS damage with no shield-first gating. The
/// energy-weapon counterpart to [`RangedPhysicalDamage`] — both pools
/// are hit on every shot regardless of Focus state.
///
/// NVPs:
///   - `HealthDamage` (i32)
///   - `FocusDamage`  (i32)
///
/// Reference: `deprecated/python/cell/effects/RangedEnergyDamage.py` —
/// `effect.qrCombatDamage(HEALTH, ...)` and `effect.qrCombatDamage(FOCUS, ...)`
/// fired back-to-back on `onPulseBegin`, no comparison gate.
pub struct RangedEnergyDamage;

impl EffectScript for RangedEnergyDamage {
    fn on_apply(&self, ctx: &mut EffectContext) {
        let focus_damage = ctx.effect.param_i32("FocusDamage").max(0);
        let health_damage = ctx.effect.param_i32("HealthDamage").max(0);
        if focus_damage == 0 && health_damage == 0 {
            return;
        }

        let Some(target) = ctx.space_mgr.get_entity_mut(ctx.target_id) else {
            tracing::debug!(
                target: "abilities",
                event = "ranged_energy_damage",
                source_id = ctx.source_id,
                target_id = ctx.target_id,
                effect_id = ctx.effect.effect_id,
                "RangedEnergyDamage: target missing — skipped",
            );
            return;
        };

        if focus_damage > 0 {
            if let Some(stat) = target.stats.get_mut(FOCUS) {
                let new_cur = (stat.cur - focus_damage).max(0);
                stat.update(stat.min, new_cur, stat.max);
            }
        }
        if health_damage > 0 {
            if let Some(stat) = target.stats.get_mut(HEALTH) {
                let new_cur = (stat.cur - health_damage).max(0);
                stat.update(stat.min, new_cur, stat.max);
            }
        }

        tracing::info!(
            target: "abilities",
            event = "ranged_energy_damage",
            source_id = ctx.source_id,
            target_id = ctx.target_id,
            effect_id = ctx.effect.effect_id,
            focus_damage,
            health_damage,
            "RangedEnergyDamage applied",
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

    // ── RangedPhysicalDamage ──────────────────────────────────────────

    fn effect_with_two_nvps(name1: &str, val1: &str, name2: &str, val2: &str) -> EffectDef {
        let mut params = HashMap::new();
        params.insert(name1.to_string(), val1.to_string());
        params.insert(name2.to_string(), val2.to_string());
        EffectDef {
            effect_id: 641,
            ability_id: 579,
            params,
            ..Default::default()
        }
    }

    /// **Shield absorbs everything → no health damage.** Mirror of
    /// `if remaining_dmg_percent > 0` in the legacy Python: when Focus
    /// fully absorbs the requested damage, the script returns without
    /// touching HEALTH. This is the load-bearing difference vs. the
    /// legacy NVP fallback (which applies both pools independently).
    /// Reverting the gate (always applying HealthDamage) would fail
    /// this test.
    #[test]
    fn ranged_physical_full_focus_absorbs_no_health_damage() {
        let mut mgr = make_mgr_with_target();
        // Focus 200/1000 in fixture; FocusDamage 100 fits entirely.
        let effect = effect_with_two_nvps("FocusDamage", "100", "HealthDamage", "10");
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        RangedPhysicalDamage.on_apply(&mut ctx);
        let e = ctx.space_mgr.get_entity(1).unwrap();
        assert_eq!(
            e.stats.get(FOCUS).unwrap().cur,
            100,
            "100 focus damage out of 200 must drain to 100"
        );
        assert_eq!(
            e.stats.get(HEALTH).unwrap().cur,
            50,
            "shield held → no HEALTH damage, even though HealthDamage NVP = 10"
        );
    }

    /// **Partial absorb → spillover lands as health damage.** Focus
    /// 30/1000, FocusDamage 100 → 30 applied to focus, 70 overflow,
    /// spillover = 70/3 = 23, plus HealthDamage 10 = 33 health damage.
    #[test]
    fn ranged_physical_partial_absorb_spills_to_health() {
        let mut mgr = make_mgr_with_target();
        if let Some(e) = mgr.get_entity_mut(1) {
            if let Some(s) = e.stats.get_mut(FOCUS) {
                s.update(0, 30, 1000);
            }
        }
        let effect = effect_with_two_nvps("FocusDamage", "100", "HealthDamage", "10");
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        RangedPhysicalDamage.on_apply(&mut ctx);
        let e = ctx.space_mgr.get_entity(1).unwrap();
        assert_eq!(
            e.stats.get(FOCUS).unwrap().cur,
            0,
            "focus drained to 0 (30 was less than 100)"
        );
        // Overflow = 70. Spillover = 70/3 = 23 (integer division).
        // Final health damage = 23 + 10 = 33. HP was 50 → 17.
        assert_eq!(
            e.stats.get(HEALTH).unwrap().cur,
            17,
            "HP 50 - (spillover 23 + HealthDamage 10) = 17"
        );
    }

    /// **No focus at all → full overflow.** With FOCUS = 0, the entire
    /// FocusDamage is overflow; spillover = 100/3 = 33; + HealthDmg 10
    /// = 43 HP loss. HP 50 → 7.
    #[test]
    fn ranged_physical_no_focus_takes_full_overflow_spillover() {
        let mut mgr = make_mgr_with_target();
        if let Some(e) = mgr.get_entity_mut(1) {
            if let Some(s) = e.stats.get_mut(FOCUS) {
                s.update(0, 0, 1000);
            }
        }
        let effect = effect_with_two_nvps("FocusDamage", "100", "HealthDamage", "10");
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        RangedPhysicalDamage.on_apply(&mut ctx);
        let e = ctx.space_mgr.get_entity(1).unwrap();
        assert_eq!(e.stats.get(FOCUS).unwrap().cur, 0);
        assert_eq!(
            e.stats.get(HEALTH).unwrap().cur,
            7,
            "HP 50 - (spillover 33 + HealthDamage 10) = 7"
        );
    }

    /// **FocusDamage = 0 → no Focus mutation AND no Health damage.**
    /// The spillover gate trips on `focus_overflow == 0`. With no
    /// Focus damage configured, overflow is also 0, so the script
    /// returns before touching HEALTH. This pins that the script is
    /// genuinely Focus-driven — HealthDamage alone shouldn't fire.
    #[test]
    fn ranged_physical_zero_focus_damage_skips_health_too() {
        let mut mgr = make_mgr_with_target();
        let effect = effect_with_two_nvps("FocusDamage", "0", "HealthDamage", "10");
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        RangedPhysicalDamage.on_apply(&mut ctx);
        let e = ctx.space_mgr.get_entity(1).unwrap();
        assert_eq!(e.stats.get(FOCUS).unwrap().cur, 200, "no focus mutation");
        assert_eq!(
            e.stats.get(HEALTH).unwrap().cur,
            50,
            "no health damage when Focus damage is zero"
        );
    }

    /// **Missing target is a graceful no-op.**
    #[test]
    fn ranged_physical_missing_target_is_noop() {
        let mut mgr = make_mgr_with_target();
        let effect = effect_with_two_nvps("FocusDamage", "100", "HealthDamage", "10");
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 999, // doesn't exist
            effect: &effect,
            space_mgr: &mut mgr,
        };
        RangedPhysicalDamage.on_apply(&mut ctx);
        let e = ctx.space_mgr.get_entity(1).unwrap();
        assert_eq!(e.stats.get(FOCUS).unwrap().cur, 200);
        assert_eq!(e.stats.get(HEALTH).unwrap().cur, 50);
    }

    // ── RangedEnergyDamage ────────────────────────────────────────────

    /// **Energy damage hits both pools simultaneously — no gating.**
    /// The structural difference vs. RangedPhysicalDamage: even if the
    /// target's Focus could absorb the requested damage, Health still
    /// takes the HealthDamage NVP. This is what makes Energy weapons
    /// the "ignore shields" counterpart to Physical.
    #[test]
    fn ranged_energy_applies_both_pools_in_parallel() {
        let mut mgr = make_mgr_with_target();
        // Focus 200/1000, HP 50/100 in fixture.
        let effect = effect_with_two_nvps("FocusDamage", "30", "HealthDamage", "15");
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        RangedEnergyDamage.on_apply(&mut ctx);
        let e = ctx.space_mgr.get_entity(1).unwrap();
        assert_eq!(
            e.stats.get(FOCUS).unwrap().cur,
            170,
            "200 - 30 = 170 (no gating — applied independently)"
        );
        assert_eq!(
            e.stats.get(HEALTH).unwrap().cur,
            35,
            "50 - 15 = 35 (applied even though Focus could have absorbed)"
        );
    }

    /// Zero-NVP edge: nothing applied either way.
    #[test]
    fn ranged_energy_zero_nvps_is_noop() {
        let mut mgr = make_mgr_with_target();
        let effect = effect_with_two_nvps("FocusDamage", "0", "HealthDamage", "0");
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        RangedEnergyDamage.on_apply(&mut ctx);
        let e = ctx.space_mgr.get_entity(1).unwrap();
        assert_eq!(e.stats.get(FOCUS).unwrap().cur, 200);
        assert_eq!(e.stats.get(HEALTH).unwrap().cur, 50);
    }

    /// Pool clamps at 0 — Focus damage exceeding cur drains to 0, not
    /// below. (For Energy there's no spillover so the excess just
    /// disappears.)
    #[test]
    fn ranged_energy_drain_clamps_at_zero() {
        let mut mgr = make_mgr_with_target();
        if let Some(e) = mgr.get_entity_mut(1) {
            if let Some(s) = e.stats.get_mut(FOCUS) {
                s.update(0, 20, 1000);
            }
        }
        let effect = effect_with_two_nvps("FocusDamage", "100", "HealthDamage", "5");
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        RangedEnergyDamage.on_apply(&mut ctx);
        let e = ctx.space_mgr.get_entity(1).unwrap();
        assert_eq!(e.stats.get(FOCUS).unwrap().cur, 0, "focus clamps at 0");
        assert_eq!(e.stats.get(HEALTH).unwrap().cur, 45, "50 - 5 = 45");
    }

    /// Pins the legacy two-step truncation: with FocusDamage=80,
    /// overflow=3 → `remaining_pct = 3*100/80 = 3` (truncated from 3.75)
    /// → `spillover = 3*80/300 = 0` (truncated from 0.8). Zero spillover.
    /// A regression to `overflow / 3` would compute `3/3 = 1` and over-
    /// damage small overflows.
    #[test]
    fn ranged_physical_small_overflow_truncates_to_zero_spillover() {
        let mut mgr = make_mgr_with_target();
        if let Some(e) = mgr.get_entity_mut(1) {
            if let Some(s) = e.stats.get_mut(FOCUS) {
                s.update(0, 77, 1000); // 80 dmg → applied 77 → overflow 3
            }
        }
        let effect = effect_with_two_nvps("FocusDamage", "80", "HealthDamage", "10");
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        RangedPhysicalDamage.on_apply(&mut ctx);
        let e = ctx.space_mgr.get_entity(1).unwrap();
        assert_eq!(e.stats.get(FOCUS).unwrap().cur, 0);
        // Spillover = 0, so final health damage is just HealthDamage = 10.
        // HP 50 - 10 = 40. With `overflow / 3` (1 spillover) it would be 39.
        assert_eq!(
            e.stats.get(HEALTH).unwrap().cur,
            40,
            "small overflow (3) truncates to zero spillover; only base \
             HealthDamage applies. Regression to overflow/3 would give 39."
        );
    }

    /// `param_i32` returns whatever the NVP parses to; the script
    /// `.max(0)`s it, so a negative NVP value (content authoring
    /// mistake) is clamped to 0 and treated as zero damage on that
    /// pool — never produces "negative damage" healing.
    #[test]
    fn ranged_physical_negative_nvps_clamp_to_zero() {
        let mut mgr = make_mgr_with_target();
        let effect = effect_with_two_nvps("FocusDamage", "-50", "HealthDamage", "-20");
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        RangedPhysicalDamage.on_apply(&mut ctx);
        let e = ctx.space_mgr.get_entity(1).unwrap();
        assert_eq!(e.stats.get(FOCUS).unwrap().cur, 200, "no focus change");
        assert_eq!(e.stats.get(HEALTH).unwrap().cur, 50, "no health change");
    }

    /// Missing target on the energy path returns silently with a debug
    /// log — companion to `ranged_physical_missing_target_is_noop`.
    #[test]
    fn ranged_energy_missing_target_is_noop() {
        let mut mgr = make_mgr_with_target();
        let effect = effect_with_two_nvps("FocusDamage", "30", "HealthDamage", "15");
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 9999,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        RangedEnergyDamage.on_apply(&mut ctx);
        let e = ctx.space_mgr.get_entity(1).unwrap();
        assert_eq!(e.stats.get(FOCUS).unwrap().cur, 200);
        assert_eq!(e.stats.get(HEALTH).unwrap().cur, 50);
    }
}
