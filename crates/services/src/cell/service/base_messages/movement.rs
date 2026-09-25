//! `BaseToCellMsg::EntityMove` handler — the client-authoritative player
//! position-update path. Runs the movement validator, snaps rejected clients
//! back via `TeleportPlayer`, and emits the sampled movement telemetry.
//! Extracted from `base_messages/mod.rs` as a pure code move.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_entity::movement_validation::MovementReject;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{
    ClientMoveOutcome, HardReject, RecoveryReport, RejectReport, SpaceManager, SuppressionReport,
};

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
    direction: [i8; 3],
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
            let id = space_mgr.player_identity(entity_id);
            // `world` on this row too: the server-side binding is known
            // here, so a `movement.validation` dashboard filtered by
            // world must not silently drop space-mismatch rows. It
            // names the world the server believes the player is in —
            // the claimed id is by definition not a binding this
            // process can resolve.
            let world = space_mgr
                .world_name_for_space(actual_space_id)
                .unwrap_or("unknown");
            tracing::warn!(
                target: "movement.validation",
                entity_id,
                account_id = id.account_id,
                player_id = id.player_id,
                claimed_space_id,
                actual_space_id,
                world = %world,
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
    // Stuck-player detectors that need state over time (self-throttled).
    crate::cell::playtest_friction::player_tick(space_mgr, entity_id, position);
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
            // Negative-log + counter, both behind one helper: the row
            // needs the world name, the navmesh gate diagnosis and the
            // per-entity throttle, all of which live on the
            // `SpaceManager`. See
            // `cell::space_manager::movement_telemetry::reject` for why
            // the throttle exists (one stuck entity produced 71% of
            // three days' reject volume), why the counter is
            // incremented even for suppressed rows, and why all three
            // hard-reject outcomes below go through the same seam.
            //
            // `reason` still carries the validation layer that fired
            // (`bounds` | `navmesh` | `teleport`); `bounds_min`/
            // `bounds_max` still let an operator confirm which AABB the
            // proposed position was tested against without grepping.
            //
            // Identity comes back out rather than being resolved twice:
            // this handler runs ~10 Hz per active player, and the
            // accepted path never pays for the lookup at all.
            let id = space_mgr.report_movement_reject(
                RejectReport {
                    common: hard_reject(entity_id, space_id, reason, position),
                    last_valid,
                    bounds: &bounds,
                },
                Instant::now(),
            );
            // Snap the offending client back. The cell entity's
            // position was NOT advanced — the next AoI tick
            // (100 ms) rebroadcasts the last-valid position to
            // witnesses, so witnesses never see the rejected
            // coordinates. TeleportPlayer routes through
            // `handle_teleport_player` which emits
            // `BASEMSG_FORCED_POSITION` to the owner; the
            // existing teleport bundle is the right primitive.
            send_snap_back(entity_id, space_id, last_valid, id, tx).await;
        }
        ClientMoveOutcome::Recovered {
            reason,
            from,
            recovered_to,
            space_id,
        } => {
            let id = space_mgr.report_movement_recovered(
                RecoveryReport {
                    common: hard_reject(entity_id, space_id, reason, position),
                    from,
                    recovered_to,
                },
                Instant::now(),
            );
            send_snap_back(entity_id, space_id, recovered_to, id, tx).await;
        }
        ClientMoveOutcome::CorrectionSuppressed {
            reason,
            from,
            space_id,
            strikes,
        } => {
            // No `send_snap_back` — that is the whole point of this
            // outcome. Everything else about it is reported exactly as
            // the other two hard rejects are.
            space_mgr.report_correction_suppressed(
                SuppressionReport {
                    common: hard_reject(entity_id, space_id, reason, position),
                    from,
                    strikes,
                },
                Instant::now(),
            );
        }
    }
}

/// The reject facts every outcome shares. One constructor so a new
/// outcome cannot be wired up with a different `reason_label` mapping or
/// a different idea of which point the diagnosis describes.
fn hard_reject(
    entity_id: u32,
    space_id: u32,
    reason: MovementReject,
    position: [f32; 3],
) -> HardReject {
    HardReject {
        entity_id,
        space_id,
        reason,
        reason_label: movement_reject_label(reason),
        position,
    }
}

/// Push one `BASEMSG_FORCED_POSITION` at the owning client.
///
/// `position == prev_pos` so the client's interpolator sees a zero-distance
/// move and hard-sets rather than sliding into place.
///
/// `id` is passed in rather than re-resolved: the caller has already paid for
/// the lookup in the branch that decided to correct, and this is the failure
/// path a player reporting "I'm stuck rubber-banding" actually shows up as —
/// it has to name the account.
async fn send_snap_back(
    entity_id: u32,
    space_id: u32,
    position: [f32; 3],
    id: PlayerIdentity,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    if let Err(e) = tx
        .send(CellToBaseMsg::TeleportPlayer {
            entity_id,
            space_id,
            position,
            prev_pos: position,
        })
        .await
    {
        tracing::warn!(
            entity_id,
            account_id = id.account_id,
            player_id = id.player_id,
            space_id,
            error = %e,
            reason = "snap_back_send_failed",
            "movement.snap_back_send_failed: snap-back TeleportPlayer send to \
             base failed — client will continue desynced"
        );
    }
}
