//! Trainer authority on the cell: resolve the player's pinned interaction
//! target into a [`TrainerPin`] for the trainability predicate, and re-send
//! the pinned trainer's `onTrainerOpen`.
//!
//! The pin is `CellEntity::last_interaction_target`, written by the
//! `interact` handler only after `interact_target_in_range` passed. A purchase
//! re-checks range here because the player may have walked away since.

use tokio::sync::mpsc;

use super::dispatch::interact_target_in_range;
use super::trainer::try_open_trainer;
use crate::ability_tree::TrainerPin;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Resolve `trainer_entity_id` (the player's pin) into a [`TrainerPin`].
///
/// `archetype_id` picks the trainer's offered list. `None` yields an empty
/// list, but the predicate rejects an archetype-less player before any
/// trainer gate runs, so that list is never read.
pub(crate) fn trainer_pin(
    space_mgr: &SpaceManager,
    player_entity_id: u32,
    trainer_entity_id: Option<u32>,
    archetype_id: Option<i32>,
) -> TrainerPin<'_> {
    let Some(trainer_entity_id) = trainer_entity_id else {
        return TrainerPin::Unpinned;
    };
    let Some(trainer) = space_mgr.get_entity(trainer_entity_id) else {
        return TrainerPin::Despawned;
    };
    let Some(list_id) = trainer
        .template_id
        .and_then(|tid| space_mgr.template_trainer_lists.get(&tid))
    else {
        return TrainerPin::NotATrainer;
    };
    let offered = archetype_id
        .and_then(|arch| space_mgr.trainer_abilities.get(&(*list_id, arch)))
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    TrainerPin::Trainer {
        offered,
        in_range: interact_target_in_range(player_entity_id, trainer_entity_id, space_mgr),
    }
}

/// Re-send `onTrainerOpen` for the player's pinned trainer, if the pin is a
/// live trainer. Returns whether a re-send was attempted.
///
/// The template check comes first so a pinned vendor or dialog NPC never
/// reaches `try_open_trainer`, whose `not_a_trainer` counter would then
/// count a trainer re-send that never was.
pub(crate) async fn resend_pinned_trainer(
    player_entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> bool {
    let Some(target) = space_mgr
        .get_entity(player_entity_id)
        .and_then(|p| p.last_interaction_target)
    else {
        return false;
    };
    let is_trainer = space_mgr
        .get_entity(target)
        .and_then(|t| t.template_id)
        .is_some_and(|tid| space_mgr.template_trainer_lists.contains_key(&tid));
    if !is_trainer {
        return false;
    }
    try_open_trainer(player_entity_id, target, tx, space_mgr).await
}
