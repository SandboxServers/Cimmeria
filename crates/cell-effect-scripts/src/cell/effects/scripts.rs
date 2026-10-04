//! Effect script implementations.
//!
//! v1 — single-shot stat scripts:
//! - [`HealHealth`] / [`HealFocus`] — flat `HealAmount` or `HealPercentage`
//!   × max (in `heal.rs`, re-exported here)
//! - [`MeleeDamage`] — `HealthDamage` raw damage
//!
//! v2 — buff/debuff scripts:
//! - [`AbsorbShield`] — an absorb shield on the timed effect ledger (in
//!   `shield/`, re-exported here)
//! - `Stun` moved to `crowd_control.rs` (re-exported here), beside
//!   `Knockdown` and `Interrupt`: a timed-ledger entry holding
//!   `BSF_MOVEMENT_LOCK` (ability mechanics AB-09).
//! - [`Suppression`] — per-pulse HEALTH chip via the `HealthDamage`
//!   NVP. Full movement-speed reduction (the original game's other
//!   half of "suppression") waits for a `MOVE_SPEED_MOD` stat the
//!   cell-entity layer doesn't expose yet.
//!
//! ## Adding a new script
//!
//! 1. Add a zero-sized struct here (or in its family's module) with an
//!    `impl EffectScript`.
//! 2. Add its row to [`super::registry::EFFECT_SCRIPTS`].
//! 3. Add a unit test in this file covering the happy path + edge cases
//!    (missing target, zero/negative NVP, missing stat).
//! 4. Seed an effect row with `script_name = "YourScriptName"` if you
//!    want existing content to dispatch through it.

use super::script_rows::script_skipped;
use super::{EffectContext, EffectScript};

// The pool heals moved to `heal.rs` (this file is over the cap); re-exported
// so `scripts::HealHealth` keeps resolving for the registry and pet scripts.
pub use super::heal::{HealFocus, HealHealth};
// Same for the stun, now in `crowd_control.rs`.
pub use super::crowd_control::Stun;
pub use super::shield::AbsorbShield;
use cimmeria_entity::stats::{FOCUS, HEALTH};

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
/// In `damage_apply` this is a damage script (AB-06, D-AB07): an effect
/// bound to it deals its `HealthDamage` through this script only, and the
/// legacy NVP pipeline skips the effect. A missed QR roll runs no script.
pub struct MeleeDamage;

impl EffectScript for MeleeDamage {
    fn on_apply(&self, ctx: &mut EffectContext) {
        let damage = ctx.effect.param_i32("HealthDamage");
        if damage <= 0 {
            script_skipped(ctx, "MeleeDamage", "no_damage_nvp");
            return;
        }
        let Some(target) = ctx.space_mgr.get_entity(ctx.target_id) else {
            script_skipped(ctx, "MeleeDamage", "target_gone");
            return;
        };
        let Some(cur) = target.stats.get(HEALTH).map(|s| s.cur) else {
            script_skipped(ctx, "MeleeDamage", "no_health_stat");
            return;
        };
        let cast_id = ctx.space_mgr.current_cast_id();
        let Some(target) = ctx.space_mgr.get_entity_mut(ctx.target_id) else {
            return;
        };
        let new_cur = (cur - damage).max(0);
        if let Some(stat) = target.stats.get_mut(HEALTH) {
            stat.update(stat.min, new_cur, stat.max);
        }
        tracing::debug!(
            target: "abilities",
            event = "melee_damage",
            stage = "apply",
            cast_id,
            source_id = ctx.source_id,
            target_id = ctx.target_id,
            effect_id = ctx.effect.effect_id,
            damage,
            new_cur,
            "MeleeDamage applied"
        );
    }
}

// ── MeleePhysicalDamage ──────────────────────────────────────────────────

/// Focus-gated physical damage for melee abilities. Direct counterpart
/// to [`RangedPhysicalDamage`] but for short-range strikes — same
/// shield-first math, no script-side range gating (the ability's
/// `max_range` field gates that one layer up in `damage_apply`).
///
/// NVPs:
///   - `FocusDamage` (i32) — amount to subtract from target FOCUS pool
///   - `HealthDamage` (i32) — base HEALTH damage added to spillover
///
/// Modeled on the legacy effect-desc convention `"-100F -10H"` used
/// by the original Strike ability (594, effect 656). The legacy
/// authoring stored those numbers in `effect_desc` as a free-text
/// hint and relied on per-script logic to do the actual application.
/// We surface them as proper NVPs and reuse the existing two-step
/// integer truncation from `RangedPhysicalDamage` so combat tuning
/// is parity-correct across both archetypes.
///
/// Behavior, mirroring the `RangedPhysicalDamage` flow:
///
/// 1. Subtract `FocusDamage` from the target's FOCUS pool. The pool
///    clamps at 0; any unused damage is the "overflow."
/// 2. If FOCUS absorbed everything (no overflow) → done. No HEALTH
///    damage applied — shields held the melee hit.
/// 3. Otherwise, compute spillover via the two-step integer formula
///    `(overflow * 100 / FocusDamage) * FocusDamage / 300`, add the
///    base `HealthDamage`, apply to HEALTH.
///
/// **Why not extend `MeleeDamage` to take a `FocusDamage` NVP?**
/// `MeleeDamage` is the simple "raw HP damage" script — flat damage
/// is the entire contract today. A drive-by addition would change
/// its semantic without renaming, and the same NVP-key collision
/// (`FocusDamage`) would silently start draining FOCUS for any
/// existing effect that happened to inherit the NVP. Splitting into
/// a named-as-such script keeps the two intentions clearly separable.
///
/// An effect bound to this script deals its damage here only:
/// `damage_apply` skips the legacy NVP pipeline for it (AB-06, D-AB07;
/// before AB-06 both ran, so Strike hit twice, audit B-22). An effect
/// with the same NVPs and no script still takes the NVP pipeline, which
/// damages both pools independently, so the target loses HEALTH even at
/// full Focus.
///
/// **Edge case — `FocusDamage = 0` with `HealthDamage > 0`:** the
/// script returns early via the `focus_overflow == 0` gate without
/// applying the `HealthDamage`. This is intentional and mirrors
/// `RangedPhysicalDamage`'s shields-first contract: with no Focus
/// component there is no shield to pierce, so the spillover-only
/// HEALTH bleed never fires. If an effect genuinely wants flat raw
/// melee damage with no Focus interaction, the right script is
/// `MeleeDamage` (which reads `HealthDamage` alone), not this one.
///
/// Reference: `crates/cell-world/src/cell/effects/scripts.rs::RangedPhysicalDamage`
/// (parent shape); `db/resources/Effects/Seed/effects.sql` row for
/// effect 656.
pub struct MeleePhysicalDamage;

impl EffectScript for MeleePhysicalDamage {
    fn on_apply(&self, ctx: &mut EffectContext) {
        let focus_damage = ctx.effect.param_i32("FocusDamage").max(0);
        let health_damage = ctx.effect.param_i32("HealthDamage").max(0);
        if focus_damage == 0 && health_damage == 0 {
            script_skipped(ctx, "MeleePhysicalDamage", "no_damage_nvp");
            return;
        }

        let cast_id = ctx.space_mgr.current_cast_id();
        if ctx.space_mgr.get_entity(ctx.target_id).is_none() {
            script_skipped(ctx, "MeleePhysicalDamage", "target_gone");
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
            // suppresses Health damage too — matches the parent
            // `RangedPhysicalDamage` conditional branch off the QR result.
            0
        };

        // Legacy gate: if Focus absorbed everything (overflow == 0),
        // NO Health damage is applied at all. The shield held the strike.
        if focus_overflow == 0 {
            tracing::debug!(
                target: "abilities",
                event = "melee_physical_damage",
                stage = "apply",
                cast_id,
                source_id = ctx.source_id,
                target_id = ctx.target_id,
                effect_id = ctx.effect.effect_id,
                focus_damage,
                focus_overflow,
                health_damage_applied = 0,
                "MeleePhysicalDamage: Focus absorbed all damage, no HEALTH bleed",
            );
            return;
        }

        // Two-step integer truncation — see `RangedPhysicalDamage` for
        // the small-overflow divergence rationale. DO NOT collapse to
        // `focus_overflow / 3`.
        let remaining_pct = focus_overflow.saturating_mul(100) / focus_damage;
        let spillover = remaining_pct.saturating_mul(focus_damage) / 300;
        let final_health_damage = spillover + health_damage;
        if final_health_damage <= 0 {
            script_skipped(ctx, "MeleePhysicalDamage", "no_health_bleed");
            return;
        }
        if let Some(stat) = target.stats.get_mut(HEALTH) {
            let new_cur = (stat.cur - final_health_damage).max(0);
            stat.update(stat.min, new_cur, stat.max);
        }
        tracing::debug!(
            target: "abilities",
            event = "melee_physical_damage",
            stage = "apply",
            cast_id,
            source_id = ctx.source_id,
            target_id = ctx.target_id,
            effect_id = ctx.effect.effect_id,
            focus_damage,
            focus_overflow,
            remaining_pct,
            spillover,
            health_damage_applied = final_health_damage,
            base_health_damage = health_damage,
            "MeleePhysicalDamage: Focus pierced, applied HEALTH bleed",
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
            script_skipped(ctx, "Suppression", "no_damage_nvp");
            return;
        }
        let cast_id = ctx.space_mgr.current_cast_id();
        if ctx.space_mgr.get_entity(ctx.target_id).is_none() {
            script_skipped(ctx, "Suppression", "target_gone");
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
        tracing::debug!(
            target: "abilities",
            event = "suppression_pulse",
            stage = "apply",
            cast_id,
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
/// An effect bound to this script deals its damage here only:
/// `damage_apply` skips the legacy NVP pipeline for it and scales the
/// two NVPs by the hit's cover and ammo scale first (AB-06, D-AB07;
/// before AB-06 both paths ran, so Pistol Shot hit twice, audit B-22).
/// A missed QR roll runs no script.
///
/// Reference: `deprecated/python/cell/effects/RangedPhysicalDamage.py`
/// and `deprecated/data-scripts/scripts/effects/RangedPhysicalDamage.script`.
pub struct RangedPhysicalDamage;

impl EffectScript for RangedPhysicalDamage {
    fn on_apply(&self, ctx: &mut EffectContext) {
        let focus_damage = ctx.effect.param_i32("FocusDamage").max(0);
        let health_damage = ctx.effect.param_i32("HealthDamage").max(0);
        if focus_damage == 0 && health_damage == 0 {
            script_skipped(ctx, "RangedPhysicalDamage", "no_damage_nvp");
            return;
        }

        let cast_id = ctx.space_mgr.current_cast_id();
        if ctx.space_mgr.get_entity(ctx.target_id).is_none() {
            script_skipped(ctx, "RangedPhysicalDamage", "target_gone");
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
                stage = "apply",
                cast_id,
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
            script_skipped(ctx, "RangedPhysicalDamage", "no_health_bleed");
            return;
        }
        if let Some(stat) = target.stats.get_mut(HEALTH) {
            let new_cur = (stat.cur - final_health_damage).max(0);
            stat.update(stat.min, new_cur, stat.max);
        }
        tracing::debug!(
            target: "abilities",
            event = "ranged_physical_damage",
            stage = "apply",
            cast_id,
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
            script_skipped(ctx, "RangedEnergyDamage", "no_damage_nvp");
            return;
        }

        let cast_id = ctx.space_mgr.current_cast_id();
        if ctx.space_mgr.get_entity(ctx.target_id).is_none() {
            script_skipped(ctx, "RangedEnergyDamage", "target_gone");
            return;
        }
        let Some(target) = ctx.space_mgr.get_entity_mut(ctx.target_id) else {
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

        tracing::debug!(
            target: "abilities",
            event = "ranged_energy_damage",
            stage = "apply",
            cast_id,
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
#[path = "scripts_tests.rs"]
mod tests;
