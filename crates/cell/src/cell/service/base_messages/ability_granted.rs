//! `BaseToCellMsg::AbilityGranted` and `ProgressionChanged` handlers.
//!
//! `AbilityGranted` mirrors a base-persisted trainer purchase onto the cell
//! entity (known set, provenance, points, spend), then sends the grant burst
//! in this order:
//!
//! 1. `onKnownAbilitiesUpdate`: the hotbar and Ability window learn the id.
//! 2. `onEntityProperty(GENERICPROPERTY_TrainingPoints, n)`: the point
//!    counter drops at once, window open or closed (AT-E1 question 3;
//!    audit A-07, where the counter stayed stale until relog).
//! 3. `onTrainerOpen` re-send, only while a trainer is pinned, so the
//!    `trainable` bytes reflect the new points and spend.
//!
//! `ProgressionChanged` keeps the cell's level and points in step with a
//! base level-up, so the gates above read what the base debits against.

use tokio::sync::mpsc;

use crate::ability_tree::training_points_property_args;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::player_init::send_known_abilities_update;

/// The payload of one `BaseToCellMsg::AbilityGranted`.
#[derive(Debug, Clone, Copy)]
pub(super) struct Granted {
    pub(super) entity_id: u32,
    pub(super) ability_id: i32,
    /// `training_points` after the debit.
    pub(super) training_points: i32,
    /// `tree_points_spent` after the increment.
    pub(super) tree_points_spent: i32,
}

/// Handle `BaseToCellMsg::AbilityGranted`.
pub(super) async fn handle_ability_granted(
    granted: Granted,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Granted {
        entity_id,
        ability_id,
        training_points,
        tree_points_spent,
    } = granted;
    // Base persisted + debited; mirror onto the cell entity before any
    // send, so the trainer re-send below computes `trainable` from the
    // post-purchase points and spend.
    if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
        entity.abilities.add_ability(ability_id);
        let progress = &mut entity.tree_progress;
        if !progress.trained_abilities.contains(&ability_id) {
            progress.trained_abilities.push(ability_id);
        }
        progress.training_points = training_points;
        progress.tree_points_spent = tree_points_spent;
    }
    tracing::info!(
        target: "abilities",
        event = "granted",
        entity_id,
        ability_id,
        training_points,
        tree_points_spent,
        "AbilityGranted: cell mirrored + hotbar refresh"
    );
    send_known_abilities_update(entity_id, tx, space_mgr).await;
    send_training_points(entity_id, training_points, tx).await;

    // Python parity (`AbilityTrainer.onTrainAbility:128`): if the
    // newly-learned ability is a prerequisite for another offered
    // ability, OR the player just ran out of training points, the
    // trainer list should refresh so the client's UI updates the
    // greyed-out state. Without this, the player sees a stale list
    // with the dependent ability still greyed out until they close
    // and re-open the trainer.
    //
    // **Contract — "resend on ANY grant while pinned":** this fires
    // for every `AbilityGranted` while `last_interaction_target` is
    // set, regardless of whether the granted ability is in the
    // trainer's offered list. We delegate the "is this newly-unlocked
    // a prereq for B?" decision to `try_open_trainer` itself, which
    // recomputes every `trainable` flag from current state (known
    // set, level, prereqs). This matches Python's
    // `AbilityTrainer.onTrainAbility` behavior: it re-fires
    // `onTrainerOpen` unconditionally after a successful train RPC.
    // `try_open_trainer` short-circuits to `false` when the pinned
    // target isn't a trainer template, so non-trainer NPCs pinned
    // as `last_interaction_target` (vendors, lootables, dialog NPCs)
    // never trigger a resend.
    //
    // `last_interaction_target` is set by `handle_interact` and
    // not cleared on trainer close. Trade-off: if a player opens
    // a trainer, closes it, then earns an ability some other way
    // (chain `Action::GrantAbility` from a quest turn-in), we'd
    // emit a spurious `onTrainerOpen`. The client tolerates an
    // unsolicited `onTrainerOpen` when the trainer window isn't
    // visible (UEvent_UI_TrainerOpen handler just shows the panel),
    // so this is harmless.
    let trainer_entity_id = space_mgr
        .get_entity(entity_id)
        .and_then(|p| p.last_interaction_target);
    if let Some(target) = trainer_entity_id {
        let is_trainer = space_mgr
            .get_entity(target)
            .and_then(|t| t.template_id)
            .is_some_and(|tid| space_mgr.template_trainer_lists.contains_key(&tid));
        if is_trainer {
            tracing::debug!(
                target: "abilities",
                event = "trainer_resend",
                entity_id,
                ability_id,
                trainer_entity_id = target,
                training_points,
                "AbilityGranted: re-sending onTrainerOpen to refresh trainable flags"
            );
            let _ =
                crate::cell::interactions::try_open_trainer(entity_id, target, tx, space_mgr).await;
        }
    }
}

/// Send the training-point counter to the owning client. Built by the same
/// builder as the level-up bundle's property (`build_grant_xp_bundle`).
async fn send_training_points(
    entity_id: u32,
    training_points: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: crate::cell::client_methods::spawnable_entity::ON_ENTITY_PROPERTY,
            args: training_points_property_args(training_points),
        })
        .await
    {
        tracing::warn!(
            target: "abilities",
            event = "training_points_send_failed",
            entity_id,
            training_points,
            error = %e,
            "AbilityGranted: training-point property send failed; counter stale until relog"
        );
    }
}

/// Handle `BaseToCellMsg::ProgressionChanged`: mirror a persisted level-up.
/// The base already sent the client its own level-up bundle, so nothing is
/// sent from here.
pub(super) fn handle_progression_changed(
    entity_id: u32,
    level: i32,
    training_points: i32,
    space_mgr: &mut SpaceManager,
) {
    let Some(entity) = space_mgr.get_entity_mut(entity_id) else {
        tracing::warn!(
            target: "abilities",
            event = "progression_changed_entity_missing",
            entity_id,
            level,
            training_points,
            "ProgressionChanged: no cell entity; trainer gates keep the old level and points"
        );
        return;
    };
    entity.level = level.max(1) as u32;
    entity.tree_progress.training_points = training_points;
    tracing::debug!(
        target: "abilities",
        event = "progression_changed",
        entity_id,
        level,
        training_points,
        "ProgressionChanged: cell level and training points updated"
    );
}
