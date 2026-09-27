//! Respec: cell method 72 `resetMyAbilities` (AT-08, D-AT03, D-AT10).
//!
//! The client's `respecAbilities()` sends the call with no arguments
//! (AT-E1 Q5, `docs/reverse-engineering/findings/ability-trainer-ui.md` §5).
//! The cell gates it and forwards `CellToBaseMsg::ResetAbilities`; the base
//! applies one guarded `UPDATE` and answers with
//! `BaseToCellMsg::AbilitiesReset`, handled in
//! `service/base_messages/respec.rs`.

use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use crate::ability_tree::{
    TrainerPin, RESPEC_COST_NAQUADAH, RESPEC_FEEDBACK_NOTHING_TRAINED,
    RESPEC_FEEDBACK_NOT_AT_TRAINER,
};
use crate::cell::interactions::{send_respec_rejection, trainer_pin};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// How long after a forwarded respec another press is dropped. The base
/// answers well inside it, so the press it drops is a double-click or a
/// spamming client, and the first press's answer is still on its way. It
/// bounds the base's row-locking `UPDATE` to one per player per window,
/// which matters for a player short of naquadah: the cell cannot see the
/// balance, so every such press would otherwise reach the base.
const RESPEC_RETRY_WINDOW: Duration = Duration::from_secs(1);

/// Why the cell refused a respec before asking the base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Refusal {
    /// The pin is missing, despawned, not a trainer, or out of range: the
    /// same authority a purchase needs (AT-04's `trainer_pin`). The offered
    /// list does not matter for a respec.
    NotAtTrainer(&'static str),
    /// The cell's mirror shows no trainer-bought ability and no spend.
    NothingTrained,
}

/// Handle `resetMyAbilities` for `entity_id`.
///
/// Gates, in order:
/// 1. The entity is a loaded character (`player_id`). A legitimate client
///    cannot fail this, so it is silent (WARN only), as for `trainAbility`.
/// 2. The pinned `last_interaction_target` is a live trainer that still
///    passes `interact_target_in_range`.
/// 3. Something is trainer-bought (`tree_progress`). A replay after a
///    successful respec stops here without a DB round trip; the base's
///    `UPDATE` guard is the authority when the two race.
///
/// A press that passes all three within `RESPEC_RETRY_WINDOW` of the last
/// forwarded one is dropped silently: the earlier press's answer is still
/// coming, so it is not a first press.
///
/// Refusals 2 and 3 answer with `onErrorCode` and the pinned trainer's
/// re-send (`send_respec_rejection`). Refusal 3 is a first press too: the
/// player clicked an enabled Respec button with nothing to reset, and the
/// project rule wants that press answered. The cost is bounded: gate 2
/// runs first, so only a player standing at a trainer gets the re-send.
pub(crate) async fn handle_reset_my_abilities(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let (player_id, refusal) = {
        let Some(entity) = space_mgr.get_entity(entity_id) else {
            return;
        };
        let Some(player_id) = entity.player_id else {
            tracing::warn!(
                entity_id,
                "resetMyAbilities: entity has no player_id — rejecting"
            );
            return;
        };
        let pin = trainer_pin(
            space_mgr,
            entity_id,
            entity.last_interaction_target,
            entity.archetype_id,
        );
        let progress = &entity.tree_progress;
        let refusal = match pin {
            TrainerPin::Unpinned => Some(Refusal::NotAtTrainer("no_trainer_pinned")),
            TrainerPin::Despawned => Some(Refusal::NotAtTrainer("trainer_despawned")),
            TrainerPin::NotATrainer => Some(Refusal::NotAtTrainer("pin_not_a_trainer")),
            TrainerPin::Trainer {
                in_range: false, ..
            } => Some(Refusal::NotAtTrainer("trainer_out_of_range")),
            TrainerPin::Trainer { in_range: true, .. }
                if progress.trained_abilities.is_empty() && progress.tree_points_spent == 0 =>
            {
                Some(Refusal::NothingTrained)
            }
            TrainerPin::Trainer { in_range: true, .. } => None,
        };
        (player_id, refusal)
    };

    let now = Instant::now();
    if refusal.is_none() {
        let recent = space_mgr
            .get_entity(entity_id)
            .and_then(|e| e.respec_requested_at)
            .is_some_and(|at| now.duration_since(at) < RESPEC_RETRY_WINDOW);
        if recent {
            tracing::debug!(
                target: "abilities",
                event = "respec_dropped",
                entity_id,
                player_id,
                "resetMyAbilities: a respec is already on its way — dropping the repeat"
            );
            return;
        }
        if let Some(e) = space_mgr.get_entity_mut(entity_id) {
            e.respec_requested_at = Some(now);
        }
    }

    match refusal {
        Some(Refusal::NotAtTrainer(reason)) => {
            // Info, not warn: walking away with the window open is ordinary
            // play, and a forged call looks the same on the wire.
            tracing::info!(
                target: "abilities",
                event = "respec_rejected",
                reason,
                entity_id,
                player_id,
                "resetMyAbilities: not at a trainer — rejecting"
            );
            send_respec_rejection(
                entity_id,
                RESPEC_FEEDBACK_NOT_AT_TRAINER,
                reason,
                tx,
                space_mgr,
            )
            .await;
        }
        Some(Refusal::NothingTrained) => {
            tracing::info!(
                target: "abilities",
                event = "respec_rejected",
                reason = "nothing_trained",
                entity_id,
                player_id,
                "resetMyAbilities: nothing trainer-bought — no change, no charge"
            );
            send_respec_rejection(
                entity_id,
                RESPEC_FEEDBACK_NOTHING_TRAINED,
                "nothing_trained",
                tx,
                space_mgr,
            )
            .await;
        }
        None => {
            tracing::info!(
                target: "abilities",
                event = "respec_requested",
                entity_id,
                player_id,
                cost = RESPEC_COST_NAQUADAH,
                "resetMyAbilities: at a trainer, requesting base reset + charge"
            );
            if let Err(e) = tx
                .send(CellToBaseMsg::ResetAbilities {
                    entity_id,
                    player_id,
                    cost: RESPEC_COST_NAQUADAH,
                })
                .await
            {
                tracing::error!(
                    target: "abilities",
                    event = "respec_send_failed",
                    entity_id,
                    player_id,
                    error = %e,
                    "ResetAbilities cell→base send failed — nothing reset"
                );
            }
        }
    }
}
