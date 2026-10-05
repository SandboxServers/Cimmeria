//! `gmGiveTrainingPoints` — grant ability-tree training points to the calling
//! GM.
//!
//! Same shape as `gmGiveCash`: validate cell-side, forward a one-way
//! `GrantTrainingPoints` to the base, and let the base send the definitive
//! feedback line once its `UPDATE` commits. The base answers the cell with
//! `TrainingPointsGranted`, which mirrors the points onto the cell entity (the
//! trainer's purchase gate and `trainable` bytes read them) and sends the
//! client counter.

use tokio::sync::mpsc;

use super::feedback::send_gm_feedback;
use super::{forward_to_base, read_i32};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `gmGiveTrainingPoints(INT32 aNumTrainingPoints)` — grant to the caller.
///
/// Additive-only: the def has no remove command, and a negative grant would
/// let a GM drive the balance below zero, which the trainer's purchase guard
/// never expects. `<= 0` is refused with feedback.
pub(super) async fn handle_give_training_points(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let amount = match read_i32(args, 0) {
        Some(v) => v,
        None => {
            tracing::warn!(
                entity_id,
                entity_name = space_mgr.entity_label(entity_id),
                args_len = args.len(),
                "gmGiveTrainingPoints: truncated args (need INT32)"
            );
            send_gm_feedback(entity_id, "gmGiveTrainingPoints: missing INT32 amount", tx).await;
            return true;
        }
    };
    if amount <= 0 {
        tracing::warn!(
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            amount,
            "gmGiveTrainingPoints: non-positive amount rejected"
        );
        send_gm_feedback(
            entity_id,
            "gmGiveTrainingPoints: amount must be positive",
            tx,
        )
        .await;
        return true;
    }
    let player_id = match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
        Some(pid) => pid,
        None => {
            tracing::warn!(
                entity_id,
                entity_name = space_mgr.entity_label(entity_id),
                "gmGiveTrainingPoints: caller has no player_id"
            );
            send_gm_feedback(
                entity_id,
                "gmGiveTrainingPoints: caller is not a player",
                tx,
            )
            .await;
            return true;
        }
    };
    tracing::info!(
        entity_id,
        entity_name = space_mgr.entity_label(entity_id),
        player_id,
        player_name = space_mgr.entity_label(entity_id),
        amount,
        "gmGiveTrainingPoints: granting training points to GM"
    );
    // No optimistic "requested" line — the base sends the definitive one
    // after the `UPDATE` commits, or a refusal if the total would overflow.
    forward_to_base(
        tx,
        CellToBaseMsg::GrantTrainingPoints {
            entity_id,
            player_id,
            amount,
            gm_feedback_to: Some(entity_id),
        },
        "gmGiveTrainingPoints",
    )
    .await;
    true
}
