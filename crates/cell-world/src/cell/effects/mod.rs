//! Effect-script dispatcher.
//!
//! When an effect with a non-NULL `EffectDef.script_name` fires, the
//! registry looks up the named script and runs it. Scripts decide their own
//! behavior (heal, damage, buff) without each call site having to hand-
//! roll the NVP read + stat mutate + wire packet send.
//!
//! ## Architecture
//!
//! - **[`EffectScript`]** trait: `on_apply`, called once per pulse fire,
//!   and `on_remove`, called once when an active instance is swept
//!   (decision 1 of `docs/architecture/abilities-and-effects-system.md`).
//!
//! - **[`EffectContext`]**: carries the source/target ids, the effect's
//!   parameters (NVP bag from `effect.params`), and the mutable
//!   [`SpaceManager`] handle so scripts can read stats + mutate the
//!   target.
//!
//! - **[`registry::EffectScripts`]**: the `script_name` to script map.
//!   The composition root builds it at startup and the cell installs it on
//!   its `SpaceManager`; [`dispatch_by_name`] and [`dispatch_on_remove`]
//!   look scripts up there. Unknown names hit the warn log and no-op (don't
//!   crash). Names match the scheme the original game's content was
//!   authored against: `HealHealth`, `HealFocus`, `MeleeDamage`, and so on.
//!
//! ## Where the scripts are
//!
//! The `EffectScript` implementations are in `cimmeria-cell-effect-scripts`
//! (#962 step 4, `docs/architecture/plugin-architecture.md` §4.4), a leaf
//! only the composition root depends on, so adding or editing a script
//! rebuilds that crate and the root, not the cell track. A new script is an
//! `impl EffectScript` in that crate plus one row in its `EFFECT_SCRIPTS`
//! table; nothing here changes.
//!
//! What stays here is the runtime the scripts plug into and the pieces the
//! layers below the leaf call directly: the trait, the context, dispatch,
//! the registry type, the passive pass ([`passives`]), the pet-script name
//! predicates ([`pet_scripts`]), the stat-buff ledger ([`stat_buff`]) and
//! the special-ammo shot helpers combat's damage path reads
//! ([`ammo_damage`], [`ammo_explosive`]).
//!
//! ## Crate split
//!
//! This synchronous layer is in `cimmeria-cell-world`, because the
//! spawn-time cover hold runs Cover Stance through it. The async pulsing
//! layer (`pulsing`: register, tick, channel cancellation) is combat:
//! `cimmeria-cell-combat` declares it beside a re-export of this module at
//! `cell::effects`.

// The special-ammo shot helpers the damage path reads (ammo campaign AM-04,
// AM-10). The ammo families' scripts are in `cimmeria-cell-effect-scripts`.
// The AB-T5 state snapshot row (`abilities.snapshot`).
pub mod ability_snapshot;
pub mod ammo_damage;
pub mod ammo_explosive;
pub mod cast_scope;
pub mod interrupt_request;
pub mod passives;
pub mod pet_scripts;
pub mod registry;
pub mod stat_buff;

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
/// `cimmeria-cell-effect-scripts` and are registered in the
/// [`registry::EffectScripts`] the cell installs on its `SpaceManager`.
///
/// `on_apply` is called once per pulse fire (initial or re-pulse from
/// the per-tick scheduler) — it returns `()`, not a `Result`. Scripts
/// MUST be infallible from the gameplay perspective: they handle
/// missing entities, missing stats, and unparseable NVPs internally
/// and produce sensible no-ops rather than panicking or propagating
/// errors. Bad input is surfaced via `tracing::warn!` so operators
/// see it without the call stack unwinding.
///
/// `on_remove` is called once when an active-effect instance is swept
/// (remaining_pulses reaches 0, target dies, attacker cancels a
/// channel). Default impl is a no-op — only scripts that mutated
/// persistent state on apply (Stun's state-flag, AbsorbShield's pool
/// capacity) need to override.
pub trait EffectScript: Send + Sync {
    fn on_apply(&self, ctx: &mut EffectContext);
    /// Called exactly once when the owning instance is removed from
    /// the entity's `active_effects`. Use for stateful cleanup —
    /// clear flags, restore stats, drain residual buffs. Skipped for
    /// single-shot effects (`pulse_count == 1`) since they never
    /// register an active instance.
    fn on_remove(&self, _ctx: &mut EffectContext) {}
}

/// Dispatch an effect by name. Returns `true` when a registered script
/// ran, `false` when no script was registered for the name (caller
/// should fall through to legacy NVP path). Unknown names log at warn.
pub fn dispatch_by_name(name: &str, ctx: &mut EffectContext) -> bool {
    match ctx.space_mgr.effect_scripts().lookup(name) {
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
                 back to legacy NVP path; add the script to the EFFECT_SCRIPTS \
                 table in cimmeria-cell-effect-scripts or correct the effect's \
                 script_name value"
            );
            false
        }
    }
}

/// Look up the script for `name` and call `on_remove`. Returns `true`
/// when a script was found. No-ops (returns `false`) for effects that
/// don't have a script_name OR whose script isn't registered — these
/// don't need cleanup.
pub fn dispatch_on_remove(name: &str, ctx: &mut EffectContext) -> bool {
    match ctx.space_mgr.effect_scripts().lookup(name) {
        Some(script) => {
            tracing::debug!(
                target: "abilities",
                event = "effect_script_remove",
                script = name,
                source_id = ctx.source_id,
                target_id = ctx.target_id,
                effect_id = ctx.effect.effect_id,
                "Dispatching effect script on_remove"
            );
            script.on_remove(ctx);
            true
        }
        None => false,
    }
}
