//! [`ClientMoveOutcome`]: what one client position update came to. Split out of `client_move.rs` (cellblock-boot-lock BL-01).

use cimmeria_entity::movement_validation::{MovementReject, SpaceBounds};

/// Outcome of `SpaceManager::apply_client_position_update`.
#[derive(Debug)]
pub enum ClientMoveOutcome {
    /// Position passed validation and was written. Carries the new
    /// position so callers that want to log it can do so without
    /// re-querying the entity.
    Accepted { position: [f32; 3] },
    /// Position failed validation and the cell entity is unchanged. The
    /// caller must emit `CellToBaseMsg::TeleportPlayer` so the offending
    /// client snaps back to `last_valid` — which has been checked to be a
    /// position this validator would itself accept.
    Rejected {
        reason: MovementReject,
        last_valid: [f32; 3],
        space_id: u32,
        /// The bounds the proposed position was tested against. Carried
        /// out so the caller's structured log can include `bounds_min`
        /// and `bounds_max` per the negative-logging convention.
        bounds: SpaceBounds,
    },
    /// Position failed validation **and** the entity's own authoritative
    /// position was unusable as a snap target. The entity has already been
    /// relocated to `recovered_to`, a terminal safe point; the caller snaps
    /// the client there instead of re-issuing the correction that was
    /// looping.
    ///
    /// Unusable is the *only* trigger. An entity standing somewhere the
    /// validator would itself accept is never relocated, however many
    /// strikes it has accrued — an exhausted budget on a sound position is
    /// [`Self::CorrectionSuppressed`]. Relocating there instead would be a
    /// server-initiated move of a player who never left a legal point, and
    /// (because recovery clears the budget) would make the budget
    /// unenforceable on any navmesh-backed world.
    Recovered {
        reason: MovementReject,
        /// The unusable position the entity was stuck at.
        from: [f32; 3],
        /// Where it now is — written to the cell entity before returning.
        recovered_to: [f32; 3],
        space_id: u32,
    },
    /// Position failed validation, the snap target is unusable, and no
    /// safe point could be resolved. The caller must **not** emit another
    /// `BASEMSG_FORCED_POSITION`: doing so is what produced the rubber-band
    /// loop. The cell entity is left alone and the next accepted position
    /// clears the state.
    CorrectionSuppressed {
        reason: MovementReject,
        from: [f32; 3],
        space_id: u32,
        /// Consecutive rejects for this entity, for the operator log.
        strikes: u32,
    },
    /// The entity is not currently in any space — likely a stale
    /// inbound packet that arrived after destroy / disconnect. The
    /// caller can safely no-op; the original `update_entity_position`
    /// silently dropped the same shape.
    EntityMissing,
}
