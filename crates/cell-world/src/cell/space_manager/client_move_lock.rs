//! The content movement hold: a player pinned to a fixed anchor by a content
//! chain (cellblock-boot-lock D-BL4). Split out of `client_move.rs`.
//!
//! A held player may move at most [`HOLD_RADIUS`] (horizontal) from the
//! anchor. Anything further is refused as [`ClientMoveOutcome::Locked`]: the
//! position is never written and the snap-back budget is untouched. The caller
//! snaps the client back at most every [`HOLD_SNAP_INTERVAL`] and repeats the
//! hold message at most every [`HOLD_FEEDBACK_INTERVAL`]. Only the horizontal
//! (x/z) displacement counts, so a held player may still jump or fall.

use std::time::{Duration, Instant};

use cimmeria_common::Vector3;

use super::{ClientMoveOutcome, SpaceManager};

/// A content movement hold (cellblock-boot-lock D-BL4), stored in
/// `CellEntity::extensions`. Lives as long as the cell entity; a relog
/// rebuilds the entity without it and the seed re-applies it.
#[derive(Debug, Clone)]
pub struct MovementHold {
    pub anchor: [f32; 3],
    pub space_id: u32,
    pub chain_id: i64,
    pub client_flag: bool,
    pub message: Option<String>,
    pub last_snap: Option<Instant>,
    pub last_feedback: Option<Instant>,
}

/// Horizontal (x/z) slack around the anchor, in world units.
pub const HOLD_RADIUS: f32 = 1.0;
/// At most one snap-back per this interval (each is a base DB write).
pub const HOLD_SNAP_INTERVAL: Duration = Duration::from_millis(500);
/// At most one repeat of the hold message per this interval.
pub const HOLD_FEEDBACK_INTERVAL: Duration = Duration::from_secs(5);

impl SpaceManager {
    /// `None`: no hold, or the move is inside the slack. `Some(Locked)`: refuse it.
    pub(super) fn check_movement_hold(
        &mut self,
        now: Instant,
        entity_id: u32,
        space_id: u32,
        proposed: Vector3,
    ) -> Option<ClientMoveOutcome> {
        let entity = self.get_entity_mut(entity_id)?;
        let held_space = entity.extensions.get::<MovementHold>()?.space_id;
        if held_space != space_id {
            // The player left the held space (GM travel, world change): the hold is stale.
            entity.extensions.remove::<MovementHold>();
            return None;
        }
        let hold = entity.extensions.get_mut::<MovementHold>()?;
        let (dx, dz) = (proposed.x - hold.anchor[0], proposed.z - hold.anchor[2]);
        if dx * dx + dz * dz <= HOLD_RADIUS * HOLD_RADIUS {
            return None;
        }
        let snap = hold
            .last_snap
            .is_none_or(|t| now.saturating_duration_since(t) >= HOLD_SNAP_INTERVAL);
        if snap {
            hold.last_snap = Some(now);
        }
        let feedback = match &hold.message {
            Some(m)
                if hold
                    .last_feedback
                    .is_none_or(|t| now.saturating_duration_since(t) >= HOLD_FEEDBACK_INTERVAL) =>
            {
                let m = m.clone();
                hold.last_feedback = Some(now);
                Some(m)
            }
            _ => None,
        };
        Some(ClientMoveOutcome::Locked {
            anchor: hold.anchor,
            space_id,
            snap,
            feedback,
        })
    }
}
