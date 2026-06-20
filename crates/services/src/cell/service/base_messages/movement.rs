//! `BaseToCellMsg::EntityMove` handler — the client-authoritative player
//! position-update path. Runs the movement validator, snaps rejected clients
//! back via `TeleportPlayer`, and emits the sampled movement telemetry.
//! Extracted from `base_messages/mod.rs` as a pure code move.

use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::mpsc;

use cimmeria_entity::movement_validation::MovementReject;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{ClientMoveOutcome, SpaceManager};

/// Stable metric/log label for a movement reject reason. Kept low-
/// cardinality (one token per layer) so the `movement_validation_rejects_total`
/// counter stays aggregatable.
fn movement_reject_label(reason: MovementReject) -> &'static str {
    match reason {
        MovementReject::OutOfBounds => "bounds",
        MovementReject::OffNavmesh => "navmesh",
        MovementReject::Teleport => "teleport",
    }
}

/// 1-in-N sampling rate for player position updates. At the 10 Hz
/// client update rate, 10 = ~1 sample per second per active player —
/// enough to spot teleports / rubber-banding / stuck positions without
/// the per-frame noise. Bump up (e.g. 50) when the field is quiet
/// and movement is the least interesting signal.
const PLAYER_MOVE_LOG_SAMPLE: u64 = 10;

/// Process-wide counter for player-move sampling. Atomic so multi-cell
/// (future) doesn't need refactoring; single-cell (today) is just an
/// inc + modulo.
static PLAYER_MOVE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Handle `BaseToCellMsg::EntityMove`.
pub(super) async fn handle_entity_move(
    entity_id: u32,
    claimed_space_id: u32,
    position: [f32; 3],
    direction: [f32; 3],
    velocity: [f32; 3],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    tracing::trace!(entity_id, ?position, "EntityMove");
    // Server↔client space divergence. The write below is
    // server-authoritative (it uses the cell's own `entity_space`
    // binding, never `claimed_space_id`), so a mismatch cannot
    // corrupt the spatial grid; it is warn-only and exists to make
    // gate-travel / instance-reset races observable. A claimed id
    // of 0 is the pre-confirmation sentinel the client sends
    // before its space is bound — skip it to avoid benign startup
    // noise. Only a *known* binding that differs is a real
    // divergence: when the entity has no binding (`None`) the
    // packet is a stale post-disconnect leftover that the apply
    // below drops as `EntityMissing`, not a space mismatch.
    if let Some(actual_space_id) = space_mgr.get_entity_space_id(entity_id) {
        if claimed_space_id != 0 && actual_space_id != claimed_space_id {
            tracing::warn!(
                target: "movement.validation",
                entity_id,
                claimed_space_id,
                actual_space_id,
                reason = "space_mismatch",
                "movement.space_mismatch: client claims a different space than \
                 the server binding (warn-only — write uses the server binding)"
            );
            cimmeria_observability::counter!(
                "movement_validation_warns_total",
                "reason" => "space_mismatch",
            );
        }
    }
    // 1-in-N sampled debug log on the canonical player-move
    // target. Player movement is high volume (~10 Hz per
    // active player) and rarely the bug source, so sampling
    // gives operators "this player is alive and moving"
    // confirmation without flooding the log stream.
    let sample = PLAYER_MOVE_COUNTER.fetch_add(1, Ordering::Relaxed);
    if sample.is_multiple_of(PLAYER_MOVE_LOG_SAMPLE) {
        tracing::debug!(
            target: "movement.player",
            event = "position_update",
            entity_id,
            x = position[0],
            y = position[1],
            z = position[2],
            vx = velocity[0],
            vy = velocity[1],
            vz = velocity[2],
            sample_index = sample,
            "player position update (sampled)"
        );
    }
    // Client-authoritative position updates go through the
    // movement validator. Server-authoritative paths (ring
    // transport, respawn, content teleport, NPC movement) call
    // `update_entity_position` directly and bypass validation —
    // they are the source of truth for those entities and
    // already snap via `BASEMSG_FORCED_POSITION` where needed.
    let outcome = space_mgr.apply_client_position_update(entity_id, position, direction, velocity);
    match outcome {
        ClientMoveOutcome::Accepted { .. } => {}
        ClientMoveOutcome::EntityMissing => {
            // Stale inbound after destroy / disconnect.
            // Matches the legacy silent-drop shape of
            // `update_entity_position`; surface as debug so a
            // future deluge here is queryable but doesn't
            // alarm by default.
            tracing::debug!(
                target: "movement.validation",
                entity_id,
                reason = "entity_missing",
                "EntityMove dropped: entity not in any space (likely post-disconnect)"
            );
        }
        ClientMoveOutcome::Rejected {
            reason,
            last_valid,
            space_id,
            bounds,
        } => {
            // Negative-log per docs/architecture/negative-logging-convention.md.
            // `reason` carries the validation layer that fired
            // (`bounds` | `navmesh` | `teleport`); `bounds_min`/
            // `bounds_max` let an operator confirm which AABB the
            // proposed position was tested against without grepping.
            let reason_label = movement_reject_label(reason);
            tracing::warn!(
                target: "movement.validation",
                entity_id,
                space_id,
                client_x = position[0],
                client_y = position[1],
                client_z = position[2],
                last_valid_x = last_valid[0],
                last_valid_y = last_valid[1],
                last_valid_z = last_valid[2],
                bounds_min_x = bounds.min[0],
                bounds_min_y = bounds.min[1],
                bounds_min_z = bounds.min[2],
                bounds_max_x = bounds.max[0],
                bounds_max_y = bounds.max[1],
                bounds_max_z = bounds.max[2],
                reason = reason_label,
                reject = ?reason,
                "movement.validation_reject: client position rejected by the \
                 {reason_label} layer — snapping back to last valid via FORCED_POSITION"
            );
            // One low-cardinality `reason` label per layer so the
            // reject rate stays aggregatable without per-entity tags.
            cimmeria_observability::counter!(
                "movement_validation_rejects_total",
                "reason" => reason_label,
            );
            // Snap the offending client back. The cell entity's
            // position was NOT advanced — the next AoI tick
            // (100 ms) rebroadcasts the last-valid position to
            // witnesses, so witnesses never see the rejected
            // coordinates. TeleportPlayer routes through
            // `handle_teleport_player` which emits
            // `BASEMSG_FORCED_POSITION` to the owner; the
            // existing teleport bundle is the right primitive.
            if let Err(e) = tx
                .send(CellToBaseMsg::TeleportPlayer {
                    entity_id,
                    space_id,
                    position: last_valid,
                    prev_pos: last_valid,
                })
                .await
            {
                tracing::warn!(
                    entity_id,
                    space_id,
                    error = %e,
                    reason = "snap_back_send_failed",
                    "movement.bounds_violation: snap-back \
                     TeleportPlayer send to base failed — \
                     client will continue desynced"
                );
            }
        }
    }
}
