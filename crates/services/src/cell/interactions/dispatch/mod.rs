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

/// Maximum distance for NPC interaction (world units).
/// From `python/common/Constants.py: MAX_INTERACT_DISTANCE = 5`.
///
/// `pub(super)` so the sibling `loot` module can re-validate looting
/// distance against the same bound the initial `interact` enforces
/// (#446 — the loot handler must re-check range on every take, not just
/// trust the interact-time `looting_entity` pin).
pub(super) const MAX_INTERACT_DISTANCE: f32 = 5.0;

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
pub(crate) fn interact_target_in_range(
    entity_id: u32,
    target_entity_id: u32,
    space_mgr: &SpaceManager,
) -> bool {
    let player_pos = match space_mgr.get_entity(entity_id) {
        Some(e) => e.position,
        None => {
            tracing::info!(
                entity_id,
                target_entity_id,
                "interact: player entity not found"
            );
            return false;
        }
    };
    let target_pos = match space_mgr.get_entity(target_entity_id) {
        Some(e) => e.position,
        None => {
            tracing::info!(
                entity_id,
                target_entity_id,
                "interact: target entity not found"
            );
            return false;
        }
    };

    // Positions are per-space coordinates and `get_entity` searches every
    // space, so without this a target in another space at the same
    // coordinates would pass as "in range". That let a client pin a trainer
    // (or fire a chain) in a space it never entered (AT-04 review).
    if space_mgr.get_entity_space_id(entity_id) != space_mgr.get_entity_space_id(target_entity_id) {
        tracing::info!(
            entity_id,
            target_entity_id,
            "interact: target is in another space"
        );
        return false;
    }

    // Compare squared distances so the common (in-range) path does no
    // sqrt. This runs on every interact, including the right-click spam
    // of ordinary play. The sqrt is paid only on the rejection branch,
    // where it buys a log line an operator can read in world units.
    let dist_sq = player_pos.distance_squared_to(&target_pos);
    if dist_sq > MAX_INTERACT_DISTANCE * MAX_INTERACT_DISTANCE {
        tracing::info!(
            entity_id,
            target_entity_id,
            dist = dist_sq.sqrt(),
            max = MAX_INTERACT_DISTANCE,
            "interact: too far away"
        );
        return false;
    }
    true
}
