//! Server-initiated effect application.
//!
//! The combat pipeline (`cell::abilities::use_ability::handle_use_ability`)
//! is the *client* entry point for firing an ability, and it is gated
//! accordingly: the caster must know the ability, the target must be a
//! hostile NPC, and the resolution path is unconditionally QR damage.
//! Those gates are correct for client input and wrong for content — a
//! scripted debuff like the Castle Cellblock wake-up sickness is applied
//! to a player who has never trained the ability, targets *itself*, and
//! is not damage at all.
//!
//! This module is the server-authoritative counterpart: it skips the
//! caster/target/hostility gates and goes straight to the effect layer.
//!
//! # Reachability is the security boundary
//!
//! Bypassing the combat gates is only safe because nothing client-driven
//! can reach this code. Three properties enforce that, and all three are
//! load-bearing:
//!
//! 1. **The module is private.** `cell/content/mod.rs` declares
//!    `mod effect_apply;` without `pub`, so the path
//!    `crate::cell::content::effect_apply` does not exist outside
//!    `cell::content`. A `use` from a cell method or a Mercury handler is
//!    a hard compile error, not a lint.
//! 2. **The functions are `pub(super)`.** Even within the crate the only
//!    modules that can name them are `cell::content` and its descendants
//!    — in practice `cell::content::executor`.
//! 3. **The ability/effect id is never client-supplied.** Both entry
//!    points take their id from a `content_actions` seed row that was
//!    loaded from the database at startup. No wire field reaches these
//!    parameters.
//!
//! Do not widen any of the three. Re-exporting these functions from
//! `cell::content`, or threading a client-supplied id into `ability_id` /
//! `effect_id`, turns this into an "apply arbitrary effect to arbitrary
//! entity" primitive that any forged packet can drive.

use std::time::Instant;

use tokio::sync::mpsc;

use crate::cell::abilities::send_entity_method;
use crate::cell::effects::{self, EffectContext};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Apply every effect owned by `ability_id` to `target_id`, sourced from
/// `invoker_id`. Backs `Action::LaunchAbility`.
///
/// Returns the number of effects that produced a lasting active-effect
/// instance on the target. A return of `0` is normal, not an error: an
/// ability whose effects are all single-shot (`pulse_count == 1`) and
/// script-less resolves to a no-op, which is what the effect definition
/// asked for.
///
/// `invoker_id` becomes the effect instance's source, which matters for
/// stacking — `register_active_effect` refreshes same-invoker instances
/// and stacks different-invoker ones. Self-applied content debuffs pass
/// the same id for both, so re-firing the chain refreshes rather than
/// stacking.
pub(super) async fn apply_ability_effects(
    ability_id: i32,
    target_id: u32,
    invoker_id: u32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> usize {
    // Clone the id list up front — the per-effect work needs `&mut
    // space_mgr` and would otherwise hold an immutable borrow of
    // `ability_defs` across the loop.
    let Some(effect_ids) = space_mgr
        .ability_defs
        .get(&ability_id)
        .map(|d| d.effect_ids.clone())
    else {
        // Seed drift: a `launch_ability` row names an ability that
        // `load_ability_defs` didn't produce. Operator-actionable, and
        // unreachable from client input, so warn is the right level.
        tracing::warn!(
            target: "abilities",
            event = "content_launch_ability_unknown",
            ability_id,
            target_id,
            invoker_id,
            chain_id,
            "Content launch_ability names an ability with no server def -- \
             nothing applied; check the content_actions seed against \
             resources.abilities"
        );
        return 0;
    };

    let mut registered = 0usize;
    for effect_id in effect_ids {
        if apply_effect(effect_id, target_id, invoker_id, chain_id, tx, space_mgr).await {
            registered += 1;
        }
    }

    tracing::info!(
        target: "abilities",
        event = "content_launch_ability",
        ability_id,
        target_id,
        invoker_id,
        chain_id,
        registered,
        "Content: launched ability (server-initiated, combat gates bypassed)"
    );

    registered
}

/// Apply a single effect to `target_id`. Backs `Action::ApplyEffect`, and
/// is the per-effect body of [`apply_ability_effects`].
///
/// Returns `true` when the effect registered a lasting active-effect
/// instance. Single-shot effects return `false` after running their
/// script (if any) — they have no duration to track.
///
/// Note that `ability_id` is *not* a parameter: `register_active_effect`
/// self-sources it from `EffectDef.ability_id` and owns the
/// `onTimerUpdate` buff-icon send, so neither is duplicated here.
pub(super) async fn apply_effect(
    effect_id: i32,
    target_id: u32,
    invoker_id: u32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let Some(effect) = space_mgr.effect_defs.get(&effect_id).cloned() else {
        tracing::warn!(
            target: "abilities",
            event = "content_apply_effect_unknown",
            effect_id,
            target_id,
            invoker_id,
            chain_id,
            "Content effect application names an effect with no server def -- \
             skipped; check the seed against resources.effects"
        );
        return false;
    };

    if space_mgr.get_entity(target_id).is_none() {
        tracing::warn!(
            target: "abilities",
            event = "content_apply_effect_no_target",
            effect_id,
            target_id,
            chain_id,
            "Content effect application target entity is gone -- skipped"
        );
        return false;
    }

    // Script dispatch. `script_name` is NULL on most seeded effects; those
    // carry their behaviour entirely in the pulse registration below, so a
    // `None` here is the common case and not worth a log line.
    if let Some(script_name) = effect.script_name.clone() {
        let mut ctx = EffectContext {
            source_id: invoker_id,
            target_id,
            effect: &effect,
            space_mgr,
        };
        effects::dispatch_by_name(&script_name, &mut ctx);

        // Flush whatever the script mutated. Scripts write stats through
        // the dirty-tracking setters but never send; the combat path
        // flushes for them in `damage_apply`, and this path has to do the
        // same or a content-applied heal/debuff mutates server state the
        // client never learns about.
        if let Some(target) = space_mgr.get_entity_mut(target_id) {
            let dirty = target.stats.serialize_dirty();
            target.stats.clear_dirty();
            if !dirty.is_empty() {
                send_entity_method(
                    target_id,
                    crate::mercury::method_idx::ON_STAT_UPDATE,
                    dirty,
                    tx,
                    space_mgr,
                )
                .await;
            }
        }
    }

    // Register the pulsing instance. No-ops and returns false for
    // single-shot effects; also emits the `onTimerUpdate` buff icon.
    effects::register_active_effect(
        space_mgr,
        target_id,
        invoker_id,
        &effect,
        Instant::now(),
        tx,
    )
    .await
}
