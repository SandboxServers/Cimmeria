//! `BaseToCellMsg::AbilityGranted` handler — mirrors a base-persisted ability
//! grant onto the cell entity, refreshes the client hotbar, and (when a trainer
//! is pinned) re-fires `onTrainerOpen` so the trainable-flags UI updates.
//! Extracted from `base_messages/mod.rs` as a pure code move.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::player_init::send_known_abilities_update;

/// Handle `BaseToCellMsg::AbilityGranted`.
pub(super) async fn handle_ability_granted(
    entity_id: u32,
    ability_id: i32,
    training_points_remaining: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Base persisted + debited; mirror onto the cell entity and
    // refresh the client hotbar via the shared helper.
    if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
        entity.abilities.add_ability(ability_id);
    }
    tracing::info!(
        target: "abilities",
        event = "granted",
        entity_id,
        ability_id,
        training_points_remaining,
        "AbilityGranted: cell mirrored + hotbar refresh"
    );
    send_known_abilities_update(entity_id, tx, space_mgr).await;

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
                training_points_remaining,
                "AbilityGranted: re-sending onTrainerOpen to refresh trainable flags"
            );
            let _ =
                crate::cell::interactions::try_open_trainer(entity_id, target, tx, space_mgr).await;
        }
    }
}
