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

/// Why [`interact_range`] refused a target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InteractRangeFail {
    /// The interacting entity is not in any loaded space.
    PlayerMissing,
    /// The target is not in any loaded space.
    TargetMissing,
    /// Both exist, in different spaces.
    OtherSpace,
    /// Same space, farther apart than `MAX_INTERACT_DISTANCE`.
    TooFar {
        /// The distance, in world units.
        dist: f32,
    },
}

/// The interaction range rule itself, with no logging: the target exists,
/// shares the player's space, and is within `MAX_INTERACT_DISTANCE`.
///
/// [`interact_target_in_range`] is this plus its log lines. The vault's
/// move predicate (`bank::vault_move_allowed`) calls it directly so a vault
/// check applies exactly the rule the interact that opened the vault did,
/// and reports its own reason instead of an "interact:" line.
pub fn interact_range(
    entity_id: u32,
    target_entity_id: u32,
    space_mgr: &SpaceManager,
) -> Result<(), InteractRangeFail> {
    let player_pos = space_mgr
        .get_entity(entity_id)
        .ok_or(InteractRangeFail::PlayerMissing)?
        .position;
    let target_pos = space_mgr
        .get_entity(target_entity_id)
        .ok_or(InteractRangeFail::TargetMissing)?
        .position;

    // Positions are per-space coordinates and `get_entity` searches every
    // space, so without this a target in another space at the same
    // coordinates would pass as "in range". That let a client pin a trainer
    // (or fire a chain) in a space it never entered (AT-04 review).
    if space_mgr.get_entity_space_id(entity_id) != space_mgr.get_entity_space_id(target_entity_id) {
        return Err(InteractRangeFail::OtherSpace);
    }

    // Compare squared distances so the common (in-range) path does no
    // sqrt. This runs on every interact, including the right-click spam
    // of ordinary play. The sqrt is paid only on the rejection branch,
    // where it buys a distance an operator can read in world units.
    let dist_sq = player_pos.distance_squared_to(&target_pos);
    if dist_sq > MAX_INTERACT_DISTANCE * MAX_INTERACT_DISTANCE {
        return Err(InteractRangeFail::TooFar {
            dist: dist_sq.sqrt(),
        });
    }
    Ok(())
}
