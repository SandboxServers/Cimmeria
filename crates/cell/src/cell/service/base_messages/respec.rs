//! `BaseToCellMsg::AbilitiesReset`: the cell half of a trainer respec
//! (AT-08).
//!
//! The base has already applied (or refused) its one guarded `UPDATE`. On
//! [`RespecOutcome::Reset`] the cell mirrors the row, cancels a warming
//! cast of a refunded ability, and sends the respec burst in this order:
//!
//! 1. `onKnownAbilitiesUpdate`: the Ability window and the hotbar's
//!    known-ability list lose the refunded ids. This is the "hotbar
//!    re-send": the client's action-bar bindings live in a client-side Lua
//!    saved variable the server cannot edit, so stale buttons stay until
//!    the player clears them (a press is refused as an unknown ability).
//! 2. `onEntityProperty(GENERICPROPERTY_TrainingPoints, n)`: the refunded
//!    point counter.
//! 3. `onCashChanged(naquadah)`: the balance after the charge.
//! 4. `onTrainerOpen` re-send, while a trainer is pinned: every node's
//!    `trainable` byte from the reset spend and points.
//!
//! A refusal sends `onErrorCode` and the trainer re-send
//! (`interactions::send_respec_rejection`); nothing is mirrored because
//! nothing changed.

use tokio::sync::mpsc;

use crate::ability_tree::{
    RespecOutcome, RESPEC_FEEDBACK_NOTHING_TRAINED, RESPEC_FEEDBACK_NOT_ENOUGH_NAQUADAH,
};
use crate::cell::client_methods::inventory::ON_CASH_CHANGED;
use crate::cell::interactions::{resend_pinned_trainer, send_respec_rejection};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::ability_granted::send_training_points;
use super::player_init::send_known_abilities_update;

/// Handle `BaseToCellMsg::AbilitiesReset`.
pub(super) async fn handle_abilities_reset(
    entity_id: u32,
    player_id: i32,
    outcome: RespecOutcome,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // The base checked the session before its `UPDATE`, not before this
    // reply. If the id now belongs to another character, neither the mirror
    // nor the feedback is theirs.
    let current = space_mgr.get_entity(entity_id).and_then(|e| e.player_id);
    if current != Some(player_id) {
        tracing::warn!(
            target: "abilities",
            event = "respec_player_mismatch",
            entity_id,
            player_id,
            current_player_id = ?current,
            "AbilitiesReset: entity no longer plays the reset character — ignoring"
        );
        return;
    }
    let (refunded, training_points, naquadah) = match outcome {
        RespecOutcome::Reset {
            refunded,
            training_points,
            naquadah,
        } => (refunded, training_points, naquadah),
        RespecOutcome::NothingToReset => {
            send_respec_rejection(
                entity_id,
                RESPEC_FEEDBACK_NOTHING_TRAINED,
                "nothing_trained",
                tx,
                space_mgr,
            )
            .await;
            return;
        }
        RespecOutcome::NotEnoughNaquadah { .. } => {
            send_respec_rejection(
                entity_id,
                RESPEC_FEEDBACK_NOT_ENOUGH_NAQUADAH,
                "not_enough_naquadah",
                tx,
                space_mgr,
            )
            .await;
            return;
        }
    };

    // Mirror before any send, so the trainer re-send computes `trainable`
    // from the reset spend and points: a root is buyable again at once.
    let Some(entity) = space_mgr.get_entity_mut(entity_id) else {
        return; // checked above
    };
    for &ability_id in &refunded {
        entity.abilities.remove_ability(ability_id);
    }
    let progress = &mut entity.tree_progress;
    progress.trained_abilities.clear();
    progress.tree_points_spent = 0;
    progress.training_points = training_points;
    // A refunded passive's effect comes off with it (pets PT-08).
    let _passives = super::passive_sync::apply_passives_and_sync(
        entity_id,
        &refunded,
        crate::cell::effects::passives::PassiveChange::Unlearned,
        tx,
        space_mgr,
    )
    .await;

    // A cast of a refunded ability parked in its warmup (AT-10) would fire
    // an ability the player no longer knows. Its interrupt frames go out
    // before the burst.
    let interrupted =
        crate::cell::abilities::interrupt_unlearned_cast(entity_id, &refunded, tx, space_mgr).await;

    tracing::info!(
        target: "abilities",
        event = "respec_applied",
        entity_id,
        refunded = ?refunded,
        training_points,
        naquadah,
        interrupted_warmup = interrupted,
        "AbilitiesReset: cell mirrored + respec burst"
    );

    send_known_abilities_update(entity_id, "respec", tx, space_mgr).await;
    send_training_points(entity_id, training_points, tx).await;
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_CASH_CHANGED,
            args: naquadah.to_le_bytes().to_vec(),
        })
        .await
    {
        tracing::warn!(
            target: "abilities",
            event = "respec_cash_send_failed",
            entity_id,
            naquadah,
            error = %e,
            "AbilitiesReset: onCashChanged send failed; the balance shows stale until relog"
        );
    }
    resend_pinned_trainer(entity_id, tx, space_mgr).await;
}
