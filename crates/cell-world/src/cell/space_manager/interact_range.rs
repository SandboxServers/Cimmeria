//! The interaction range rule: the target exists, shares the player's space,
//! and is within [`MAX_INTERACT_DISTANCE`].
//!
//! It lives here, below `cimmeria-cell-interactions`, so that the vault
//! verdict ([`super::vault_access`]) can be taken by every crate that
//! forwards an inventory request, the content executor included.
//! `cell::interactions::dispatch` re-exports it at its old path.

use super::SpaceManager;

/// Maximum distance for NPC interaction (world units).
/// From `python/common/Constants.py: MAX_INTERACT_DISTANCE = 5`.
pub const MAX_INTERACT_DISTANCE: f32 = 5.0;

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
/// `interact_target_in_range` (`cell::interactions::dispatch`) is this plus
/// its log lines. The vault predicate ([`super::vault_move_allowed`]) calls
/// it directly so a vault check applies exactly the rule the interact that
/// opened the vault did, and reports its own reason instead of an
/// "interact:" line.
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
