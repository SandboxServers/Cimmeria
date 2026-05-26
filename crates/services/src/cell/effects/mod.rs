//! Effect-script dispatcher (#331 Phase 1).
//!
//! `EffectDef.script_name: Option<String>` has been loaded from the DB for
//! some time but never dispatched — the field was dead. This module wires
//! it up: when an effect with a non-NULL `script_name` fires, the registry
//! looks up the named script and runs it. Scripts decide their own
//! behavior (heal, damage, buff) without each call site having to hand-
//! roll the NVP read + stat mutate + wire packet send.
//!
//! ## Architecture
//!
//! - **[`EffectScript`]** trait — one method (`on_apply`) for the v1
//!   single-shot model. The Python reference (`AbilityManager.py`) calls
//!   `EffectInstance.doAction` once per pulse with the full ctx; we
//!   collapse to `on_apply` for now since the cell has no continuous
//!   pulse-tick infrastructure yet. Channelled / multi-pulse effects
//!   are tracked as a v2 follow-up that adds `on_pulse_begin` /
//!   `on_pulse_end` + a pulse-tick loop.
//!
//! - **[`EffectContext`]** — carries the source/target ids, the effect's
//!   parameters (NVP bag from `effect.params`), and the mutable
//!   [`SpaceManager`] handle so scripts can read stats + mutate the
//!   target.
//!
//! - **[`registry::dispatch`]** — static `HashMap<&'static str, &dyn
//!   EffectScript>` keyed by `script_name`. Unknown names hit the warn
//!   log and no-op (don't crash). Match the script-naming scheme that
//!   the original game's content was authored against — `HealHealth`,
//!   `HealFocus`, `MeleeDamage`, and the rest as they're added.
//!
//! ## v1 scope (this PR)
//!
//! Three scripts are seeded:
//! - [`scripts::HealFocus`] — `HealPercentage` NVP × max focus
//! - [`scripts::HealHealth`] — `HealPercentage` NVP × max health
//! - [`scripts::MeleeDamage`] — `HealthDamage` NVP through the damage
//!   pipeline
//!
//! ## What's NOT here (intentional)
//!
//! - **No DB seed for `script_name` values.** Existing effect rows
//!   continue to flow through the legacy `HealthDamage`/`FocusDamage`
//!   NVP path in `damage_apply`. Operators opt in per-effect by
//!   setting `script_name` to one of the registered strings. A future
//!   migration can promote heal effects (659, 1383, 1215) en masse.
//! - **No pulse-tick model.** v1 fires `on_apply` once per use; the
//!   pulse-tick + `on_pulse_begin` extension lands when the first
//!   channelled effect goes in.
//! - **No effect_nvps DB schema migration.** The existing
//!   `effect.params` HashMap is already loaded from `resources.effect_nvps`
//!   by `load_effect_defs`. Scripts read it directly via `ctx.effect.param_*`.

pub mod registry;
pub mod scripts;

use crate::cell::space_manager::SpaceManager;
use cimmeria_entity::abilities::EffectDef;

/// Per-script execution context.
///
/// Carries everything a script needs to read source/target state, read
/// the effect's NVP params, and mutate the target via `space_mgr`.
/// The `source_id` is the entity that cast the ability (attacker or
/// self-heal caster); `target_id` is the entity the effect resolves on.
pub struct EffectContext<'a> {
    /// Entity that cast the ability that owns this effect.
    pub source_id: u32,
    /// Entity the effect lands on (may equal `source_id` for self-targeted).
    pub target_id: u32,
    /// The effect being applied. Read NVP params via `ctx.effect.param_i32`
    /// / `param_f32` — the helpers already on `EffectDef`.
    pub effect: &'a EffectDef,
    /// Mutable space manager — scripts read source stats then mutate the
    /// target. Borrow discipline lives inside each script.
    pub space_mgr: &'a mut SpaceManager,
}

/// One executable behavior keyed by `script_name`. Implementors live in
/// [`scripts`] and register themselves via the [`registry::dispatch`]
/// static table.
///
/// The `on_apply` method is called exactly once per effect invocation
/// in v1. Returns `Ok(())` on success or `Err(msg)` for diagnostic
/// logging (the caller logs at `warn!`; the error doesn't propagate to
/// the client). Scripts MUST be infallible from the gameplay
/// perspective — they handle missing entities, missing stats, etc.
/// internally and produce sensible no-ops on bad input.
pub trait EffectScript: Send + Sync {
    fn on_apply(&self, ctx: &mut EffectContext);
}

/// Dispatch an effect by name. Returns `true` when a registered script
/// ran, `false` when no script was registered for the name (caller
/// should fall through to legacy NVP path). Unknown names log at warn.
pub fn dispatch_by_name(name: &str, ctx: &mut EffectContext) -> bool {
    match registry::lookup(name) {
        Some(script) => {
            tracing::debug!(
                target: "abilities",
                event = "effect_script_dispatch",
                script = name,
                source_id = ctx.source_id,
                target_id = ctx.target_id,
                effect_id = ctx.effect.effect_id,
                "Dispatching effect script"
            );
            script.on_apply(ctx);
            true
        }
        None => {
            tracing::warn!(
                target: "abilities",
                event = "effect_script_unknown",
                script = name,
                source_id = ctx.source_id,
                target_id = ctx.target_id,
                effect_id = ctx.effect.effect_id,
                "Effect has script_name but no script registered — falling \
                 back to legacy NVP path; add a script to cell/effects/scripts/ \
                 or correct the effect's script_name value"
            );
            false
        }
    }
}
