//! Auto-reload trigger after an ammo-consuming fire.
//!
//! Split out of the main `handle_use_ability` flow: when a player fire
//! drains the active clip to zero and the player has the `autoReload`
//! client option set, this requests a reload via the same path the
//! manual `R` keypress takes.

/// If this fire decremented player ammo to zero AND the player's
/// `autoReload` client option is set, automatically request a reload —
/// the same path the manual `R` keypress takes.
///
/// Routes through [`crate::cell::cell_methods::player::world::handle_reload`]
/// so the Phase A draw-window logic and Phase B warmup/cooldown are both
/// honoured uniformly with manual reloads. Gated on `is_player` because
/// NPCs do not consume ammo (their `set_slot_ammo` branch never runs).
///
/// Called after `apply_damage_to_target` releases the entity borrow so
/// `handle_reload` can re-acquire `&mut SpaceManager` for its own state
/// mutations. Re-entrancy with the per-tick `pending_reload_tick` is fine:
/// `handle_reload`'s phase-A guard rejects a second invocation while the
/// draw window is mid-flight (`pending_reload_at` is in the future).
pub(super) async fn maybe_trigger_auto_reload(
    entity_id: u32,
    ammo_was_consumed: bool,
    ability_id: i32,
    tx: &tokio::sync::mpsc::Sender<crate::cell::messages::CellToBaseMsg>,
    space_mgr: &mut crate::cell::space_manager::SpaceManager,
) {
    if !ammo_was_consumed {
        return;
    }
    // Snapshot the gate inside an immutable borrow; release before
    // `handle_reload` re-acquires the mutable borrow.
    let should_reload = match space_mgr.get_entity(entity_id) {
        Some(e) => {
            e.is_player
                && e.system_options.auto_reload
                // Active slot must actually carry a clip — melee
                // weapons report `active_clip_size() == 0` and
                // `active_ammo() == 0` would otherwise look like
                // "out of bullets" and queue a no-op reload that the
                // `handle_reload` "already at max ammo" early-return
                // would catch. Skipping here saves one wasted call.
                && e.active_clip_size() > 0
                && e.active_ammo() == 0
                // Don't queue auto-reload on top of an in-flight reload —
                // `handle_reload` would early-return at "already at max
                // ammo" or push another deferred phase A on top of the
                // existing one.
                && e.reload_complete_at.is_none()
                && e.pending_reload_at.is_none()
        }
        None => false,
    };
    if !should_reload {
        return;
    }
    tracing::info!(
        entity_id,
        ability_id,
        "useAbility: auto-reload triggered (autoReload + clip empty)"
    );
    crate::cell::cell_methods::player::world::handle_reload(entity_id, tx, space_mgr).await;
}

/// Test-only re-export of [`maybe_trigger_auto_reload`] so cross-module
/// tests can pin the gate decisions directly without driving the full
/// fire pipeline. Production paths inside this module call the
/// non-public version.
#[cfg(test)]
pub(crate) async fn maybe_trigger_auto_reload_for_test(
    entity_id: u32,
    ammo_was_consumed: bool,
    ability_id: i32,
    tx: &tokio::sync::mpsc::Sender<crate::cell::messages::CellToBaseMsg>,
    space_mgr: &mut crate::cell::space_manager::SpaceManager,
) {
    maybe_trigger_auto_reload(entity_id, ammo_was_consumed, ability_id, tx, space_mgr).await;
}
