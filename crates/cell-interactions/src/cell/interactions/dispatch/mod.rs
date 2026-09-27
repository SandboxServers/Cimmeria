//! Top-level interaction routing — `handle_interact` and `handle_initial_response`.
//!
//! Split along the two entry points:
//! - [`interact`]          — `handle_interact` (the `interact(targetEntityId)`
//!   cell method): validate, distance-check, pin target, dispatch by type.
//! - [`initial_response`]  — `handle_initial_response` (the `initialResponse`
//!   cell method): recover the pinned NPC and fire the matching dialog.

mod initial_response;
mod interact;

#[cfg(test)]
mod tests;

use crate::cell::space_manager::SpaceManager;

pub use initial_response::handle_initial_response;
pub use interact::handle_interact;

// The range rule lives in `cimmeria-cell-world` so the vault verdict (BV-03)
// can be taken below this crate. `MAX_INTERACT_DISTANCE` stays `pub(super)`
// here so the sibling `loot` module re-validates looting distance against the
// same bound the initial `interact` enforces (#446).
pub(super) use crate::cell::space_manager::MAX_INTERACT_DISTANCE;
pub use crate::cell::space_manager::{interact_range, InteractRangeFail};

/// Does `entity_id` exist, does `target_entity_id` exist, are they in the
/// same space, and are they within `MAX_INTERACT_DISTANCE` of each other?
///
/// [`handle_interact`] performs this check inline because it needs the
/// positions and interaction data anyway. This standalone version exists
/// for the **outer** dispatcher in
/// `cell_methods::player::interaction::interact`, which reaches the
/// content-chain dispatch and the trainer UI *before* `handle_interact`
/// and so never inherited the check.
///
/// That gap was real: a client could send `interact` naming any tagged
/// NPC anywhere on the map and fire its content chains — accepting
/// missions, advancing steps, launching minigames — from arbitrary
/// distance, and (after the `last_interaction_target` pin moved ahead of
/// the chain dispatch) stamp an unvalidated entity id that
/// `handle_initial_response` puts straight onto the wire. Both the pin
/// and the dispatch are now gated on this.
///
/// Logs at `info` on rejection, matching the inner path's level: a
/// too-far click is ordinary client behaviour (lag, a moving target),
/// not an error.
pub fn interact_target_in_range(
    entity_id: u32,
    target_entity_id: u32,
    space_mgr: &SpaceManager,
) -> bool {
    match interact_range(entity_id, target_entity_id, space_mgr) {
        Ok(()) => true,
        Err(InteractRangeFail::PlayerMissing) => {
            tracing::info!(
                entity_id,
                target_entity_id,
                "interact: player entity not found"
            );
            false
        }
        Err(InteractRangeFail::TargetMissing) => {
            tracing::info!(
                entity_id,
                target_entity_id,
                "interact: target entity not found"
            );
            false
        }
        Err(InteractRangeFail::OtherSpace) => {
            tracing::info!(
                entity_id,
                target_entity_id,
                "interact: target is in another space"
            );
            false
        }
        Err(InteractRangeFail::TooFar { dist }) => {
            tracing::info!(
                entity_id,
                target_entity_id,
                dist,
                max = MAX_INTERACT_DISTANCE,
                "interact: too far away"
            );
            false
        }
    }
}
