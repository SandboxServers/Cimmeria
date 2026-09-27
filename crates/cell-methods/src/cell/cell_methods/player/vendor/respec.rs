//! Respec: cell method 72 `resetMyAbilities` (AT-08, D-AT03, D-AT10).
//!
//! The client's `respecAbilities()` sends the call with no arguments
//! (AT-E1 Q5, `docs/reverse-engineering/findings/ability-trainer-ui.md` §5).
//! The cell gates it and forwards `CellToBaseMsg::ResetAbilities`; the base
//! applies one guarded `UPDATE` and answers with
//! `BaseToCellMsg::AbilitiesReset`, handled in
//! `service/base_messages/respec.rs`.

use tokio::sync::mpsc;

use crate::ability_tree::{
    TrainerPin, RESPEC_COST_NAQUADAH, RESPEC_FEEDBACK_NOTHING_TRAINED,
    RESPEC_FEEDBACK_NOT_AT_TRAINER,
};
use crate::cell::interactions::{send_respec_rejection, trainer_pin};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

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
